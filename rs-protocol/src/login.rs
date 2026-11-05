use crate::LoginResponse;
use rs_crypto::rsa::RsaKey;
#[cfg(since_647)]
use rs_crypto::xtea::Xtea;
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::crc::getcrc;
use rs_io::packet::RsaFrame;
use rs_ver::STRING_TERMINATOR;

#[cfg(before_647)]
pub const LOGIN_MAX_PAYLOAD: usize = u8::MAX as usize;
#[cfg(since_647)]
pub const LOGIN_MAX_PAYLOAD: usize = 4 + RSA_BLOCK_MAX + MAX_USERNAME_LEN + 1 + 6 + 140 + 8;
#[cfg(before_647)]
const MAX_USERNAME_LEN: usize = 12;
#[cfg(since_647)]
const MAX_USERNAME_LEN: usize = 320;
#[cfg(since_647)]
const RSA_BLOCK_MAX: usize = 2 + 257;
#[cfg(since_647)]
pub const CREATE_ACCOUNT_MAX_PAYLOAD: usize =
    2 + RSA_BLOCK_MAX + 2 + 8 + 1 + 1 + 24 + 1 + 1 + 1 + 7 + 321 + 129 + 257;
#[cfg(since_647)]
pub const CHECK_NAME_MAX_PAYLOAD: usize = 2 + RSA_BLOCK_MAX + 1 + 7 + 321;

pub struct LoginBlock {
    pub username: String,
    pub password: String,
    pub seed: [i32; 4],
    pub low_memory: bool,
    pub uid: Option<i32>,
    #[cfg(since_647)]
    pub settings: ClientSettings,
    #[cfg(since_647)]
    pub archive_crcs: [i32; 35],
}

impl LoginBlock {
    #[cfg(before_647)]
    pub fn decode(
        mut buf: Packet,
        rsa: &'static RsaKey,
        expected_crc: i32,
    ) -> Result<Self, LoginResponse> {
        require(&buf, 1)?;
        let mut revision = u16::from(buf.g1());
        if revision == 255 {
            require(&buf, 2)?;
            revision = buf.g2();
        }
        if revision.to_string() != env!("REV").split('.').next().unwrap() {
            return Err(LoginResponse::RuneScapeUpdated);
        }
        require(&buf, 1 + 9 * 4)?;
        let low_memory = buf.g1() & 0x1 != 0;
        let crc = getcrc(&buf.data, buf.pos, buf.pos + 9 * 4);
        buf.pos += 9 * 4;
        if crc != expected_crc {
            return Err(LoginResponse::RuneScapeUpdated);
        }
        let (mut buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        require(&buf, 4)?;
        let uid = buf.g4s();
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?;
        let password = read_string(&mut buf, 20)?;
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::InvalidCredentials);
        }
        Ok(Self {
            username,
            password,
            seed,
            low_memory,
            uid: (uid != 0 && uid != 1337).then_some(uid),
        })
    }

    #[cfg(since_647)]
    pub fn decode(
        mut buf: Packet,
        rsa: &'static RsaKey,
        lobby: bool,
    ) -> Result<Self, LoginResponse> {
        require(&buf, 4)?;
        if buf.g4s() != 647 {
            return Err(LoginResponse::InvalidCredentials);
        }
        let (mut rsa_buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        require(&rsa_buf, 8)?;
        rsa_buf.pos += 8;
        let password = read_string(&mut rsa_buf, 128)?.to_lowercase();
        require(&rsa_buf, 16)?;
        let mut buf = decrypt_xtea(buf, seed)?;
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?.to_lowercase();
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::InvalidCredentials);
        }
        let mut settings = ClientSettings::default();
        if !lobby {
            require(&buf, 7)?;
            buf.pos += 1;
            settings.window_mode = buf.g1();
            settings.screen_width = buf.g2();
            settings.screen_height = buf.g2();
            settings.detail = buf.g1();
        }
        let mut archive_crcs = [0; 35];
        require(&buf, size_of_val(&archive_crcs))?;
        buf.pos = buf.len() - size_of_val(&archive_crcs);
        for crc in &mut archive_crcs {
            *crc = buf.g4s();
        }
        Ok(Self {
            username,
            password,
            seed,
            low_memory: false,
            uid: None,
            settings,
            archive_crcs,
        })
    }
}

