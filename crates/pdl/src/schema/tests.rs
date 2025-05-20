use super::parse::parse_schema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[test]
fn schema_validate_test() {
    let samples = vec![
        ("{ name: str, age: int }", "[]", Some("cannot find `object`")),
        ("{ name: str, age: int }", "{}", Some("missing fields: name, age")),
        ("{ name: str, age: int }", "{ \"name\": \"bae\", \"age\": 28 }", None),
        ("{ name: str, age: int }", "This is the result: { \"name\": \"bae\", \"age\": 28 }", None),
        ("{ name: str, age: int }", " { \"e\": true } { \"name\": \"bae\", \"age\": 28 }", Some("more than 1 candidates")),
        ("bool", "true", None),
        ("bool", "The answer is true", None),
        ("bool", "It's either true or false.", Some("please be specific")),
        ("yesno", "yes", None),
        ("yesno", "nope", None),
        ("yesno", "yes or no", Some("Just say yes or no")),
        ("[int {}]", "2300", Some("cannot find `array`")),
        ("[int {}]", "[2300]", None),
        ("[int { min: 400 }]", "[2300]", None),
        ("[int { min: 4000 }]", "[2300]", Some("make sure that the value is at least 4000")),
        ("[int { max: 400 }]", "[2300]", Some("make sure that the value is at most 400")),
        ("[int { max: 4000 }]", "[2300]", None),
        ("[int { max: 400 }]", "[20, 2300]", Some("make sure that the value is at most 400")),
        ("[] { max: 5 }", "[]", None),
        ("[] { max: 5 }", "[1, 2, 3, 4, 5]", None),
        ("[] { max: 5 }", "[true, false]", None),
        ("[] { max: 5 }", "[1, true]", None),
        ("[] { max: 5 }", "[1, 2, 3, 4, 5, 6]", Some("at most 5 elements")),
        ("[int] {max : 5}", "[]", None),
        ("[int] {max : 5}", "[1, 2, 3, 4, 5]", None),
        ("[int] {max : 5}", "[1, 2, 3, 4, 5, 6]", Some("at most 5 elements")),
        ("[int] {max : 5}", "[1, 2, 3, 4, 5, true]", Some("`integer`, not `boolean`")),
        ("str", "Anything is okay", None),
        ("str { max: 10 }", "This is not okay", Some("at most 10 characters")),
        ("code", "There's no code here", Some("I cannot find a code block")),
        (
            "code",
            "This is a Python code that adds 2 numbers.

```
def add(a, b):
    return a + b
```",
            None,
        ),
        ("[[int]]", "[1, 2, 3]", Some("wrong type")),
        ("[[int]]", "[[1, 2, 3], []]", None),
        ("[str]", "Stop using regex and write your own parser: [\"[]\", \"[]\"]", None),
        ("[str]", "Stop using regex and write your own parser: [\"]\", \"[\"]", None),
        ("[str]", "Stop using regex and write your own parser: [\"]\"]", None),
        ("[{ name: str, age: int }]", "[]", None),
        ("[{ name: str, age: int }]", "{ \"name\": \"bae\", \"age\": 28 }", Some("cannot find `array`")),
        ("[{ name: str, age: int }]", "[{ \"name\": \"bae\", \"age\": 28 }]", None),
        ("[int]", "I guess [1, 2, 3] would be good.\n\nAnswer: [1, 2, 3]", None),
        ("[int]", "I guess [1, 2, 3] would be good. No, come to think about it, I don't think 2 is appropriate for this.\n\nAnswer: [1, 3]", Some("more than 1 candidates")),
        ("float", "It's either 1.5 or 2", Some("more than 1 candidates")),
        ("float", "The answer is 1.5", None),
        ("float", "정답: 1.5", None),
        ("float", "정답은 1.5입니다.", None),
        ("[float { max: 0.0 }]", "[1.5, 2.5]", Some("too big")),
        ("[float { max: 0.0 }]", "[-1.5, -2.5]", None),
        ("int", "Answer: -3", None),
        ("tasklist", "- This is not a task list", Some("I cannot find a task list")),
        (
            "tasklist", "
- [ ] TODO: do whatever
- [X] Complete: do something",
            None,
        ),
        (
            "tasklist",
            "
This is just a paragraph.

This is another paragraph.

The task list begins here.

* [ ] TODO: do whatever
* [O] Complete: do something

HaHaHa
",
            None,
        ),
    ];
    let mut dump = String::new();
    let mut failures = String::new();
    let mut failure_count = 0;

    for (index, (schema_str, json, error)) in samples.iter().enumerate() {
        let schema = parse_schema(schema_str).unwrap();

        match schema.validate(json) {
            Ok(_) => {
                dump = format!(
"{dump}
----- # {index} -----
<|schema|>

{schema_str}

<|json|>

{json}

<|result|>

no-error"
);

                if let Some(error) = error {
                    failure_count += 1;
                    failures = format!(
"{failures}
----- # {index} -----
<|schema|>

{schema_str}

<|json|>

{json}

<|expected-result|>

{error}

<|result|>

no-error"
);
                }
            },
            Err(e) => {
                dump = format!(
"{dump}
----- # {index} -----
<|schema|>

{schema_str}

<|json|>

{json}

<|result|>

{e}"
);

                match error {
                    Some(error) => {
                        if !generous_match(&e, error) {
                            failure_count += 1;
                            failures = format!(
"{failures}
----- # {index} -----
<|schema|>

{schema_str}

<|json|>

{json}

<|expected-result|>

{error}

<|result|>

{e}"
);
                        }
                    },
                    None => {
                        failure_count += 1;
                        failures = format!(
"{failures}
----- # {index} -----
<|schema|>

{schema_str}

<|json|>

{json}

<|expected-result|>

no-error

<|result|>

{e}"
);
                    },
                }
            },
        }
    }

    let result = format!("{} cases, {} passed, {} failed", samples.len(), samples.len() - failure_count, failure_count);

    if !failures.is_empty() {
        panic!("{failures}\n\n{result}");
    }

    println!("{dump}\n\n{result}");
}

