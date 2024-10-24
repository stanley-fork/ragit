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
    ImageType,
    MediaMessageBuilder,
    Message,
    MessageContent,
    ModelKind as ChatModel,
    Request as ChatRequest,
    Response as ChatResponse,
    Role,
    message_contents_to_json_array,
    message_contents_to_string,
    messages_from_file,
    messages_from_pdl,
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
pub use crate::json_type::{
    JsonType,
    get_type,
};
pub use crate::record::RecordAt;
pub use crate::utils::{decode_base64, encode_base64};
