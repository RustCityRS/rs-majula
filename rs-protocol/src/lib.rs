use thiserror::Error;

pub mod network;

/// Protocol error types
#[derive(Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),

    #[error("Invalid packet length: {0}")]
    InvalidLength(usize),

    #[error("Login error: {0}")]
    Login(String),

    #[error("Game packet error: {0}")]
    GamePacket(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Service opcodes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServiceOpcode {
    /// Game login service
    GameLogin = 14,
}

impl TryFrom<u8> for ServiceOpcode {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            14 => Ok(ServiceOpcode::GameLogin),
            _ => Err(ProtocolError::InvalidOpcode(value)),
        }
    }
}

/// Login response codes
#[repr(u8)]
pub enum LoginResponse {
    Success = 2,
    InvalidCredentials = 3,
    AccountDisabled = 4,
    AlreadyLoggedIn = 5,
    RuneScapeUpdated = 6,
    WorldFull = 7,
    LoginServerOffline = 8,
    TooManyConnections = 9,
    BadSession = 10,
    Rejected = 11,
    MembersOnly = 12,
    CouldNotComplete = 13,
    ServerUpdating = 14,
    Reconnect = 15,
    TooManyAttempts = 16,
    MembersArea = 17,
    SuccessModerator = 18,
}