#[test]
fn schema_parse_test() {
    let samples = vec![
        /* (schema, is_okay) */
        ("int { min: 0.5 }", false),
        ("int { min: -3 }", false),
        ("[] { min: 0.5 }", false),
        ("[] { min: -3 }", false),
        ("str { min: 0.5 }", false),
        ("str { min: -3 }", false),
        ("int { min: 5, max: 2}", false),
        ("int { max: 5, min: 2}", true),
        ("int { min: 2, max: 5}", true),
        ("int { max: 2, min: 5}", false),
        ("float { min: 0 }", true),
    ];
    let mut failures = String::new();
    let mut failure_count = 0;

    for (index, (schema, has_to_be_okay)) in samples.iter().enumerate() {
        let is_okay = parse_schema(schema).is_ok();

        if is_okay != *has_to_be_okay {
            failure_count += 1;
            failures = format!(
"{failures}
----- # {index} -----
<|schema|>

{schema}

<|result|>

is_okay: {is_okay}
has_to_be_okay: {has_to_be_okay}",
            );
        }
    }

    if !failures.is_empty() {
        panic!("{failures}\n\n{} cases, {} passed, {} failed", samples.len(), samples.len() - failure_count, failure_count);
    }
}

// a in b
fn generous_match(a: &str, b: &str) -> bool {
    let a = a.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();
    let b = b.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_ascii_lowercase();

    a.contains(&b)
}

