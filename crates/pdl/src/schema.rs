// First and foremost goal of schema validation is to give nice error messages to LLMs.

use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::str::FromStr;

mod parse;

pub use parse::{SchemaParseError, parse_schema};

#[derive(Clone, Debug, PartialEq)]
pub enum SchemaType {
    Integer,
    Float,
    String,
    Array(Option<Box<Schema>>),
    Boolean,
    Object(Vec<(String, Schema)>),
    Null,
}

impl SchemaType {
    pub fn type_name(&self) -> &'static str {
        match self {
            SchemaType::Integer => "integer",
            SchemaType::Float => "float",
            SchemaType::String => "string",
            SchemaType::Array(_) => "array",
            SchemaType::Boolean => "boolean",
            SchemaType::Object(_) => "object",
            SchemaType::Null => "null",
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
    // This is an error message for LLMs, not for ordinary programmers.
    // It has to be short and readable english sentences, not typical
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
                "Your output has a wrong type. It has to be `{}`, not `{}`",
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
        if let Schema { r#type: SchemaType::String, constraint } = self {
            let (len_min, len_max) = constraint.as_ref().map(
                |Constraint { min, max }| (
                    min.as_ref().map(|n| n.parse::<usize>().unwrap()),
                    max.as_ref().map(|n| n.parse::<usize>().unwrap()),
                )
            ).unwrap_or((None, None));
            let s_len = s.chars().count();

            if let Some(len_min) = len_min {
                if s_len < len_min {
                    return Err(format!("Your output is too short. Please make sure that it's at least {len_min} characters."));
                }
            }

            if let Some(len_max) = len_max {
                if s_len > len_max {
                    return Err(format!("Your output is too long. Please make sure that it's at most {len_max} characters."));
                }
            }

            return Ok(Value::String(s.to_string()));
        }

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
            (SchemaType::String, Value::String(s)) => {
                check_range(SchemaType::String, &self.constraint, s.len())?;
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
            (SchemaType::Boolean, Value::Bool(_)) => Ok(()),
            (t1, t2) => Err(SchemaError::TypeError {
                expected: t1.clone(),
                got: get_schema_type(t2),
            }),
        }
    }

    // It tries to extract a json value from a haystack.
    // It raises an error if there are multiple candidates.
    // It can be more generous than json's syntax (e.g. it allows `true` and `True`),
    // but it's return value must be a valid json.
    fn extract_text(&self, s: &str) -> Result<String, String> {
        if let SchemaType::Boolean = &self.r#type {
            let s = s.to_ascii_lowercase();
            let t = s.contains("true");
            let f = s.contains("false");

            return match (t, f) {
                (true, false) => Ok(String::from("true")),
                (false, true) => Ok(String::from("false")),
                (true, true) => Err(String::from("Your output contains both `true` and `false`. Please be specific.")),
                (false, false) => Err(String::from("I cannot find `boolean` in your output. Please make sure that your output contains a valid json value.")),
            };
        }

        if let SchemaType::Null = &self.r#type {
            let low = s.to_ascii_lowercase();

            if low == "null" || low == "none" {
                return Ok(String::from("null"));
            }

            else {
                return Err(format!("{s:?} is not null."));
            }
        }

        let re = match &self.r#type {
            SchemaType::Integer => Regex::new(r"^[^0-9]*([0-9]+)[^0-9]*$").unwrap(),
            SchemaType::Float => Regex::new(r"^[^0-9]*([0-9]+(?:\.[0-9]+)?)[^0-9]*$").unwrap(),
            SchemaType::Array(_) => Regex::new(r"(?s)[^\[\]]*(\[.*\])[^\[\]]*").unwrap(),
            SchemaType::Object(_) => Regex::new(r"(?s)[^{}]*(\{.*\})[^{}]*").unwrap(),
            SchemaType::String => unreachable!(),
            SchemaType::Boolean => unreachable!(),
            SchemaType::Null => unreachable!(),
        };

        match re.captures(s) {
            Some(cap) => Ok(cap[1].to_string()),
            None => Err(format!("I cannot find `{}` in your output. Please make sure that your output contains a valid json value.", self.type_name())),
        }
    }

    pub fn default_integer() -> Self {
        Schema {
            r#type: SchemaType::Integer,
            constraint: None,
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

    pub fn default_array(r#type: Schema) -> Self {
        Schema {
            r#type: SchemaType::Array(Some(Box::new(r#type))),
            constraint: None,
        }
    }

    pub fn default_boolean() -> Self {
        Schema {
            r#type: SchemaType::Boolean,
            constraint: None,
        }
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        assert!(self.constraint.is_none());
        self.constraint = Some(constraint);
    }

    pub fn validate_constraint(&self) -> Result<(), SchemaParseError> {
        match (&self.r#type, &self.constraint) {
            (ty @ (SchemaType::Integer | SchemaType::Array(_) | SchemaType::String), Some(constraint)) => {
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

                if matches!(ty, SchemaType::Integer) || matches!(ty, SchemaType::Array(_)) {
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
            (ty @ (SchemaType::Null | SchemaType::Boolean | SchemaType::Object(_)), Some(constraint)) => {
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
        if let Constraint { min: Some(min), max } = &constraint {
            let min = min.parse::<T>().unwrap();

            if n < min {
                return Err(SchemaError::RangeError {
                    s1: String::from(if schema.is_number() { "small" } else { "short" }),
                    s2: String::from(if schema.is_number() { "is at least" } else { "has at least" }),
                    s3: if schema.is_number() { n.to_string() } else if schema.is_array() { format!("{n} elements") } else { format!("{n} characters") },
                });
            }
        }

        if let Constraint { min, max: Some(max) } = &constraint {
            let max = max.parse::<T>().unwrap();

            if n > max {
                return Err(SchemaError::RangeError {
                    s1: String::from(if schema.is_number() { "big" } else { "long" }),
                    s2: String::from(if schema.is_number() { "is at most" } else { "has at most" }),
                    s3: if schema.is_number() { n.to_string() } else if schema.is_array() { format!("{n} elements") } else { format!("{n} characters") },
                });
            }
        }
    }

    Ok(())
}
