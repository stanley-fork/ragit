use crate::api_provider::ApiProvider;
use json::JsonValue;
use ragit_pdl::{
    Message,
    MessageContent,
    encode_base64,
};

pub fn message_content_to_json(message: &MessageContent, api_provider: ApiProvider) -> JsonValue {
    match message {
        MessageContent::String(s) => {
            let mut content = JsonValue::new_object();
            content.insert("type", "text").unwrap();
            content.insert("text", s.to_string()).unwrap();

            content
        },
        MessageContent::Image { image_type, bytes } => match api_provider {
            ApiProvider::Anthropic => {
                let mut content = JsonValue::new_object();
                content.insert("type", "image").unwrap();

                let mut source = JsonValue::new_object();
                source.insert("type", "base64").unwrap();
                source.insert("media_type", image_type.get_media_type()).unwrap();
                source.insert("data", encode_base64(bytes)).unwrap();

                content.insert("source", source).unwrap();
                content
            },
            _ => {  // assume the others are all openai-compatible
                let mut content = JsonValue::new_object();
                content.insert("type", "image_url").unwrap();

                let mut image_url = JsonValue::new_object();
                image_url.insert("url", format!("data:{};base64,{}", image_type.get_media_type(), encode_base64(bytes))).unwrap();
                content.insert("image_url", image_url).unwrap();
                content
            },
        },
    }
}

pub fn message_contents_to_json_array(contents: &[MessageContent], api_provider: ApiProvider) -> JsonValue {
    JsonValue::Array(contents.iter().map(
        |content| message_content_to_json(content, api_provider)
    ).collect())
}

pub fn message_to_json(message: &Message, api_provider: ApiProvider) -> JsonValue {
    let mut result = JsonValue::new_object();
    result.insert("role", message.role.to_api_string()).unwrap();

    match (api_provider, message.content.len()) {
        (_, 0) => panic!("a message without any content"),
        (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Ollama, 1) if matches!(&message.content[0], MessageContent::String(_)) => match &message.content[0] {
            MessageContent::String(s) => {
                result.insert("content", s.clone()).unwrap();
            },
            MessageContent::Image { .. } => unreachable!(),
        },
        (ApiProvider::Groq | ApiProvider::Anthropic | ApiProvider::Cohere | ApiProvider::OpenAi | ApiProvider::Ollama | ApiProvider::DeepSeek, _) => {
            result.insert("content", message_contents_to_json_array(&message.content, api_provider)).unwrap();
        },
        (ApiProvider::Replicate, _) => panic!("no chat models for replicate"),
        (ApiProvider::Dummy, _) => unreachable!(),
    }

    result
}