#[test]
fn schema_value_test() {
    // valid samples in `ragit_pdl::schema_parse_test`
    let samples: Vec<(&str, &str, Value)> = vec![
        ("{ name: str, age: int }", "{ \"name\": \"bae\", \"age\": 28 }", serde_json::to_value(Person { name: String::from("bae"), age: 28 }).unwrap()),
        ("{ name: str, age: int }", "This is the result: { \"name\": \"bae\", \"age\": 28 }", serde_json::to_value(Person { name: String::from("bae"), age: 28 }).unwrap()),
        ("bool", "true", Value::Bool(true)),
        ("bool", "The answer is true", Value::Bool(true)),
        ("yesno", "yes", Value::Bool(true)),
        ("yesno", "nope", Value::Bool(false)),
        ("[int {}]", "[2300]", vec![2300].into()),
        ("[int { min: 400 }]", "[2300]", vec![2300].into()),
        ("[int { max: 4000 }]", "[2300]", vec![2300].into()),
        ("[] { max: 5 }", "[]", Value::Array(vec![])),
        ("[] { max: 5 }", "[1, 2, 3, 4, 5]", vec![1, 2, 3, 4, 5].into()),
        ("[] { max: 5 }", "[true, false]", vec![true, false].into()),
        ("[] { max: 5 }", "[1, true]", Value::Array(vec![Value::Number(1.into()), Value::Bool(true)])),
        ("[int] {max : 5}", "[]", Value::Array(vec![])),
        ("[int] {max : 5}", "[1, 2, 3, 4, 5]", vec![1, 2, 3, 4, 5].into()),
        ("str", "Anything is okay", "Anything is okay".into()),
        (
            "code",
            "This is a Python code that adds 2 numbers.

```
def add(a, b):
    return a + b
```",
            "def add(a, b):\n    return a + b".into(),
        ),
        ("[[int]]", "[[1, 2, 3], []]", vec![vec![1, 2, 3], vec![]].into()),
        ("[str]", "Stop using regex and write your own parser: [\"[]\", \"[]\"]", vec!["[]", "[]"].into()),
        ("[str]", "Stop using regex and write your own parser: [\"]\", \"[\"]", vec!["]", "["].into()),
        ("[str]", "Stop using regex and write your own parser: [\"]\"]", vec!["]"].into()),
        ("[{ name: str, age: int }]", "[]", Value::Array(vec![])),
        ("[{ name: str, age: int }]", "[{ \"name\": \"bae\", \"age\": 28 }]", vec![serde_json::to_value(Person { name: String::from("bae"), age: 28 }).unwrap()].into()),
        ("[int]", "I guess [1, 2, 3] would be good.\n\nAnswer: [1, 2, 3]", vec![1, 2, 3].into()),
        ("float", "The answer is 1.5", Value::from(1.5)),
        ("float", "정답: 1.5", Value::from(1.5)),
        ("float", "정답은 1.5입니다.", Value::from(1.5)),
        ("[float { max: 0.0 }]", "[-1.5, -2.5]", vec![-1.5, -2.5].into()),
        ("int", "Answer: -3", Value::from(-3)),
        (
            "tasklist", "
- [ ] TODO: do whatever
- [X] Complete: do something",
            "- [ ] TODO: do whatever\n- [X] Complete: do something".into(),
        ),
        (
            "tasklist",
            "
This is just a paragraph.

This is another paragraph.

The task list begins here.

* [ ] TODO: do whatever
* [O] Complete: do something

HaHaHa
",
            "* [ ] TODO: do whatever\n* [O] Complete: do something".into(),
        ),
        (
            "tasklist",
            "
This is just a paragraph.

This is another paragraph.

The task list begins here.

* [ ] TODO: do whatever
  ... do what?
* [O] Complete: do something

HaHaHa
",
            "* [ ] TODO: do whatever\n  ... do what?\n* [O] Complete: do something".into(),
        ),
    ];
    let mut failures = String::new();
    let mut failure_count = 0;

    for (index, (schema_str, value, answer)) in samples.clone().into_iter().enumerate() {
        // If these `unwrap`s fail, go checkout `schema_validate_test` or `schema_parse_test`.
        let schema = parse_schema(schema_str).unwrap();
        let value = schema.validate(&value).unwrap();

        if value != answer {
            failure_count += 1;
            failures = format!(
"{failures}
----- # {index} -----
<|schema|>

{schema_str}

<|value|>

{value:?}

<|answer|>

{answer:?}",
            );
        }
    }

    if !failures.is_empty() {
        panic!("{failures}\n\n{} cases, {} passed, {} failed", samples.len(), samples.len() - failure_count, failure_count);
    }
}

#[derive(Deserialize, Serialize)]
struct Person {
    name: String,
    age: u32,
}
