pub mod login;
pub mod network;

#[cfg(since_244)]
pub mod js5;
/// 289's `@xxx@` text codes as 2011's tags.
#[cfg(since_530)]
pub mod markup;

#[derive(Debug)]
#[repr(u8)]
pub enum LoginResponse {
    LoginOk = 2,
    BadCredentials = 3,
    Banned = 4,
    Duplicate = 5,
    OutOfDateClientVersion = 6,
    ServerFull = 7,
    LoginServerOffline = 8,
    IpLimit = 9,
    BadSession = 10,
    Rejected = 11,
    NeedMembers = 12,
    BadSave = 13,
    UpdateInProgress = 14,
    ReconnectOk = 15,
    TooManyAttempts = 16,
    MoveToFree = 17,
    #[cfg(before_254)]
    SuccessModerator = 18,
    #[cfg(all(since_244, before_254))]
    SuccessJagexModerator = 19,
    #[cfg(since_244)]
    LoginMismatch = 20,
    #[cfg(since_254)]
    HopBlocked = 21,
    #[cfg(since_647)]
    MovingTooFast = 22,
    #[cfg(since_530)]
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

/// Create-account service response codes: 530's services 36, 147 and 186,
/// and 647's 22.
#[cfg(since_530)]
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
