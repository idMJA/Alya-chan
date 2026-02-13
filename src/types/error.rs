use std::fmt;

#[derive(Debug)]
pub enum BotError {
    Twilight(twilight_http::Error),
    Gateway(twilight_gateway::error::ReceiveMessageError),
    Other(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Twilight(e) => write!(f, "Twilight error: {e}"),
            Self::Gateway(e) => write!(f, "Gateway error: {e}"),
            Self::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for BotError {}

impl From<twilight_http::Error> for BotError {
    fn from(e: twilight_http::Error) -> Self {
        Self::Twilight(e)
    }
}

impl From<twilight_gateway::error::ReceiveMessageError> for BotError {
    fn from(e: twilight_gateway::error::ReceiveMessageError) -> Self {
        Self::Gateway(e)
    }
}

// Tambahkan From implementations yang kurang
impl From<twilight_http::response::DeserializeBodyError> for BotError {
    fn from(e: twilight_http::response::DeserializeBodyError) -> Self {
        Self::Other(format!("Deserialization error: {e}"))
    }
}

impl From<&str> for BotError {
    fn from(s: &str) -> Self {
        Self::Other(s.to_string())
    }
}

impl From<String> for BotError {
    fn from(s: String) -> Self {
        Self::Other(s)
    }
}

impl From<Box<dyn std::error::Error>> for BotError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Self::Other(e.to_string())
    }
}

pub type BotResult<T> = Result<T, BotError>;
