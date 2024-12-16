use super::parse::parse_schema;

#[test]
fn schema_validate_test() {
    let samples = vec![
        ("{ name: str, age: int }", "[]", Some("cannot find `object`")),
        ("{ name: str, age: int }", "{}", Some("missing fields: name, age")),
        ("{ name: str, age: int }", "{ \"name\": \"bae\", \"age\": 28 }", None),
        ("{ name: str, age: int }", "This is the result: { \"name\": \"bae\", \"age\": 28 }", None),
        ("{ name: str, age: int }", " { \"e\": true } { \"name\": \"bae\", \"age\": 28 }", Some("cannot parse your json output")),
        ("bool", "true", None),
        ("bool", "The answer is true", None),
        ("bool", "It's either true or false.", Some("please be specific")),
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
    ];
    let mut dump = String::new();
    let mut failures = String::new();
    let mut failure_count = 0;

    for (index, (schema_str, json, error)) in samples.iter().enumerate() {
        let schema = parse_schema(schema_str.as_bytes()).unwrap();

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
        let is_okay = parse_schema(schema.as_bytes()).is_ok();

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