fn require(buf: &Packet, len: usize) -> Result<(), LoginResponse> {
    if buf.len().saturating_sub(buf.pos) < len {
        return Err(LoginResponse::Rejected);
    }
    Ok(())
}

fn read_string(buf: &mut Packet, max: usize) -> Result<String, LoginResponse> {
    let len = buf.data[buf.pos..]
        .iter()
        .position(|&b| b == STRING_TERMINATOR)
        .ok_or(LoginResponse::InvalidCredentials)?;
    if len > max {
        return Err(LoginResponse::InvalidCredentials);
    }
    Ok(buf.gjstr(STRING_TERMINATOR))
}

fn decrypt_rsa(
    buf: &mut Packet,
    rsa: &'static RsaKey,
) -> Result<(Packet, [i32; 4]), LoginResponse> {
    let start = buf.pos;
    #[cfg(before_647)]
    let len = {
        require(buf, 1)?;
        buf.g1() as usize
    };
    #[cfg(since_647)]
    let len = {
        require(buf, 2)?;
        buf.g2() as usize
    };
    require(buf, len)?;
    if len == 0 {
        return Err(LoginResponse::Rejected);
    }
    buf.pos += len;
    let mut decrypted = Packet::from(buf.data[start..buf.pos].to_vec());
    let modulus = rsa.n.bits().div_ceil(8) as usize;
    decrypted.data.resize(decrypted.len().max(modulus), 0);
    #[cfg(before_647)]
    decrypted.rsadec(RsaFrame::Byte, rsa);
    #[cfg(since_647)]
    decrypted.rsadec(RsaFrame::Short, rsa);
    require(&decrypted, 17)?;
    if decrypted.g1() != 10 {
        return Err(LoginResponse::RuneScapeUpdated);
    }
    let seed = [
        decrypted.g4s(),
        decrypted.g4s(),
        decrypted.g4s(),
        decrypted.g4s(),
    ];
    Ok((decrypted, seed))
}

#[cfg(since_647)]
fn decrypt_xtea(buf: Packet, seed: [i32; 4]) -> Result<Packet, LoginResponse> {
    let mut data = buf.data;
    data.drain(..buf.pos);
    let len = data.len() & !7;
    Xtea::decrypt(&mut data[..len], &seed.map(|key| key as u32))
        .map_err(|_| LoginResponse::Rejected)?;
    Ok(Packet::from(data))
}

#[cfg(since_647)]
fn decode_account(mut buf: Packet, rsa: &'static RsaKey) -> Result<Packet, LoginResponse> {
    require(&buf, 2)?;
    if buf.g2() != 647 {
        return Err(LoginResponse::RuneScapeUpdated);
    }
    let (_, seed) = decrypt_rsa(&mut buf, rsa)?;
    decrypt_xtea(buf, seed)
}

#[cfg(since_647)]
#[derive(Default)]
pub struct ClientSettings {
    pub window_mode: u8,
    pub screen_width: u16,
    pub screen_height: u16,
    pub detail: u8,
}

#[cfg(since_647)]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
    pub age: u8,
}

#[cfg(since_647)]
impl CreateAccountRequest {
    pub fn decode(buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        let mut buf = decode_account(buf, rsa)?;
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?.to_lowercase();
        require(&buf, 2)?;
        buf.pos += 2;
        let password = read_string(&mut buf, 128)?;
        require(&buf, 8 + 1 + 1 + 24 + 1)?;
        buf.pos += 8 + 1 + 1 + 24;
        if buf.g1() != 0 {
            read_string(&mut buf, 256)?;
        }
        require(&buf, 2 + 7)?;
        let age = buf.g1();
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::InvalidCredentials);
        }
        Ok(Self {
            username,
            password,
            age,
        })
    }
}

#[cfg(since_647)]
pub struct CheckNameRequest {
    pub name: String,
}

#[cfg(since_647)]
impl CheckNameRequest {
    pub fn decode(buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        let mut buf = decode_account(buf, rsa)?;
        let name = read_string(&mut buf, MAX_USERNAME_LEN)?.to_lowercase();
        require(&buf, 1 + 7)?;
        if name.is_empty() || name.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::InvalidCredentials);
        }
        Ok(Self { name })
    }
}

