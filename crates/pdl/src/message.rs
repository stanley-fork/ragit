use std::fmt;
use crate::image::ImageType;
use crate::role::Role;
use crate::util::encode_base64;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: Vec<MessageContent>,
}

impl Message {
    pub fn simple_message(role: Role, message: String) -> Self {
        Message {
            role,
            content: vec![MessageContent::String(message)],
        }
    }

    pub fn is_valid_system_prompt(&self) -> bool {
        self.role == Role::System
        && self.content.len() == 1
        && matches!(&self.content[0], MessageContent::String(_))
    }

    pub fn is_user_prompt(&self) -> bool {
        self.role == Role::User
    }

    pub fn is_assistant_prompt(&self) -> bool {
        self.role == Role::Assistant
    }

    pub fn has_image(&self) -> bool {
        self.content.iter().any(|content| matches!(content, MessageContent::Image { .. }))
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum MessageContent {
    String(String),
    Image {
        image_type: ImageType,
        bytes: Vec<u8>,
    },
}

impl MessageContent {
    pub fn unwrap_str(&self) -> &str {
        match self {
            MessageContent::String(s) => s.as_str(),
            _ => panic!("{self:?} is not a string"),
        }
    }

    pub fn simple_message(s: String) -> Vec<Self> {
        vec![MessageContent::String(s)]
    }

    pub fn is_string(&self) -> bool {
        matches!(self, MessageContent::String(_))
    }
}

impl fmt::Display for MessageContent {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            MessageContent::String(s) => write!(fmt, "{s}"),
            MessageContent::Image { image_type, bytes } => write!(
                fmt,
                "<|raw_media({}:{})|>",
                image_type.to_extension(),
                encode_base64(bytes),
            ),
        }
    }
}
