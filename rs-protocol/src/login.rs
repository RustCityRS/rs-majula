//! Login protocol
//!
//! Handles the login handshake and player authentication for revision 647.

use rs_crypto::rsa::RsaKey;
use rs_io::Packet;
use rs_io::packet::RsaFrame;

use crate::{LoginResponse as LoginResponseCode, ProtocolError, Result};

// ── Protocol payload limits ─────────────────────────────────────────────────
// Derived from the rev-647 client wire format (Class164.create_account,
// AClass7.check_name, Static276.method4437). Each constant documents what
// the client actually writes so the server can reject oversized frames.

/// Max email/username length in bytes (CP-1252 encoded, excluding null terminator).
pub const MAX_USERNAME_LEN: usize = 320;

/// Max password length in bytes.
const MAX_PASSWORD_LEN: usize = 128;

/// Max additional_info string length in create-account.
const MAX_ADDITIONAL_INFO_LEN: usize = 256;

/// RSA ciphertext max size: 2-byte length prefix + 257 bytes (2048-bit key
/// modulus = 256 bytes, +1 for Java BigInteger sign byte).
const RSA_BLOCK_MAX: usize = 2 + 257;

/// Create-account XTEA fixed fields (Class164.create_account):
/// affiliate(2) + user_flow(8) + language(1) + serial_id(1) +
/// device_info(24) + has_additional(1) + age(1) + newsletter(1) + padding(7)
const CREATE_ACCOUNT_XTEA_FIXED: usize = 2 + 8 + 1 + 1 + 24 + 1 + 1 + 1 + 7;

/// Max payload for create-account (service byte 22).
/// revision(2) + RSA block + XTEA(fixed + username + password + additional_info)
pub const CREATE_ACCOUNT_MAX_PAYLOAD: usize = 2
    + RSA_BLOCK_MAX
    + CREATE_ACCOUNT_XTEA_FIXED
    + (MAX_USERNAME_LEN + 1)
    + (MAX_PASSWORD_LEN + 1)
    + (MAX_ADDITIONAL_INFO_LEN + 1);

/// Check-name XTEA fixed fields (AClass7.check_name): language(1) + padding(7)
const CHECK_NAME_XTEA_FIXED: usize = 1 + 7;

/// Max payload for check-name (service byte 28).
/// revision(2) + RSA block + XTEA(fixed + name)
pub const CHECK_NAME_MAX_PAYLOAD: usize =
    2 + RSA_BLOCK_MAX + CHECK_NAME_XTEA_FIXED + (MAX_USERNAME_LEN + 1);

/// Login XTEA fixed fields (Static276.method4437):
/// settings(6) + 35×CRC(140) + tail padding(up to 7)
const LOGIN_XTEA_FIXED: usize = 6 + 140 + 8;

/// Max payload for login (service byte 14).
/// revision(4) + RSA block + XTEA(username + fixed)
pub const LOGIN_MAX_PAYLOAD: usize = 4 + RSA_BLOCK_MAX + (MAX_USERNAME_LEN + 1) + LOGIN_XTEA_FIXED;

/// Login request from client
#[derive(Debug, Clone)]
pub struct LoginRequest {
    /// Login type (new or reconnect)
    pub login_type: LoginType,
    /// Protocol revision
    pub revision: u32,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// ISAAC seeds (for encrypting game packets; signed, as sent by the client)
    pub isaac_seeds: [i32; 4],
    /// Client settings
    pub settings: ClientSettings,
    /// Archive CRCs sent by the client (35 values matching Class130.method834 order)
    pub archive_crcs: [i32; 35],
}

/// Login type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginType {
    /// New game world login
    New = 16,
    /// Reconnect (after disconnect)
    Reconnect = 18,
    /// Lobby login
    Lobby = 19,
}

impl TryFrom<u8> for LoginType {
    type Error = ProtocolError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            16 => Ok(LoginType::New),
            18 => Ok(LoginType::Reconnect),
            19 => Ok(LoginType::Lobby),
            _ => Err(ProtocolError::Login(format!(
                "Invalid login type: {}",
                value
            ))),
        }
    }
}

