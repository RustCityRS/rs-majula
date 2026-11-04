pub mod network;

/// JS5 (cache serving) protocol — rev 647.
#[cfg(since_647)]
pub mod js5;
/// Login/lobby protocol — rev 647.
#[cfg(since_647)]
pub mod login;
/// 289's `@xxx@` text codes as 2011's tags.
#[cfg(since_647)]
pub mod markup;

#[cfg(since_647)]
pub use login::{
    CHECK_NAME_MAX_PAYLOAD, CREATE_ACCOUNT_MAX_PAYLOAD, CheckNameRequest, ClientSettings,
    CreateAccountRequest, LOGIN_MAX_PAYLOAD, LoginDecoder, LoginRequest, LoginType,
    MAX_USERNAME_LEN, is_valid_email,
};

/// Protocol error types
#[cfg(since_647)]
#[derive(thiserror::Error, Debug)]
pub enum ProtocolError {
    #[error("Invalid opcode: {0}")]
    InvalidOpcode(u8),

    #[error("Invalid packet length: {0}")]
    InvalidLength(usize),

    #[error("Login error: {0}")]
    Login(String),

    #[error("JS5 error: {0}")]
    Js5(String),

    #[error("Game packet error: {0}")]
    GamePacket(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(since_647)]
pub type Result<T> = std::result::Result<T, ProtocolError>;

/// Service opcodes
#[cfg(since_647)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ServiceOpcode {
    /// Game login service
    GameLogin = 14,
    /// JS5 (cache) service
    Js5 = 15,
    /// Create account service
    CreateAccount = 22,
    /// Check name availability
    CheckName = 28,
}

#[cfg(since_647)]
impl TryFrom<u8> for ServiceOpcode {
    type Error = ProtocolError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            14 => Ok(ServiceOpcode::GameLogin),
            15 => Ok(ServiceOpcode::Js5),
            22 => Ok(ServiceOpcode::CreateAccount),
            28 => Ok(ServiceOpcode::CheckName),
            _ => Err(ProtocolError::InvalidOpcode(value)),
        }
    }
}

/// Login response codes
#[repr(u8)]
pub enum LoginResponse {
    SuccessNormal = 2,
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
    #[cfg(before_254)]
    SuccessModerator = 18,
    #[cfg(all(since_244, before_254))]
    SuccessJagexModerator = 19,
    #[cfg(since_244)]
    InvalidLoginServer = 20,
    #[cfg(since_254)]
    ProfileTransfer = 21,
    #[cfg(since_647)]
    MovingTooFast = 22,
    #[cfg(since_647)]
    NotLoggedIn = 23,
    #[cfg(since_647)]
    IpBanned = 26,
    #[cfg(since_647)]
    Idk30 = 30,
    #[cfg(since_647)]
    Idk35 = 35,
    /// Client-out-of-date response (CRC mismatch is checked in create account too).
    #[cfg(since_647)]
    Idk37 = 37,
    /// "You cannot ... at this time" response.
    #[cfg(since_647)]
    Idk38 = 38,
}

/// Check-email service (service byte 28) response codes — rev 647.
#[cfg(since_647)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CheckEmailResponse {
    UnexpectedServerResponse = 0,
    Success = 2,
    ErrorContactingServer = 3,
    InvalidEmailAddress = 21,
    EmailAddressIsAlreadyInUse = 20,
}

/// Create-account service (service byte 22) response codes — rev 647.
#[cfg(since_647)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CreateAccountResponse {
    UnexpectedServerResponse = 0,
    Success = 2,
    ServerOffline = 3,
    ServerFull = 7,
    IpLimit = 9,
    DateOfBirthInvalid = 10,
    DateOfBirthFuture = 11,
    DateOfBirthThisYear = 12,
    DateOfBirthLastYear = 13,
    CountryInvalid = 14,
    NameUnavailable = 20,
    NameSuggestions = 21,
    NameInvalid = 22,
    PasswordInvalidLength = 30,
    PasswordInvalidChars = 31,
    PasswordGuessable = 32,
    PasswordGuessable1 = 33,
    PasswordSimilarToName = 34,
    PasswordSimilarToName1 = 35,
    PasswordSimilarToName2 = 36,
    ClientOutOfDate = 37,
    CannotCreateAtThisTime = 38,
    EmailInvalid = 41,
    EmailInvalid1 = 42,
    EmailInvalid2 = 43,
}
