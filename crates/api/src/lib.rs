mod api_key;
mod api_provider;
mod chat;
mod error;
mod image;
mod json_type;
pub mod record;
mod utils;

pub use crate::api_key::load_api_key;
pub use crate::api_provider::ApiProvider;
pub use crate::chat::{
    ModelKind as ChatModel,
    Request as ChatRequest,
    Response as ChatResponse,
};
pub use crate::error::Error;
pub use crate::image::{
    MODELS as IMAGE_MODELS,
    ModelKind as ImageModel,
    CreateRequest as ImageCreateRequest,
    CreateResponse as ImageCreateResponse,
    GetRequest as ImageGetRequest,
    GetResponse as ImageGetResponse,
    HandleResult as HandleImageResult,
};
pub use crate::json_type::JsonType;
pub use crate::record::RecordAt;

#[cfg(test)]
mod tests {
    use crate::{ChatModel, ChatRequest};
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
            let request = ChatRequest {
                model: ChatModel::Gpt4OMini,
                messages,
                ..ChatRequest::default()
            };
            let response = request.send().await.unwrap().get_message(0).unwrap().to_ascii_lowercase();

            // TODO: it's pratically correct, but not formally correct
            assert!(response.contains("hello"));
            assert!(response.contains("world"));
        }

        remove_dir_all("__tmp_pdl_test").unwrap();
    }
}
