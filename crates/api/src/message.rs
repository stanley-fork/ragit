use crate::api_provider::ApiProvider;
use ragit_pdl::{
    Message,
    MessageContent,
    encode_base64,
};
use serde_json::{Map, Value};

pub fn message_content_to_json(message: &MessageContent, api_provider: &ApiProvider) -> Value {
    match message {
        MessageContent::String(s) => {
            let mut content = Map::new();

            if api_provider != &ApiProvider::Google {
                content.insert(String::from("type"), "text".into());
            }

            content.insert(String::from("text"), s.to_string().into());
            content.into()
        },
        MessageContent::Image { image_type, bytes } => match api_provider {
            ApiProvider::Anthropic => {
                let mut content = Map::new();
                content.insert(String::from("type"), "image".into());

                let mut source = Map::new();
                source.insert(String::from("type"), "base64".into());
                source.insert(String::from("media_type"), image_type.get_media_type().into());
                source.insert(String::from("data"), encode_base64(bytes).into());

                content.insert(String::from("source"), source.into());
                content.into()
            },
            ApiProvider::Google => {
                let mut content = Map::new();
                let mut inline_data = Map::new();

                inline_data.insert(String::from("mime_type"), image_type.get_media_type().into());
                inline_data.insert(String::from("data"), encode_base64(bytes).into());

                content.insert(String::from("inline_data"), inline_data.into());
                content.into()
            },
            // TODO: does cohere support images?
            _ => {  // assume the others are all openai-compatible
                let mut content = Map::new();
                content.insert(String::from("type"), "image_url".into());

                let mut image_url = Map::new();
                image_url.insert(String::from("url"), format!("data:{};base64,{}", image_type.get_media_type(), encode_base64(bytes)).into());
                content.insert(String::from("image_url"), image_url.into());
                content.into()
            },
        },
    }
}

pub fn message_contents_to_json_array(contents: &[MessageContent], api_provider: &ApiProvider) -> Value {
    match api_provider {
        ApiProvider::Google => Value::Array(contents.iter().map(
            |content| message_content_to_json(content, api_provider)
        ).collect()),
        _ => {
            if contents.len() == 1 && contents[0].is_string() {
                Value::String(contents[0].unwrap_str().into())
            }

            else {
                Value::Array(contents.iter().map(
                    |content| message_content_to_json(content, api_provider)
                ).collect())
            }
        },
    }
}

pub fn message_to_json(message: &Message, api_provider: &ApiProvider) -> Value {
    let mut result = Map::new();
    result.insert(String::from("role"), message.role.to_api_string(matches!(api_provider, ApiProvider::Google)).into());

    match (api_provider, message.content.len()) {
        (_, 0) => panic!("a message without any content"),
        (ApiProvider::Google, _) => {
            result.insert(String::from("parts"), message_contents_to_json_array(&message.content, api_provider));
        },
        (ApiProvider::Anthropic, 1) if matches!(&message.content[0], MessageContent::String(_)) => match &message.content[0] {
            MessageContent::String(s) => {
                result.insert(String::from("content"), s.to_string().into());
            },
            MessageContent::Image { .. } => unreachable!(),
        },
        (ApiProvider::Anthropic | ApiProvider::Cohere | ApiProvider::OpenAi { .. }, _) => {
            result.insert(String::from("content"), message_contents_to_json_array(&message.content, api_provider));
        },
        (ApiProvider::Test(_), _) => unreachable!(),
    }

    result.into()
}