/// Client settings
#[derive(Debug, Clone, Default)]
pub struct ClientSettings {
    /// Window mode (0 = fixed, 1 = resizable)
    pub window_mode: u8,
    /// Screen width
    pub screen_width: u16,
    /// Screen height
    pub screen_height: u16,
    /// Detail settings
    pub detail: u8,
}

/// Login response to client
#[derive(Debug, Clone)]
pub struct LoginResponse {
    /// Response code
    pub code: u8,
    /// Player rights (0 = normal, 1 = player mod, 2 = mod, etc.)
    pub rights: u8,
    /// Player is flagged
    pub flagged: bool,
    /// Player index
    pub player_index: u16,
}

impl LoginResponse {
    /// Successful login
    pub fn success(player_index: u16) -> Self {
        Self {
            code: LoginResponseCode::SuccessNormal as u8,
            rights: 0,
            flagged: false,
            player_index,
        }
    }

    /// Failed login with specific code
    pub fn failed(code: LoginResponseCode) -> Self {
        Self {
            code: code as u8,
            rights: 0,
            flagged: false,
            player_index: 0,
        }
    }

    /// Encode into `out`.
    pub fn encode(&self, out: &mut rs_io::Packet) {
        out.p1(self.code);
        out.p1(self.rights);
        out.p1(if self.flagged { 1 } else { 0 });
        out.p2(self.player_index);
    }
}

/// Login decoder state machine
#[derive(Debug)]
pub struct LoginDecoder {
    /// RSA key for decrypting login blocks
    rsa_keys: &'static RsaKey,
}

impl LoginDecoder {
    /// Create a new login decoder with RSA keys
    pub fn new(rsa_keys: &'static RsaKey) -> Self {
        Self { rsa_keys }
    }

    /// Decode a login request from raw bytes.
    ///
    /// Rev 647 outer frame (from Static276.method4437, state 3):
    /// ```text
    /// login_type  : u8        (16 = new, 18 = reconnect)
    /// payload_len : u16 BE
    /// -- payload --
    /// revision    : i32 BE    (647)
    /// rsa_len     : u16 BE    (written by method8045 → method8040)
    /// rsa_data    : [u8; rsa_len]
    /// xtea_data   : [u8; ...]  (XTEA-encrypted with client seeds)
    /// ```
    ///
    /// RSA inner block (after decrypt):
    /// ```text
    /// magic       : u8        (10)
    /// seeds[0..4] : [i32; 4]  (16 bytes, ISAAC seeds)
    /// uid         : i64 BE    (8 bytes)
    /// password    : CString
    /// padding     : i64 BE    (0)
    /// client_rand : i64 BE    (random)
    /// ```
    ///
    /// XTEA inner block (after decrypt with seeds):
    /// ```text
    /// username    : CString
    /// ... client settings ...
    /// ```
    pub fn decode(&self, data: &[u8]) -> Result<LoginRequest> {
        if data.len() < 3 {
            return Err(ProtocolError::Login("Login packet too short".to_string()));
        }

        let mut buf = Packet::from(data.to_vec());

        // ── Outer frame ──────────────────────────────────────────────────────
        let login_type = buf.g1().try_into()?;
        let payload_len = buf.g2() as usize;

        let remaining = buf.data.len() - buf.pos;
        if remaining < payload_len {
            return Err(ProtocolError::Login(format!(
                "Login payload truncated: need {payload_len}, have {}",
                remaining
            )));
        }
        let payload_end = buf.pos + payload_len;

        // Revision (i32 BE, not u16)
        if payload_end - buf.pos < 4 {
            return Err(ProtocolError::Login("Missing revision".to_string()));
        }
        let revision = buf.g4s();
        if revision != 647 {
            return Err(ProtocolError::Login(format!(
                "Wrong revision: client={revision}, server=647"
            )));
        }

        // ── RSA block ────────────────────────────────────────────────────────
        let rsa_start = buf.pos;
        let rsa_len = buf.g2() as usize;
        if payload_end - buf.pos < rsa_len {
            return Err(ProtocolError::Login(format!(
                "RSA block truncated: need {rsa_len}, have {}",
                payload_end - buf.pos
            )));
        }

        let xtea_data = buf.data[buf.pos + rsa_len..payload_end].to_vec();

        tracing::debug!(
            "Login packet: type={:?}, revision={}, rsa_len={}, total_payload={}, xtea_remaining={}",
            login_type,
            revision,
            rsa_len,
            payload_len,
            xtea_data.len()
        );

        buf.pos = rsa_start;
        buf.rsadec(RsaFrame::Short, self.rsa_keys);

        tracing::debug!(
            "RSA decrypted: first bytes: {:02x?}",
            &buf.data[..buf.data.len().min(20)]
        );

        Self::parse_decrypted_block(login_type, &buf.data, &xtea_data)
    }

