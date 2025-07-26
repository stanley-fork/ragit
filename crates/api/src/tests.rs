// This is a design mistake. `ragit-pdl` has to be in `ragit-api` crate, instead of being a separate crate.
// It's so strange to test pdl functionalities in `ragit-api` crate.

use crate::{ModelRaw, Request};
use ragit_fs::{
    WriteMode,
    create_dir_all,
    current_dir,
    remove_dir_all,
    write_bytes,
    write_string,
};
use ragit_pdl::{Pdl, parse_pdl, parse_pdl_from_file};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

#[tokio::test]
async fn media_pdl_test() {
    // path relative to pdl file
    let pdl1 = "
<|user|>

What do you see in this picture?

<|media(../images/sample.webp)|>
";
    // path relative to pwd
    let pdl2 = "
<|user|>

What do you see in this picture?

<|media(__tmp_pdl_test/images/sample.webp)|>
";

    create_dir_all("__tmp_pdl_test/pdl").unwrap();
    create_dir_all("__tmp_pdl_test/images").unwrap();
    let image_file = include_bytes!("../../../tests/images/hello_world.webp");
    write_string("__tmp_pdl_test/pdl/sample1.pdl", pdl1, WriteMode::AlwaysCreate).unwrap();
    write_bytes("__tmp_pdl_test/images/sample.webp", image_file, WriteMode::AlwaysCreate).unwrap();

    let Pdl { messages: messages1, .. } = parse_pdl_from_file(
        "__tmp_pdl_test/pdl/sample1.pdl",
        &tera::Context::new(),
        true,  // strict mode
    ).unwrap();
    let Pdl { messages: messages2, .. } = parse_pdl(
        pdl2,
        &tera::Context::new(),
        &current_dir().unwrap(),
        true,  // strict mode
    ).unwrap();

    for messages in [messages1, messages2] {
        for model in [
            ModelRaw::gpt_4o_mini(),
            ModelRaw::gemini_2_flash(),
        ] {
            let request = Request {
                model: (&model).try_into().unwrap(),
                messages: messages.clone(),
                ..Request::default()
            };
            let response = request.send().await.unwrap().get_message(0).unwrap().to_ascii_lowercase();

            // TODO: it's pratically correct, but not formally correct
            assert!(response.contains("hello"));
            assert!(response.contains("world"));
        }
    }

    remove_dir_all("__tmp_pdl_test").unwrap();
}

#[tokio::test]
async fn simple_schema_test() {
    let pdl = "
<|schema|>

bool

<|user|>

Is Rust a strictly typed programming language? Just say \"true\" or \"false\".
";
    assert_eq!(true, run_pdl::<_, bool>(pdl, Map::new()).await);

    let pdl = "
<|schema|>

yesno

<|user|>

Is Rust a strictly typed programming language? Just say yes or no.
";
    assert_eq!(true, run_pdl::<_, bool>(pdl, Map::new()).await);

    let pdl = "
<|schema|>

code

<|user|>

Write me a Python code that calculates an inverse of a matrix. Please wrap your code with 3 backticks, using markdown's fenced-code-block syntax.
";
    let code = run_pdl::<_, String>(pdl, Map::new()).await;

    // TODO: any better way to test this case?
    assert!(code.contains("def"));
    assert!(!code.contains("```"));

    let pdl = "
<|schema|>

[int { min: 1, max: {{documents | length}} }]

<|user|>

Below is a list of documents. Choose documents that are related to {{topic}}. You can select an arbitrary number of documents. Your output has to be in a json format, an array of integers. If no documents are relevant, just give me an empty array.

{% for document in documents %}
{{loop.index}}. {{document}}
{% endfor %}
";
    let result = run_pdl::<_, Vec<usize>>(pdl, json!({
        "documents": vec![
            "Rust programming manual: How to define a new function",
            "Introduction to CPU: How computers work",
            "Apple Pie Recipe",
            "Healthy and delicious food",
        ],
        "topic": "food",
    })).await;
    assert_eq!(result.len(), 2);
    assert!(result.contains(&3));
    assert!(result.contains(&4));

    let pdl = "
<|schema|>

[{ name: string, age: integer }]{ min: {{num_students}}, max: {{num_students}} }

<|user|>

Below is a csv file of the students of Ragit Highschool. I want you to convert it to a json array, where the schema is `[{ \"name\": string, \"age\": integer }]`. Make sure that the array includes all the {{num_students}} students.

{{csv_data}}
";
    let csv_data = "
name,age,hobby
Tom,12,soccer
Mark,13,computer
Sam,12,baseball
";
    let result = run_pdl::<_, Vec<Student>>(pdl, json!({
        "num_students": 3,
        "csv_data": csv_data,
    })).await;
    assert_eq!(result.len(), 3);
    assert!(result.contains(&Student { name: String::from("Tom"), age: 12 }));
    assert!(result.contains(&Student { name: String::from("Mark"), age: 13 }));
    assert!(result.contains(&Student { name: String::from("Sam"), age: 12 }));
}

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
struct Student {
    name: String,
    age: usize,
}

async fn run_pdl<T: Serialize, U: Default + DeserializeOwned>(pdl: &str, context: T) -> U {
    let Value::Object(context_hash_map) = serde_json::to_value(context).unwrap() else { panic!("expected an object") };
    let mut context = tera::Context::new();

    for (k, v) in context_hash_map.iter() {
        context.insert(k, v);
    }

    let Pdl { messages, schema } = parse_pdl(
        pdl,
        &context,
        ".",   // no media files
        true,  // strict mode
    ).unwrap();
    let request = Request {
        model: (&ModelRaw::gpt_4o_mini()).try_into().unwrap(),
        messages,
        schema,
        ..Request::default()
    };
    let response = request.send_and_validate::<U>(U::default()).await.unwrap();

    response
}
