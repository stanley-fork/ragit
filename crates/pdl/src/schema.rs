// First and foremost goal of schema validation is to give nice error messages to LLMs.

use crate::error::Error;
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::str::FromStr;

mod code_fence;
mod parse;
mod parse_value;
mod task_list;

pub use code_fence::try_extract_code_fence;
pub use parse::{SchemaParseError, parse_schema};
use parse_value::{JsonMatch, extract_jsonish_literal};
pub use task_list::{count_task_list_elements, try_extract_task_list};

#[cfg(test)]
mod tests;

// After adding a non-json schema_type,
//
// 1. Make sure that the code compiles.
// 2. Add a test case in `tests/`.
// 3. Update `render_pdl_schema`.
#[derive(Clone, Debug, PartialEq)]
pub enum SchemaType {
    Integer,
    Float,
    String,
    Array(Option<Box<Schema>>),
    Boolean,
    Object(Vec<(String, Schema)>),
    Null,
    Yesno,
    Code,

    // https://github.github.com/gfm/#task-list-items-extension-
    // https://github.com/baehyunsol/ragit/issues/17
    TaskList,
}

impl SchemaType {
    // LLMs will see this name (e.g. "I cannot find `array` in your output.")
    pub fn type_name(&self) -> &'static str {
        match self {
            SchemaType::Integer => "integer",
            SchemaType::Float => "float",
            SchemaType::String => "string",
            SchemaType::Array(_) => "array",
            SchemaType::Boolean => "boolean",
            SchemaType::Object(_) => "object",
            SchemaType::Null => "null",
            SchemaType::Yesno => "yes or no",
            SchemaType::Code => "code",
            SchemaType::TaskList => "markdown task list",
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            SchemaType::Integer
            | SchemaType::Float => true,
            _ => false,
        }
    }

    pub fn is_array(&self) -> bool {
        matches!(self, SchemaType::Array(_))
    }

    pub fn unwrap_keys(&self) -> Vec<String> {
        match self {
            SchemaType::Object(obj) => obj.iter().map(|(key, _)| key.to_string()).collect(),
            _ => panic!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum SchemaError {
    // _ is too (small | big | short | long). Make sure that _ (is at least | is at most | has at least | has at most) (N | N characters | N elements).
    RangeError {
        s1: String,  // small | big | short | long
        s2: String,  // is at least | is as most | has at least | has at most
        s3: String,  // N | N characters | N elements
    },
    MissingKeys(Vec<String>),
    UnnecessaryKeys(Vec<String>),
    ErrorInObject {
        key: String,
        error: Box<SchemaError>,
    },
    ErrorInArray {
        index: usize,
        error: Box<SchemaError>,
    },
    TypeError {
        expected: SchemaType,
        got: SchemaType,
    },
}

impl SchemaError {
    // This is an error message for LLMs, not for (human) programmers.
    // It has to be short and readable english sentences, unlike
    // compiler error messages.
    pub fn prettify(&self, schema: &Schema) -> String {
        match self {
            SchemaError::RangeError { s1, s2, s3 } => format!("Your output is too {s1}. Make sure that the output {s2} {s3}."),
            SchemaError::MissingKeys(keys) => {
                let schema_keys = schema.unwrap_keys();

                format!(
                    "Your output is missing {}: {}. Make sure that your output contains {} key{}: {}",
                    if keys.len() == 1 { "a field" } else { "fields "},
                    keys.join(", "),
                    schema_keys.len(),
                    if schema_keys.len() == 1 { "" } else { "s" },
                    schema_keys.join(", "),
                )
            },
            SchemaError::UnnecessaryKeys(keys) => {
                let schema_keys = schema.unwrap_keys();

                format!(
                    "Your output has {}unnecessary key{}: {}. Make sure that the output contains {}key{}: {}",
                    if keys.len() == 1 { "an " } else { "" },
                    if keys.len() == 1 { "" } else { "s" },
                    keys.join(", "),
                    if schema_keys.len() == 1 { "a " } else { "" },
                    if schema_keys.len() == 1 { "" } else { "s" },
                    schema_keys.join(", "),
                )
            },
            SchemaError::ErrorInObject { key, error } => match error.as_ref() {
                SchemaError::RangeError { s1, s2, s3 } => format!(
                    "Field `{key}` of your output is too {s1}. Make sure that the field {s2} {s3}.",
                ),
                SchemaError::TypeError { expected, got } => format!(
                    "Field `{key}` of your output has a wrong type. Make sure that the field is `{}`, not `{}`.",
                    expected.type_name(),
                    got.type_name(),
                ),
                // It assumes that the models can find the schema somewhere in prompts.
                // TODO: better error messages in these cases
                _ => String::from("Please make sure that your output has a correct schema."),
            },
            SchemaError::ErrorInArray { index, error } => match error.as_ref() {
                SchemaError::RangeError { s1, s2, s3 } => format!(
                    "The {} value of your output is too {s1}. Make sure that the value {s2} {s3}.",
                    match index {
                        0 => String::from("first"),
                        1 => String::from("second"),
                        2 => String::from("third"),
                        3 => String::from("forth"),
                        4 => String::from("fifth"),
                        n => format!("{}th", n + 1),
                    },
                ),
                SchemaError::TypeError { expected, got } => format!(
                    "The {} value of your output has a wrong type. Make sure all the elements are `{}`, not `{}`.",
                    match index {
                        0 => String::from("first"),
                        1 => String::from("second"),
                        2 => String::from("third"),
                        3 => String::from("forth"),
                        4 => String::from("fifth"),
                        n => format!("{}th", n + 1),
                    },
                    expected.type_name(),
                    got.type_name(),
                ),
                // It assumes that the models can find the schema somewhere in prompts.
                // TODO: better error messages in these cases
                _ => String::from("Please make sure that your output has a correct schema."),
            },
            SchemaError::TypeError { expected, got } => format!(
                "Your output has a wrong type. It has to be `{}`, not `{}`.",
                expected.type_name(),
                got.type_name(),
            ),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Schema {
    r#type: SchemaType,
    constraint: Option<Constraint>,
}

impl Schema {
    // If it's `Ok(s)`, `s` is an evaluable json string.
    // If it's `Err(e)`, `e` is an error message which must be sent to the LLM.
    pub fn validate(&self, s: &str) -> Result<Value, String> {
        let extracted_text = self.extract_text(s)?;
        let v = match serde_json::from_str::<Value>(&extracted_text) {
            Ok(v) => v,
            Err(_) => {
                return Err(String::from("I cannot parse your json output. Please make sure that your output contains valid json with valid data."));
            },
        };

        self.validate_value(&v).map_err(|e| e.prettify(self))?;
        Ok(v)
    }

    fn validate_value(&self, v: &Value) -> Result<(), SchemaError> {
        match (&self.r#type, v) {
            (SchemaType::Integer, Value::Number(n)) => match n.as_i64() {
                Some(n) => {
                    check_range(SchemaType::Integer, &self.constraint, n)?;
                    Ok(())
                },
                None => Err(SchemaError::TypeError {
                    expected: SchemaType::Integer,
                    got: SchemaType::Float,
                }),
            },
            (SchemaType::Float, Value::Number(n)) => match n.as_f64() {
                Some(n) => {
                    check_range(SchemaType::Float, &self.constraint, n)?;
                    Ok(())
                },
                None => unreachable!(),
            },
            (ty @ (SchemaType::String | SchemaType::Code), Value::String(s)) => {
                check_range(ty.clone(), &self.constraint, s.len())?;
                Ok(())
            },
            (SchemaType::Array(schema), Value::Array(v)) => {
                if let Some(schema) = schema {
                    for (index, e) in v.iter().enumerate() {
                        if let Err(e) = schema.validate_value(e) {
                            return Err(SchemaError::ErrorInArray { index, error: Box::new(e) });
                        }
                    }
                }

                check_range(SchemaType::Array(None), &self.constraint, v.len())?;
                Ok(())
            },
            (SchemaType::Object(obj_schema), Value::Object(obj)) => {
                let mut keys_in_schema = HashSet::with_capacity(obj_schema.len());
                let mut missing_keys = vec![];
                let mut unnecessary_keys = vec![];

                for (k, v_schema) in obj_schema.iter() {
                    keys_in_schema.insert(k);

                    match obj.get(k) {
                        Some(v) => match v_schema.validate_value(v) {
                            Ok(_) => {},
                            Err(e) => {
                                return Err(SchemaError::ErrorInObject {
                                    key: k.to_string(),
                                    error: Box::new(e),
                                });
                            },
                        },
                        None => {
                            missing_keys.push(k.to_string());
                        },
                    }
                }

                for k in obj.keys() {
                    if !keys_in_schema.contains(k) {
                        unnecessary_keys.push(k.to_string());
                    }
                }

                if !missing_keys.is_empty() {
                    Err(SchemaError::MissingKeys(missing_keys))
                }

                else if !unnecessary_keys.is_empty() {
                    Err(SchemaError::UnnecessaryKeys(unnecessary_keys))
                }

                else {
                    Ok(())
                }
            },
            (SchemaType::TaskList, Value::String(s)) => {
                check_range(SchemaType::TaskList, &self.constraint, count_task_list_elements(s))?;
                Ok(())
            },
            (SchemaType::Boolean | SchemaType::Yesno, Value::Bool(_)) => Ok(()),
            (t1, t2) => Err(SchemaError::TypeError {
                expected: t1.clone(),
                got: get_schema_type(t2),
            }),
        }
    }

    // It tries to extract a json value from a haystack.
    // It raises an error if there are multiple candidates.
    // It can be more generous than json's syntax (e.g. it allows `true` and `True`),
    // but its return value must be a valid json.
    fn extract_text(&self, s: &str) -> Result<String, String> {
        match &self.r#type {
            SchemaType::Boolean | SchemaType::Yesno => {
                let s = s.to_ascii_lowercase();
                let t = if self.r#type == SchemaType::Boolean { s.contains("true")} else { s.contains("yes") };
                let f = if self.r#type == SchemaType::Boolean { s.contains("false")} else { s.contains("no") };

                match (t, f) {
                    (true, false) => Ok(String::from("true")),
                    (false, true) => Ok(String::from("false")),
                    (true, true) => if self.r#type == SchemaType::Boolean {
                        Err(String::from("Your output contains both `true` and `false`. Please be specific."))
                    } else {
                        Err(String::from("Just say yes or no."))
                    },
                    (false, false) => if self.r#type == SchemaType::Boolean {
                        Err(String::from("I cannot find `boolean` in your output. Please make sure that your output contains a valid json value."))
                    } else {
                        Err(String::from("Just say yes or no."))
                    },
                }
            },
            SchemaType::Null => {
                let low = s.to_ascii_lowercase();

                if low == "null" || low == "none" {
                    Ok(String::from("null"))
                }

                else {
                    Err(format!("{s:?} is not null."))
                }
            },
            SchemaType::String => Ok(format!("{s:?}")),
            SchemaType::Code => Ok(format!("{:?}", try_extract_code_fence(s)?)),
            SchemaType::TaskList => Ok(format!("{:?}", try_extract_task_list(s)?)),
            SchemaType::Integer | SchemaType::Float
            | SchemaType::Array(_) | SchemaType::Object(_) => {
                let mut jsonish_literals = extract_jsonish_literal(s);

                match jsonish_literals.get_matches(&self.r#type) {
                    JsonMatch::NoMatch => Err(format!("I cannot find `{}` in your output. Please make sure that your output contains a valid json value.", self.type_name())),
                    JsonMatch::MultipleMatches => Err(format!("I see more than 1 candidates that look like `{}`. I don't know which one to choose. Please give me just one `{}`.", self.type_name(), self.type_name())),
                    JsonMatch::Match(s) => Ok(s.to_string()),
                }
            },
        }
    }

    pub fn default_integer() -> Self {
        Schema {
            r#type: SchemaType::Integer,
            constraint: None,
        }
    }

    /// Both inclusive.
    pub fn integer_between(min: Option<i128>, max: Option<i128>) -> Self {
        Schema {
            r#type: SchemaType::Integer,
            constraint: Some(Constraint {
                min: min.map(|n| n.to_string()),
                max: max.map(|n| n.to_string()),
            }),
        }
    }

    pub fn default_float() -> Self {
        Schema {
            r#type: SchemaType::Float,
            constraint: None,
        }
    }

    pub fn default_string() -> Self {
        Schema {
            r#type: SchemaType::String,
            constraint: None,
        }
    }

    pub fn default_array(r#type: Option<Schema>) -> Self {
        Schema {
            r#type: SchemaType::Array(r#type.map(|t| Box::new(t))),
            constraint: None,
        }
    }

    pub fn default_boolean() -> Self {
        Schema {
            r#type: SchemaType::Boolean,
            constraint: None,
        }
    }

    pub fn default_yesno() -> Self {
        Schema {
            r#type: SchemaType::Yesno,
            constraint: None,
        }
    }

    pub fn default_code() -> Self {
        Schema {
            r#type: SchemaType::Code,
            constraint: None,
        }
    }

    pub fn default_task_list() -> Self {
        Schema {
            r#type: SchemaType::TaskList,
            constraint: None,
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        debug_assert!(self.constraint.is_none());
        self.constraint = Some(constraint);
    }

    pub fn validate_constraint(&self) -> Result<(), SchemaParseError> {
        match (&self.r#type, &self.constraint) {
            (ty @ (SchemaType::Integer | SchemaType::Array(_) | SchemaType::String | SchemaType::TaskList | SchemaType::Code), Some(constraint)) => {
                let mut min_ = i64::MIN;
                let mut max_ = i64::MAX;

                if let Some(min) = &constraint.min {
                    match min.parse::<i64>() {
                        Ok(n) => { min_ = n; },
                        Err(_) => {
                            return Err(SchemaParseError::InvalidConstraint(format!("{min:?} is not a valid integer.")));
                        },
                    }
                }

                if let Some(max) = &constraint.max {
                    match max.parse::<i64>() {
                        Ok(n) => { max_ = n; },
                        Err(_) => {
                            return Err(SchemaParseError::InvalidConstraint(format!("{max:?} is not a valid integer.")));
                        },
                    }
                }

                if min_ > max_ {
                    return Err(SchemaParseError::InvalidConstraint(format!("`min` ({min_}) is greater than `max` ({max_}).")));
                }

                if matches!(ty, SchemaType::String) || matches!(ty, SchemaType::Array(_)) {
                    if constraint.min.is_some() && min_ < 0 {
                        return Err(SchemaParseError::InvalidConstraint(format!("`min` is supposed to be a positive integer, but is {min_}")));
                    }

                    if constraint.max.is_some() && max_ < 0 {
                        return Err(SchemaParseError::InvalidConstraint(format!("`max` is supposed to be a positive integer, but is {max_}")));
                    }
                }

                Ok(())
            },
            (SchemaType::Float, Some(constraint)) => {
                let mut min_ = f64::MIN;
                let mut max_ = f64::MAX;

                if let Some(min) = &constraint.min {
                    match min.parse::<f64>() {
                        Ok(n) => { min_ = n; },
                        Err(_) => {
                            return Err(SchemaParseError::InvalidConstraint(format!("{min:?} is not a valid number.")));
                        },
                    }
                }

                if let Some(max) = &constraint.max {
                    match max.parse::<f64>() {
                        Ok(n) => { max_ = n; },
                        Err(_) => {
                            return Err(SchemaParseError::InvalidConstraint(format!("{max:?} is not a valid number.")));
                        },
                    }
                }

                if min_ > max_ {
                    return Err(SchemaParseError::InvalidConstraint(format!("`min` ({min_}) is greater than `max` ({max_}).")));
                }

                Ok(())
            },
            (ty @ (SchemaType::Null | SchemaType::Boolean | SchemaType::Object(_) | SchemaType::Yesno), Some(constraint)) => {
                if constraint.min.is_some() {
                    Err(SchemaParseError::InvalidConstraint(format!(
                        "Type `{}` cannot have constraint `min`",
                        ty.type_name(),
                    )))
                }

                else if constraint.max.is_some() {
                    Err(SchemaParseError::InvalidConstraint(format!(
                        "Type `{}` cannot have constraint `max`",
                        ty.type_name(),
                    )))
                }

                else {
                    Ok(())
                }
            },
            (_, None) => Ok(()),
        }
    }

    pub fn type_name(&self) -> &'static str {
        self.r#type.type_name()
    }

    pub fn unwrap_keys(&self) -> Vec<String> {
        self.r#type.unwrap_keys()
    }
}

/// pdl schema is a bit unintuitive when you're using non-json schema.
/// For example, schema `yesno` will become `Value::Bool`. If you naively
/// convert this to a string, you'll get "true" or "false", not "yes" or "no".
///
/// Likewise, schema `code` will become `Value::String` whose content is the code.
/// If you naively convert this to a string (using serde_json::to_string), you'll
/// get something like "\"fn main() ...\"".
pub fn render_pdl_schema(
    schema: &Schema,

    // Result of `Schema::validate`
    value: &Value,
) -> Result<String, Error> {
    let s = match (&schema.r#type, value) {
        (SchemaType::Code, Value::String(s)) => s.to_string(),
        (SchemaType::TaskList, Value::String(s)) => s.to_string(),
        (SchemaType::Yesno, Value::Bool(b)) => if *b {
            String::from("yes")
        } else {
            String::from("no")
        },
        _ => serde_json::to_string_pretty(value)?,
    };

    Ok(s)
}

// union of all constraints
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Constraint {
    // for `Integer` and `Float`, these are min/max values
    // for `String`, these are min/max char len
    // for `Array`, these are min/max len
    min: Option<String>,
    max: Option<String>,
}

fn get_schema_type(v: &Value) -> SchemaType {
    match v {
        Value::Number(n) => {
            if n.is_i64() {
                SchemaType::Integer
            }

            else {
                SchemaType::Float
            }
        },
        Value::String(_) => SchemaType::String,
        Value::Array(_) => SchemaType::Array(None),
        Value::Object(_) => SchemaType::Object(vec![]),
        Value::Bool(_) => SchemaType::Boolean,
        Value::Null => SchemaType::Null,
    }
}

fn check_range<T: PartialOrd + FromStr + ToString + Display>(schema: SchemaType, constraint: &Option<Constraint>, n: T) -> Result<(), SchemaError> where <T as FromStr>::Err: Debug {
    // It's okay to unwrap values because `Constraint` is always validated at creation.
    if let Some(constraint) = constraint {
        if let Constraint { min: Some(min), .. } = &constraint {
            let min = min.parse::<T>().unwrap();

            if n < min {
                return Err(SchemaError::RangeError {
                    s1: String::from(if schema.is_number() { "small" } else { "short" }),
                    s2: String::from(if schema.is_number() { "is at least" } else { "has at least" }),
                    s3: if schema.is_number() { min.to_string() } else if schema.is_array() { format!("{min} elements") } else { format!("{min} characters") },
                });
            }
        }

        if let Constraint { max: Some(max), .. } = &constraint {
            let max = max.parse::<T>().unwrap();

            if n > max {
                return Err(SchemaError::RangeError {
                    s1: String::from(if schema.is_number() { "big" } else { "long" }),
                    s2: String::from(if schema.is_number() { "is at most" } else { "has at most" }),
                    s3: if schema.is_number() { max.to_string() } else if schema.is_array() { format!("{max} elements") } else { format!("{max} characters") },
                });
            }
        }
    }

    Ok(())
}
