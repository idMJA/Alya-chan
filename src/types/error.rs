use std::fmt;

#[derive(Debug)]
#[allow(dead_code)]
pub enum BotError {
    Twilight(String),
    #[allow(dead_code)]
    CommandNotFound(String),
    #[allow(dead_code)]
    MissingPermissions(String),
    #[allow(dead_code)]
    InvalidArguments(String),
    #[allow(dead_code)]
    Other(String),
}

impl fmt::Display for BotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Twilight(msg) => write!(f, "Twilight error: {}", msg),
            Self::CommandNotFound(cmd) => write!(f, "Command not found: {}", cmd),
            Self::MissingPermissions(msg) => write!(f, "Missing permissions: {}", msg),
            Self::InvalidArguments(msg) => write!(f, "Invalid arguments: {}", msg),
            Self::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for BotError {}

impl From<twilight_http::Error> for BotError {
    fn from(err: twilight_http::Error) -> Self {
        Self::Twilight(err.to_string())
    }
}

impl From<twilight_gateway::error::ReceiveMessageError> for BotError {
    fn from(err: twilight_gateway::error::ReceiveMessageError) -> Self {
        Self::Twilight(err.to_string())
    }
}

pub type BotResult<T> = Result<T, BotError>;
