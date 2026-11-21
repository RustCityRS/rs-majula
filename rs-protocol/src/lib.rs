pub mod login;
pub mod network;

#[cfg(since_244)]
pub mod js5;
/// 289's `@xxx@` text codes as 2011's tags.
#[cfg(since_530)]
pub mod markup;
#[cfg(since_530)]
pub mod world_list;

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