#[cfg(since_647)]
pub fn game_login_reply(staffmodlevel: u8, pid: u16) -> Vec<u8> {
    let mut buf = Packet::new(15);
    buf.p1(LoginResponse::SuccessNormal as u8);
    buf.p1(13);
    buf.p1(staffmodlevel);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(1);
    buf.p1(0);
    buf.p2(pid);
    buf.p1(1);
    buf.p3(0);
    buf.p1(1);
    buf.data
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_crypto::rsa::parse_rsa_key_from_pem;
    use std::sync::LazyLock;

    static KEY: LazyLock<RsaKey> = LazyLock::new(|| {
        parse_rsa_key_from_pem(
            "-----BEGIN PRIVATE KEY-----
MIIBVAIBADANBgkqhkiG9w0BAQEFAASCAT4wggE6AgEAAkEAzLUczq12qOSGm+Pv
HSiJ+BPm/Z7vKITVMcUbipwycMcYZF8TjWiiShYLuVtWLA5mEE86FKvrmaf79/tG
RTB6LQIDAQABAkEAu3ZgGyTfNWuRmdDyeFFXh8cyEbAqc4CxfBJ1VkoUJxGJDM+z
m9DXk/vYvL5G+PbNOOHIE2VKNkWbdgX3OQ8GQQIhAOWUQkJEfVYNX8whrjFEzd9j
SprujROHi4IKxzI1o4mrAiEA5EQY/mqL8LIEM7+1UXBN8dPJlSmcxcyRGwL39pTh
o4cCIB0xw1NF/mJJBRuiVNJzG3MC32PgXhRTskvxLu+VnpxNAiBm59BAufXWj9pX
HgD28uMgtzK0fSsA/QUZoU/6KQpD9wIgGDW/g+ztlu98aA9+zzaKbU77J/pDsAdN
Az5KOlJPE6c=
-----END PRIVATE KEY-----",
        )
        .unwrap()
    });
    const SEED: [i32; 4] = [1, -2, 3, i32::MIN];

    fn rsa_block(uid: i32) -> Packet {
        let mut buf = Packet::new(256);
        buf.p1(10);
        for key in SEED {
            buf.p4(key);
        }
        #[cfg(before_647)]
        {
            buf.p4(uid);
            buf.pjstr("Player", 10);
            buf.pjstr("Password£", 10);
            buf.rsaenc(RsaFrame::Byte, &KEY);
        }
        #[cfg(since_647)]
        {
            buf.p8(uid as i64);
            buf.pjstr("Password£", 0);
            buf.p8(0);
            buf.p8(123);
            buf.rsaenc(RsaFrame::Short, &KEY);
        }
        buf.data.truncate(buf.pos);
        buf
    }

    #[cfg(before_647)]
    fn login_payload(uid: i32) -> Packet {
        let rsa = rsa_block(uid);
        let mut buf = Packet::new(300);
        let revision = env!("REV")
            .split('.')
            .next()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        if revision >= 255 {
            buf.p1(255);
            buf.p2(revision);
        } else {
            buf.p1(revision as u8);
        }
        buf.p1(1);
        for crc in 0..9 {
            buf.p4(crc);
        }
        buf.pdata(&rsa.data, 0, rsa.pos);
        buf.data.truncate(buf.pos);
        buf.pos = 0;
        buf
    }

    #[cfg(since_647)]
    fn login_payload(lobby: bool, extra: usize) -> Packet {
        let rsa = rsa_block(123);
        let mut body = Packet::new(512);
        body.pjstr("Player", 0);
        if !lobby {
            body.p1(0);
            body.p1(2);
            body.p2(800);
            body.p2(600);
            body.p1(4);
        }
        body.pdata(&[42; 24], 0, 24);
        body.pos += extra;
        for crc in 0..35 {
            body.p4(crc - 17);
        }
        let len = body.pos & !7;
        Xtea::encrypt(&mut body.data[..len], &SEED.map(|key| key as u32)).unwrap();
        let mut buf = Packet::new(4 + rsa.pos + body.pos);
        buf.p4(647);
        buf.pdata(&rsa.data, 0, rsa.pos);
        buf.pdata(&body.data, 0, body.pos);
        buf.pos = 0;
        buf
    }

    #[cfg(before_647)]
    #[test]
    fn login_preserves_credentials_seeds_crc_and_install_id() {
        for uid in [0, 1337, 123456] {
            let payload = login_payload(uid);
            let crc_start = if payload.data[0] == 255 { 4 } else { 2 };
            let crc = getcrc(&payload.data[crc_start..crc_start + 36], 0, 36);
            let login = LoginBlock::decode(payload, &KEY, crc).unwrap();
            assert_eq!(login.username, "Player");
            assert_eq!(login.password, "Password£");
            assert_eq!(login.seed, SEED);
            assert!(login.low_memory);
            assert_eq!(login.uid, (uid == 123456).then_some(uid));
            assert!(matches!(
                LoginBlock::decode(login_payload(uid), &KEY, crc ^ 1),
                Err(LoginResponse::RuneScapeUpdated)
            ));
        }
    }

    #[cfg(since_647)]
    #[test]
    fn game_and_lobby_login_keep_plaintext_xtea_tail_and_crcs() {
        for lobby in [false, true] {
            for extra in 0..8 {
                let login = LoginBlock::decode(login_payload(lobby, extra), &KEY, lobby).unwrap();
                assert_eq!(login.username, "player");
                assert_eq!(login.password, "password£");
                assert_eq!(login.seed, SEED);
                assert!(!login.low_memory);
                assert_eq!(login.uid, None);
                assert_eq!(login.archive_crcs, std::array::from_fn(|i| i as i32 - 17));
                if !lobby {
                    assert_eq!(login.settings.window_mode, 2);
                    assert_eq!(login.settings.screen_width, 800);
                    assert_eq!(login.settings.screen_height, 600);
                    assert_eq!(login.settings.detail, 4);
                }
            }
        }
    }

    #[test]
    fn login_rejects_truncated_headers_and_rsa_frames() {
        #[cfg(before_647)]
        let payload = login_payload(123);
        #[cfg(since_647)]
        let payload = login_payload(false, 0);
        #[cfg(before_647)]
        let end = payload.len();
        #[cfg(since_647)]
        let end = 6 + u16::from_be_bytes([payload.data[4], payload.data[5]]) as usize;
        for len in 0..end {
            let result = LoginBlock::decode(
                Packet::from(payload.data[..len].to_vec()),
                &KEY,
                #[cfg(before_647)]
                {
                    let start = if payload.data[0] == 255 { 4 } else { 2 };
                    getcrc(&payload.data[start..start + 36], 0, 36)
                },
                #[cfg(since_647)]
                false,
            );
            assert!(result.is_err(), "accepted truncation at {len}");
        }
        let mut string = Packet::from(vec![65; 32]);
        assert!(read_string(&mut string, 320).is_err());
    }

    #[cfg(since_647)]
    #[test]
    fn account_services_share_the_rsa_and_xtea_envelope() {
        for create in [false, true] {
            let mut rsa = Packet::new(256);
            rsa.p1(10);
            for key in SEED {
                rsa.p4(key);
            }
            rsa.pos += 42;
            rsa.rsaenc(RsaFrame::Short, &KEY);
            let mut body = Packet::new(512);
            body.pjstr("Player@Example.com", 0);
            if create {
                body.p2(12);
                body.pjstr("PassWord£", 0);
                body.p8(100);
                body.p1(0);
                body.p1(0);
                body.pos += 24;
                body.p1(1);
                body.pjstr("additional", 0);
                body.p1(25);
                body.p1(1);
            } else {
                body.p1(0);
            }
            body.pos += 7;
            let len = body.pos & !7;
            Xtea::encrypt(&mut body.data[..len], &SEED.map(|key| key as u32)).unwrap();
            let mut payload = Packet::new(2 + rsa.pos + body.pos);
            payload.p2(647);
            payload.pdata(&rsa.data, 0, rsa.pos);
            payload.pdata(&body.data, 0, body.pos);
            payload.pos = 0;
            if create {
                let request = CreateAccountRequest::decode(payload, &KEY).unwrap();
                assert_eq!(request.username, "player@example.com");
                assert_eq!(request.password, "PassWord£");
                assert_eq!(request.age, 25);
            } else {
                assert_eq!(
                    CheckNameRequest::decode(payload, &KEY).unwrap().name,
                    "player@example.com"
                );
            }
        }
    }

    #[cfg(since_647)]
    #[test]
    fn game_login_reply_matches_the_client_fields() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, 0x123),
                [2, 13, level, 0, 0, 0, 1, 0, 1, 0x23, 1, 0, 0, 0, 1]
            );
        }
    }
}