    /// Parse the decrypted RSA inner block and XTEA-encrypted section.
    ///
    /// RSA block: magic(10) | seeds[4×i32] | uid(i64) | username(CStr) | i64 | i64
    /// XTEA block (decrypted): password(CStr) | ... settings ...
    fn parse_decrypted_block(
        login_type: LoginType,
        rsa_data: &[u8],
        xtea_data: &[u8],
    ) -> Result<LoginRequest> {
        if rsa_data.len() < 25 {
            return Err(ProtocolError::Login(
                "Decrypted RSA block too short".to_string(),
            ));
        }

        let mut buf = Packet::from(rsa_data.to_vec());

        let magic = buf.g1();
        if magic != 10 {
            return Err(ProtocolError::Login(format!("Bad RSA magic: {magic}")));
        }

        // ISAAC seeds — signed i32s as the client transmits them
        let isaac_seeds = [buf.g4s(), buf.g4s(), buf.g4s(), buf.g4s()];

        // uid + password + two i64s
        let _uid = buf.g8s();
        let password = buf.gjstr(0).to_lowercase();
        // Skip the two trailing i64 values (padding + client random)
        if buf.data.len() - buf.pos >= 16 {
            let _ = buf.g8s();
            let _ = buf.g8s();
        }

        // ── XTEA section: decrypt with client seeds to get username + settings ──
        let (username, settings, archive_crcs) = if !xtea_data.is_empty() {
            let xtea_keys = [
                isaac_seeds[0] as u32,
                isaac_seeds[1] as u32,
                isaac_seeds[2] as u32,
                isaac_seeds[3] as u32,
            ];
            let mut decrypted = xtea_data.to_vec();
            let xtea_len = decrypted.len();
            // tinyenc only encrypts complete 8-byte blocks; tail bytes are plaintext
            let enc_len = xtea_len & !7;
            if enc_len > 0 {
                let _ = rs_crypto::xtea::Xtea::decrypt(&mut decrypted[..enc_len], &xtea_keys);
            }
            let mut dbuf = Packet::from(decrypted[..xtea_len].to_vec());
            let username = dbuf.gjstr(0).to_lowercase();

            let mut cs = ClientSettings::default();
            if login_type != LoginType::Lobby && dbuf.data.len() - dbuf.pos >= 5 {
                let _auth_type = dbuf.g1();
                cs.window_mode = dbuf.g1();
                cs.screen_width = dbuf.g2();
                cs.screen_height = dbuf.g2();
                tracing::debug!(
                    "Display settings: mode={} {}x{}",
                    cs.window_mode,
                    cs.screen_width,
                    cs.screen_height
                );
            }

            // CRCs: 35 × i32 written by Class130.method834 as the last thing
            // before tinyenc. Read from the known end of the original XTEA data.
            let mut archive_crcs = [0i32; 35];
            let crc_bytes = 35 * 4;
            if xtea_len >= crc_bytes {
                let mut crc_buf = Packet::from(decrypted[xtea_len - crc_bytes..xtea_len].to_vec());
                for crc in &mut archive_crcs {
                    *crc = crc_buf.g4s();
                }
                tracing::debug!(
                    "Client CRCs (first 5): [{:#010x}, {:#010x}, {:#010x}, {:#010x}, {:#010x}]",
                    archive_crcs[0],
                    archive_crcs[1],
                    archive_crcs[2],
                    archive_crcs[3],
                    archive_crcs[4],
                );
            }
            (username, cs, archive_crcs)
        } else {
            (String::new(), ClientSettings::default(), [0i32; 35])
        };

        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(ProtocolError::Login("Invalid username length".to_string()));
        }

        tracing::info!(
            "Login: user='{}' type={:?} display_mode={}",
            username,
            login_type,
            settings.window_mode
        );

