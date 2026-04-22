use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MarginMode {
    Cross,
    Isolated,
    Raw(String),
}

impl MarginMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cross => "cross",
            Self::Isolated => "isolated",
            Self::Raw(value) => value,
        }
    }

    pub(crate) fn as_okx_td_mode(&self) -> String {
        match self {
            Self::Cross => "cross".to_string(),
            Self::Isolated => "isolated".to_string(),
            Self::Raw(value) => value.clone(),
        }
    }

    pub(crate) fn as_binance_margin_type(&self) -> String {
        match self {
            Self::Cross => "CROSSED".to_string(),
            Self::Isolated => "ISOLATED".to_string(),
            Self::Raw(value) => value.to_ascii_uppercase(),
        }
    }

    pub(crate) fn as_bitget_margin_mode(&self) -> String {
        match self {
            Self::Cross => "crossed".to_string(),
            Self::Isolated => "isolated".to_string(),
            Self::Raw(value) => value.clone(),
        }
    }
}

impl From<&str> for MarginMode {
    fn from(value: &str) -> Self {
        match value.to_ascii_lowercase().as_str() {
            "cross" | "crossed" => Self::Cross,
            "isolated" => Self::Isolated,
            _ => Self::Raw(value.to_string()),
        }
    }
}

impl From<String> for MarginMode {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

impl Serialize for MarginMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for MarginMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}
