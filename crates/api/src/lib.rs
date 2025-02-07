use ragit_fs::{WriteMode, read_string, write_string};

mod api_provider;
mod error;
mod json_type;
mod message;
mod model;
pub mod record;
mod request;
mod response;

pub use crate::api_provider::ApiProvider;
pub use crate::error::Error;
pub use crate::json_type::JsonType;
pub use crate::message::message_contents_to_json_array;
pub use crate::model::{Model, ModelRaw, get_model_by_name};
pub use crate::record::RecordAt;
pub use crate::request::Request;
pub use crate::response::Response;

pub use ragit_pdl::{
    ImageType,
    Message,
    MessageContent,
    Role,
    Schema,
};

pub fn load_models(json_path: &str) -> Result<Vec<Model>, Error> {
    let models = read_string(json_path)?;
    let models: Vec<ModelRaw> = serde_json::from_str(&models)?;
    let mut result = Vec::with_capacity(models.len());

    for model in models.iter() {
        result.push(Model::try_from(model)?);
    }

    Ok(result)
}

pub fn save_models(models: &[Model], path: &str) -> Result<(), Error> {
    let models: Vec<ModelRaw> = models.iter().map(|model| model.into()).collect();
    Ok(write_string(
        path,
        &serde_json::to_string_pretty(&models)?,
        WriteMode::CreateOrTruncate,
    )?)
}

#[cfg(test)]
mod tests {
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
            true,
            true,
        ).unwrap();
        let Pdl { messages: messages2, .. } = parse_pdl(
            pdl2,
            &tera::Context::new(),
            &current_dir().unwrap(),
            true,
            true,
        ).unwrap();

        for messages in [messages1, messages2] {
            let request = Request {
                model: (&ModelRaw::gpt_4o_mini()).try_into().unwrap(),
                messages,
                ..Request::default()
            };
            let response = request.send().await.unwrap().get_message(0).unwrap().to_ascii_lowercase();

            // TODO: it's pratically correct, but not formally correct
            assert!(response.contains("hello"));
            assert!(response.contains("world"));
        }

        remove_dir_all("__tmp_pdl_test").unwrap();
    }
}