        Ok(LoginRequest {
            login_type,
            revision: 647,
            username,
            password,
            isaac_seeds,
            settings,
            archive_crcs,
        })
    }

    /// Decode a create account request from the payload bytes (length prefix
    /// already consumed by the caller).
    ///
    /// Payload layout:
    /// ```text
    /// revision    : u16 BE   (647)
    /// rsa_data    : [u8; rsa_len]  (contains magic + 4 XTEA keys + padding)
    /// xtea_data   : [u8; ...]      (username, affiliate, password, metadata)
    /// ```
    pub fn decode_create_account(&self, data: &[u8]) -> Result<CreateAccountRequest> {
        if data.len() < 4 {
            return Err(ProtocolError::Login(
                "Create account packet too short".to_string(),
            ));
        }

        let mut buf = Packet::from(data.to_vec());
        let payload_end = data.len();

        let revision = buf.g2();
        if revision != 647 {
            return Err(ProtocolError::Login(format!(
                "Wrong revision: client={revision}, server=647"
            )));
        }

        let rsa_start = buf.pos;
        let rsa_len = buf.g2() as usize;
        if payload_end - buf.pos < rsa_len {
            return Err(ProtocolError::Login(format!(
                "RSA block truncated: need {rsa_len}, have {}",
                payload_end - buf.pos
            )));
        }

        let xtea_data = buf.data[buf.pos + rsa_len..payload_end].to_vec();

        buf.pos = rsa_start;
        buf.rsadec(RsaFrame::Short, self.rsa_keys);

        let magic = buf.g1();
        if magic != 10 {
            return Err(ProtocolError::Login(format!("Bad RSA magic: {magic}")));
        }

        let xtea_keys = [
            buf.g4s() as u32,
            buf.g4s() as u32,
            buf.g4s() as u32,
            buf.g4s() as u32,
        ];

        if xtea_data.is_empty() {
            return Err(ProtocolError::Login(
                "Missing XTEA data in create account".to_string(),
            ));
        }

        let mut decrypted = xtea_data;
        let enc_len = decrypted.len() & !7;
        if enc_len > 0 {
            let _ = rs_crypto::xtea::Xtea::decrypt(&mut decrypted[..enc_len], &xtea_keys);
        }
        let mut dbuf = Packet::from(decrypted);

        let username = dbuf.gjstr(0).to_lowercase();
        let _affiliate_id = dbuf.g2();
        let password = dbuf.gjstr(0);
        let _user_flow = dbuf.g8s();
        let _language = dbuf.g1();
        let _serial_id = dbuf.g1();
        // Skip 24 bytes of device info
        for _ in 0..24 {
            if dbuf.data.len() - dbuf.pos > 0 {
                dbuf.g1();
            }
        }
        let has_additional = if dbuf.data.len() - dbuf.pos > 0 {
            dbuf.g1() != 0
        } else {
            false
        };
        if has_additional {
            let _ = dbuf.gjstr(0);
        }
        let age = if dbuf.data.len() - dbuf.pos > 0 {
            dbuf.g1()
        } else {
            0
        };

        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(ProtocolError::Login("Invalid username length".to_string()));
        }

        Ok(CreateAccountRequest {
            username,
            password,
            age,
        })
    }

    /// Decode a check-name request (service byte 28).
    ///
    /// Same RSA+XTEA structure as create account, but the XTEA payload
    /// contains only the name + language + padding.
    pub fn decode_check_name(&self, data: &[u8]) -> Result<CheckNameRequest> {
        if data.len() < 4 {
            return Err(ProtocolError::Login(
                "Check name packet too short".to_string(),
            ));
        }

        let mut buf = Packet::from(data.to_vec());
        let payload_end = data.len();

        let revision = buf.g2();
        if revision != 647 {
            return Err(ProtocolError::Login(format!(
                "Wrong revision: client={revision}, server=647"
            )));
        }

        let rsa_start = buf.pos;
        let rsa_len = buf.g2() as usize;
        if payload_end - buf.pos < rsa_len {
            return Err(ProtocolError::Login(format!(
                "RSA block truncated: need {rsa_len}, have {}",
                payload_end - buf.pos
            )));
        }

        let xtea_data = buf.data[buf.pos + rsa_len..payload_end].to_vec();

        buf.pos = rsa_start;
        buf.rsadec(RsaFrame::Short, self.rsa_keys);

        let magic = buf.g1();
        if magic != 10 {
            return Err(ProtocolError::Login(format!("Bad RSA magic: {magic}")));
        }

        let xtea_keys = [
            buf.g4s() as u32,
            buf.g4s() as u32,
            buf.g4s() as u32,
            buf.g4s() as u32,
        ];

        if xtea_data.is_empty() {
            return Err(ProtocolError::Login(
                "Missing XTEA data in check name".to_string(),
            ));
        }

        let mut decrypted = xtea_data;
        let enc_len = decrypted.len() & !7;
        if enc_len > 0 {
            let _ = rs_crypto::xtea::Xtea::decrypt(&mut decrypted[..enc_len], &xtea_keys);
        }
        let mut dbuf = Packet::from(decrypted);

        let name = dbuf.gjstr(0).to_lowercase();

        if name.is_empty() || name.len() > MAX_USERNAME_LEN {
            return Err(ProtocolError::Login("Invalid name length".to_string()));
        }

        Ok(CheckNameRequest { name })
    }

    /// Get the RSA public key modulus (for client configuration)
    pub fn public_key_modulus(&self) -> Vec<u8> {
        self.rsa_keys.n.to_bytes_be().1
    }

    /// Get the RSA public key exponent (for client configuration)
    pub fn public_key_exponent(&self) -> Vec<u8> {
        self.rsa_keys.e.to_bytes_be().1
    }
}

