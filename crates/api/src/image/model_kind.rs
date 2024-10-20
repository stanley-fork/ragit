use crate::{ApiProvider, Error};
use std::str::FromStr;

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ModelKind {
    Sdxl,
    FluxPro,
    FluxDev,
    FluxSchnell,
    DallE2,
    DallE3,
}

pub const MODELS: [ModelKind; 6] = [
    ModelKind::Sdxl,
    ModelKind::FluxPro,
    ModelKind::FluxDev,
    ModelKind::FluxSchnell,
    ModelKind::DallE2,
    ModelKind::DallE3,
];

impl ModelKind {
    pub fn to_api_friendly_name(&self) -> &'static str {
        match self {
            // it uses a version hash instead of name
            ModelKind::Sdxl => "7762fd07cf82c948538e41f63f77d685e02b063e37e496e96eefd46c929f9bdc",

            // these models have names
            ModelKind::FluxPro => "flux-1.1-pro",
            ModelKind::FluxDev => "flux-dev",
            ModelKind::FluxSchnell => "flux-schnell",
            ModelKind::DallE2 => "dall-e-2",
            ModelKind::DallE3 => "dall-e-3",
        }
    }

    pub fn to_human_friendly_name(&self) -> &'static str {
        match self {
            ModelKind::Sdxl => "sdxl",
            ModelKind::FluxPro => "flux-pro",
            ModelKind::FluxDev => "flux-dev",
            ModelKind::FluxSchnell => "flux-schnell",
            ModelKind::DallE2 => "dall-e-2",
            ModelKind::DallE3 => "dall-e-3",
        }
    }

    pub fn uses_version_hash(&self) -> bool {
        match self {
            ModelKind::Sdxl => true,
            ModelKind::FluxPro => false,
            ModelKind::FluxDev => false,
            ModelKind::FluxSchnell => false,
            ModelKind::DallE2 => false,
            ModelKind::DallE3 => false,
        }
    }

    pub fn dollars_per_1m_seconds(&self) -> u64 {
        match self {
            ModelKind::Sdxl => 725,

            // these have fixed prices
            ModelKind::FluxPro => 0,
            ModelKind::FluxDev => 0,
            ModelKind::FluxSchnell => 0,
            ModelKind::DallE2 => 0,
            ModelKind::DallE3 => 0,
        }
    }

    pub fn dollars_per_1m_image(&self) -> Option<u64> {
        match self {
            ModelKind::Sdxl => None,

            // these have fixed prices
            ModelKind::FluxPro => Some(40_000),
            ModelKind::FluxDev => Some(30_000),
            ModelKind::FluxSchnell => Some(3_000),

            // TODO: their prices depend on size & quality
            ModelKind::DallE2 => Some(80_000),
            ModelKind::DallE3 => Some(20_000),
        }
    }

    // milliseconds
    pub fn api_timeout(&self) -> u64 {
        match self {
            // replicate image models generate images in 2-step
            ModelKind::Sdxl => 8_000,
            ModelKind::FluxPro => 8_000,
            ModelKind::FluxDev => 8_000,
            ModelKind::FluxSchnell => 8_000,

            // openai image models generate images in 1-step
            ModelKind::DallE2 => 60_000,
            ModelKind::DallE3 => 60_000,
        }
    }

    pub fn get_api_provider(&self) -> ApiProvider {
        match self {
            ModelKind::Sdxl => ApiProvider::Replicate,
            ModelKind::FluxPro => ApiProvider::Replicate,
            ModelKind::FluxDev => ApiProvider::Replicate,
            ModelKind::FluxSchnell => ApiProvider::Replicate,
            ModelKind::DallE2 => ApiProvider::OpenAi,
            ModelKind::DallE3 => ApiProvider::OpenAi,
        }
    }
}

impl FromStr for ModelKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<ModelKind, Error> {
        match s.to_ascii_lowercase().replace(" ", "").replace("-", "") {
            s if s.contains("sdxl") => Ok(ModelKind::Sdxl),
            s if s.contains("flux") && s.contains("pro") => Ok(ModelKind::FluxPro),
            s if s.contains("flux") && s.contains("dev") => Ok(ModelKind::FluxDev),
            s if s.contains("flux") && s.contains("schnell") => Ok(ModelKind::FluxSchnell),
            s if s.contains("dall") && s.contains("e") && s.contains("2") => Ok(ModelKind::DallE2),
            s if s.contains("dall") && s.contains("e") && s.contains("3") => Ok(ModelKind::DallE3),
            _ => Err(Error::InvalidModelKind(s.to_string())),
        }
    }
}