/// Check name availability request (service byte 28)
#[derive(Debug, Clone)]
pub struct CheckNameRequest {
    pub name: String,
}

/// Create account request from client (service byte 22)
#[derive(Debug, Clone)]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
    pub age: u8,
}

/// Structural email validation: `local@domain.tld`.
pub fn is_valid_email(s: &str) -> bool {
    let Some((local, domain)) = s.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.is_empty() || s.matches('@').count() != 1 {
        return false;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return false;
    }
    if !local
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
    {
        return false;
    }
    if domain.starts_with('.')
        || domain.starts_with('-')
        || domain.ends_with('.')
        || domain.ends_with('-')
    {
        return false;
    }
    if !domain.contains('.') {
        return false;
    }
    if !domain
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
    {
        return false;
    }
    let tld = domain.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_type_from_u8() {
        assert!(matches!(LoginType::try_from(16), Ok(LoginType::New)));
        assert!(matches!(LoginType::try_from(18), Ok(LoginType::Reconnect)));
        assert!(LoginType::try_from(99).is_err());
    }

    #[test]
    fn test_login_response_encode() {
        let response = LoginResponse::success(42);
        let mut pkt = rs_io::Packet::new(16);
        response.encode(&mut pkt);
        let encoded = &pkt.data[..pkt.pos];

        assert_eq!(encoded[0], 2); // Success code
        assert_eq!(encoded[1], 0); // Rights
        assert_eq!(encoded[2], 0); // Not flagged
        assert_eq!(&encoded[3..5], &[0, 42]); // Player index (big endian)
    }

    #[test]
    fn test_login_response_failed() {
        let response = LoginResponse::failed(LoginResponseCode::InvalidCredentials);
        assert_eq!(response.code, 3);
    }

    #[test]
    fn test_valid_emails() {
        assert!(is_valid_email("user@example.com"));
        assert!(is_valid_email("user.name@example.com"));
        assert!(is_valid_email("user+tag@example.co.uk"));
        assert!(is_valid_email("a@b.cc"));
        assert!(is_valid_email("user-name@sub.domain.com"));
    }

    #[test]
    fn test_invalid_emails() {
        assert!(!is_valid_email("@@@"));
        assert!(!is_valid_email(""));
        assert!(!is_valid_email("noatsign"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@@domain.com"));
        assert!(!is_valid_email("user@domain"));
        assert!(!is_valid_email("user@.domain.com"));
        assert!(!is_valid_email("user@domain."));
        assert!(!is_valid_email("user@domain.c"));
        assert!(!is_valid_email(".user@domain.com"));
        assert!(!is_valid_email("user.@domain.com"));
        assert!(!is_valid_email("us..er@domain.com"));
        assert!(!is_valid_email("user@-domain.com"));
        assert!(!is_valid_email("user name@domain.com"));
    }
}
