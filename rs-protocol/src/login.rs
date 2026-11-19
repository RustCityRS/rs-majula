use crate::LoginResponse;
#[cfg(since_647)]
use crate::network::game::server::player_info::player_info_init;
use rs_crypto::rsa::RsaKey;
#[cfg(since_647)]
use rs_crypto::tinykey::TinyKey;
use rs_io::Packet;
#[cfg(before_530)]
use rs_io::crc::getcrc;
use rs_io::packet::RsaFrame;
#[cfg(all(since_530, before_647))]
use rs_util::base37::{is_valid_userhash, to_raw_username};
#[cfg(since_647)]
use rs_util::base37::{is_valid_userhash, to_userhash};
use rs_ver::STRING_TERMINATOR;
#[cfg(since_647)]
use std::net::Ipv4Addr;

#[cfg(before_530)]
pub const LOGIN_MAX_PAYLOAD: usize = u8::MAX as usize;
#[cfg(all(since_530, before_647))]
pub const LOGIN_MAX_PAYLOAD: usize = 159 + MAX_SETTINGS_LEN + 1 + 1 + u8::MAX as usize;
#[cfg(all(since_647, before_742))]
pub const LOGIN_MAX_PAYLOAD: usize = 4 + RSA_BLOCK_MAX + MAX_USERNAME_LEN + 1 + 6 + 140 + 8;
#[cfg(since_742)]
pub const LOGIN_MAX_PAYLOAD: usize = 4096;
#[cfg(before_530)]
const MAX_USERNAME_LEN: usize = 12;
#[cfg(since_647)]
const MAX_USERNAME_LEN: usize = 320;
#[cfg(all(since_647, before_742))]
const RSA_BLOCK_MAX: usize = 2 + 257;
/// The largest account service request: 647's create account.
#[cfg(all(since_647, before_742))]
pub const ACCOUNT_SERVICE_MAX_PAYLOAD: usize = 1014;
/// The reply to every account service: the clients show it as an error
/// contacting the create system. Accounts are made on the website.
#[cfg(all(since_530, before_742))]
pub const ACCOUNT_SERVICE_OFFLINE: u8 = 3;
#[cfg(all(since_530, before_647))]
const ARCHIVE_CRCS: usize = 28;
#[cfg(all(since_647, before_742))]
const ARCHIVE_CRCS: usize = 35;
#[cfg(since_742)]
const ARCHIVE_CRCS: usize = 37;
#[cfg(all(since_530, before_647))]
const MAX_SETTINGS_LEN: usize = 255;
#[cfg(all(since_530, before_647))]
const WORLD_SELECTED: u8 = 101;

pub struct LoginBlock {
    pub username: String,
    pub password: String,
    pub seed: [i32; 4],
    pub low_memory: bool,
    pub uid: Option<i32>,
    #[cfg(since_647)]
    pub client_session: i64,
    #[cfg(since_530)]
    pub settings: ClientSettings,
    #[cfg(since_530)]
    pub archive_crcs: [i32; ARCHIVE_CRCS],
    #[cfg(since_742)]
    pub reconnect: bool,
}

impl LoginBlock {
    #[cfg(before_530)]
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
            return Err(LoginResponse::OutOfDateClientVersion);
        }
        require(&buf, 1 + 9 * 4)?;
        let low_memory = buf.g1() & 0x1 != 0;
        let crc = getcrc(&buf.data, buf.pos, buf.pos + 9 * 4);
        buf.pos += 9 * 4;
        if crc != expected_crc {
            return Err(LoginResponse::OutOfDateClientVersion);
        }
        let (mut buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        require(&buf, 4)?;
        let uid = buf.g4s();
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?;
        let password = read_string(&mut buf, 20)?;
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::BadCredentials);
        }
        Ok(Self {
            username,
            password,
            seed,
            low_memory,
            uid: (uid != 0 && uid != 1337).then_some(uid),
        })
    }

    #[cfg(all(since_530, before_647))]
    pub fn decode(mut buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        require(&buf, 4)?;
        if buf.g4s() != 530 {
            return Err(LoginResponse::OutOfDateClientVersion);
        }
        require(&buf, 3 + 6 + 24)?;
        buf.pos += 3;
        let settings = ClientSettings {
            window_mode: buf.g1(),
            screen_width: buf.g2(),
            screen_height: buf.g2(),
            detail: buf.g1(),
        };
        buf.pos += 24;
        read_string(&mut buf, MAX_SETTINGS_LEN)?;
        let mut archive_crcs = [0; ARCHIVE_CRCS];
        require(&buf, 4 + 4 + 2 + size_of_val(&archive_crcs))?;
        buf.pos += 4 + 4 + 2;
        for crc in &mut archive_crcs {
            *crc = buf.g4s();
        }
        let (mut buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        let username = read_username37(&mut buf)?;
        let password = read_string(&mut buf, 20)?;
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

    #[cfg(since_647)]
    pub fn decode(
        mut buf: Packet,
        rsa: &'static RsaKey,
        lobby: bool,
    ) -> Result<Self, LoginResponse> {
        require(&buf, 4)?;
        if buf.g4s().to_string() != env!("REV") {
            return Err(LoginResponse::BadCredentials);
        }
        #[cfg(since_742)]
        let reconnect = {
            require(&buf, 4 + usize::from(!lobby))?;
            if buf.g4s() != 1 {
                return Err(LoginResponse::BadCredentials);
            }
            !lobby && buf.g1() == 1
        };
        let (mut rsa_buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        require(&rsa_buf, 8)?;
        rsa_buf.pos += 8;
        let password = read_string(&mut rsa_buf, 128)?.to_lowercase();
        require(&rsa_buf, 16)?;
        rsa_buf.g8s();
        let client_session = rsa_buf.g8s();
        let mut buf = decrypt_xtea(buf, seed)?;
        #[cfg(since_742)]
        {
            require(&buf, 1)?;
            if buf.g1() != 1 {
                return Err(LoginResponse::BadCredentials);
            }
        }
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?.to_lowercase();
        if username.len() > MAX_USERNAME_LEN || !is_valid_userhash(to_userhash(&username)) {
            return Err(LoginResponse::BadCredentials);
        }
        let mut settings = ClientSettings::default();
        if !lobby {
            #[cfg(before_742)]
            {
                require(&buf, 7)?;
                buf.pos += 1;
            }
            #[cfg(since_742)]
            require(&buf, 6)?;
            settings.window_mode = buf.g1();
            settings.screen_width = buf.g2();
            settings.screen_height = buf.g2();
            settings.detail = buf.g1();
        }
        let mut archive_crcs = [0; ARCHIVE_CRCS];
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
            client_session,
            settings,
            archive_crcs,
            #[cfg(since_742)]
            reconnect,
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
        .ok_or(LoginResponse::BadCredentials)?;
    if len > max {
        return Err(LoginResponse::BadCredentials);
    }
    Ok(buf.gjstr(STRING_TERMINATOR))
}

#[cfg(all(since_530, before_647))]
fn read_username37(buf: &mut Packet) -> Result<String, LoginResponse> {
    require(buf, 8)?;
    let username37 = buf.g8s() as u64;
    if !is_valid_userhash(username37) {
        return Err(LoginResponse::BadCredentials);
    }
    Ok(to_raw_username(username37))
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
    decrypted.rsadecrypt(RsaFrame::Byte, rsa);
    #[cfg(since_647)]
    decrypted.rsadecrypt(RsaFrame::Short, rsa);
    require(&decrypted, 17)?;
    if decrypted.g1() != 10 {
        return Err(LoginResponse::OutOfDateClientVersion);
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
    TinyKey::decrypt(&mut data[..len], &seed.map(|key| key as u32))
        .map_err(|_| LoginResponse::Rejected)?;
    Ok(Packet::from(data))
}

#[cfg(since_530)]
#[derive(Default)]
pub struct ClientSettings {
    pub window_mode: u8,
    pub screen_width: u16,
    pub screen_height: u16,
    pub detail: u8,
}

#[cfg(all(since_530, before_647))]
pub fn world_select_reply(world: u16) -> [u8; 3] {
    let [hi, lo] = world.to_be_bytes();
    [WORLD_SELECTED, hi, lo]
}

/// The game login reply. `member` is the player's own membership and
/// `map_members` the world's.
#[cfg(all(since_530, before_647))]
pub fn game_login_reply(staffmodlevel: u8, pid: u16, member: bool, map_members: bool) -> Vec<u8> {
    let mut buf = Packet::new(12);
    buf.p1(LoginResponse::LoginOk as u8);
    buf.p1(staffmodlevel);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p2(pid);
    buf.p1(member.into());
    buf.p1(map_members.into());
    buf.data
}

/// The player moderator rank the clients give staff level 1: 5 and 6 add
/// the mute option to reports.
#[cfg(since_647)]
pub const PLAYER_MOD_LEVEL: u8 = 5;

/// The game login reply. `member` is the player's own membership and
/// `map_members` the world's.
#[cfg(since_647)]
pub fn game_login_reply(
    staffmodlevel: u8,
    player_mod_level: u8,
    pid: u16,
    member: bool,
    map_members: bool,
    #[cfg(since_742)] display_name: &str,
) -> Vec<u8> {
    #[cfg(before_742)]
    let len = 13;
    #[cfg(since_742)]
    let len = 13 + Packet::sizejstr(display_name);
    let mut buf = Packet::new(2 + len);
    buf.p1(LoginResponse::LoginOk as u8);
    buf.p1(len as u8);
    buf.p1(staffmodlevel);
    buf.p1(player_mod_level);
    buf.p1(0);
    buf.p1(0);
    buf.p1(1);
    buf.p1(0);
    buf.p2(pid);
    buf.p1(member.into());
    buf.p3(0);
    buf.p1(map_members.into());
    #[cfg(since_742)]
    buf.pjstr(display_name, 0);
    buf.data
}

/// What a lobby login reply tells the client about the account. Days are
/// runedays, 0 for none: no recovery questions, no earlier login, no card.
/// Times are milliseconds: `members_until` since 1970 and `members_left`
/// from now, 0 for none. `play_age` is the account's age in minutes
/// (inferred from Jagex's own replies); the clients only test it for 0, a new
/// account, which the 647 lobby takes straight into the game.
#[cfg(since_647)]
pub struct LobbyAccount<'a> {
    pub display_name: &'a str,
    pub staff_mod_level: u8,
    pub player_mod_level: u8,
    pub underage: bool,
    pub chat_consent: bool,
    pub advert_consent: bool,
    #[cfg(since_742)]
    pub date_of_birth: i32,
    #[cfg(since_742)]
    pub gender: u8,
    pub members_until: i64,
    pub members_left: i64,
    pub member: bool,
    pub subscribed: bool,
    pub jcoins: i32,
    #[cfg(since_742)]
    pub loyalty_points: i32,
    pub recovery_day: u16,
    pub unread_messages: u16,
    pub last_login_day: u16,
    pub last_login_ip: Ipv4Addr,
    pub email_status: u8,
    pub card_expiry_day: u16,
    pub grace_expiry_day: u16,
    pub dob_requested: bool,
    pub play_age: i32,
    pub world: Option<(u16, &'a str)>,
}

/// The reply's "members stats" byte: no 647 or 742 script reads it.
#[cfg(since_647)]
const MEMBERS_STATS: u8 = 0;

/// The byte after the play age, which the name check also sets: no 647 or
/// 742 script reads it.
#[cfg(since_647)]
const NAME_CHECKED: u8 = 0;

/// 742's byte before the loyalty points: no 742 script reads it.
#[cfg(since_742)]
const LOYALTY_FLAG: u8 = 0;

/// The lobby's login reply.
#[cfg(since_647)]
pub fn lobby_login_reply(account: &LobbyAccount) -> Vec<u8> {
    #[cfg(before_742)]
    let len = 50 + Packet::sizejstr(account.display_name);
    #[cfg(since_742)]
    let len = 59 + Packet::sizejstr(account.display_name);
    let (world, address) = account
        .world
        .filter(|(_, addr)| len + addr.len() <= u8::MAX as usize)
        .unwrap_or((u16::MAX, ""));
    let len = len + address.len();
    let members_left = account.members_left.clamp(0, (1 << 40) - 1);
    let mut buf = Packet::new(2 + len);
    buf.p1(LoginResponse::LoginOk as u8);
    buf.p1(len as u8);
    buf.p1(account.staff_mod_level);
    buf.p1(account.player_mod_level);
    buf.p1(account.underage.into());
    #[cfg(since_742)]
    {
        buf.p3(account.date_of_birth);
        buf.p1(account.gender);
    }
    buf.p1(account.chat_consent.into());
    buf.p1(account.advert_consent.into());
    buf.p8(account.members_until);
    buf.p1((members_left >> 32) as u8);
    buf.p4(members_left as i32);
    buf.p1(u8::from(account.member) | u8::from(account.subscribed) << 1);
    buf.p4(account.jcoins);
    #[cfg(since_742)]
    {
        buf.p1(LOYALTY_FLAG);
        buf.p4(account.loyalty_points);
    }
    buf.p2(account.recovery_day);
    buf.p2(account.unread_messages);
    buf.p2(account.last_login_day);
    buf.p4(u32::from(account.last_login_ip) as i32);
    buf.p1(account.email_status);
    buf.p2(account.card_expiry_day);
    buf.p2(account.grace_expiry_day);
    buf.p1(account.dob_requested.into());
    pjstr2(&mut buf, account.display_name);
    buf.p1(MEMBERS_STATS);
    buf.p4(account.play_age);
    buf.p1(NAME_CHECKED);
    buf.p2(world);
    pjstr2(&mut buf, address);
    buf.data
}

#[cfg(since_647)]
fn pjstr2(buf: &mut Packet, s: &str) {
    buf.p1(0);
    buf.pjstr(s, 0);
}

/// A reconnecting 647 client resumes its session from its `PLAYER_INFO`
/// state again: the reply code, then that state behind a two-byte length.
#[cfg(since_647)]
pub fn reconnect_reply(pid: u16, x: u16, z: u16, level: u8, regions: &[u32]) -> Vec<u8> {
    let init = player_info_init(pid, x, z, level, regions);
    let mut buf = Packet::new(1 + 2 + init.pos);
    buf.p1(LoginResponse::ReconnectOk as u8);
    buf.p2(init.pos as u16);
    buf.pdata(&init.data, 0, init.pos);
    buf.data
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_crypto::rsa::parse_rsa_key_from_pem;
    #[cfg(all(since_530, before_647))]
    use rs_util::base37::to_userhash;
    #[cfg(since_530)]
    use std::array;
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

    fn rsa_block(id: i64) -> Packet {
        let mut buf = Packet::new(256);
        buf.p1(10);
        for key in SEED {
            buf.p4(key);
        }
        #[cfg(before_530)]
        {
            buf.p4(id as i32);
            buf.pjstr("Player", 10);
            buf.pjstr("Password£", 10);
            buf.rsaencrypt(RsaFrame::Byte, &KEY);
        }
        #[cfg(all(since_530, before_647))]
        {
            buf.p8(id);
            buf.pjstr("Password£", 0);
            buf.rsaencrypt(RsaFrame::Byte, &KEY);
        }
        #[cfg(since_647)]
        {
            buf.p8(id);
            buf.pjstr("Password£", 0);
            buf.p8(0);
            buf.p8(123);
            buf.rsaencrypt(RsaFrame::Short, &KEY);
        }
        buf.data.truncate(buf.pos);
        buf
    }

    #[cfg(before_530)]
    fn login_payload(uid: i32) -> Packet {
        let rsa = rsa_block(uid.into());
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

    #[cfg(all(since_530, before_647))]
    fn login_payload(revision: i32, username37: i64) -> Packet {
        let rsa = rsa_block(username37);
        let mut buf = Packet::new(512);
        buf.p4(revision);
        buf.p1(1);
        buf.p1(0);
        buf.p1(0);
        buf.p1(0);
        buf.p2(765);
        buf.p2(503);
        buf.p1(2);
        buf.pdata(&[0xFF; 24], 0, 24);
        buf.pjstr("settings", 0);
        buf.p4(0);
        buf.p4(0x0041_0000);
        buf.p2(7);
        for crc in 0..ARCHIVE_CRCS as i32 {
            buf.p4(crc - 13);
        }
        buf.pdata(&rsa.data, 0, rsa.pos);
        assert_eq!(buf.pos, 159 + rsa.pos + Packet::sizejstr("settings"));
        buf.data.truncate(buf.pos);
        buf.pos = 0;
        buf
    }

    #[cfg(since_647)]
    fn login_payload(lobby: bool, extra: usize) -> Packet {
        let rsa = rsa_block(123);
        let mut body = Packet::new(512);
        #[cfg(since_742)]
        body.p1(1);
        body.pjstr("Player", 0);
        if !lobby {
            #[cfg(before_742)]
            body.p1(0);
            body.p1(2);
            body.p2(800);
            body.p2(600);
            body.p1(4);
        }
        body.pdata(&[42; 24], 0, 24);
        body.pos += extra;
        for crc in 0..ARCHIVE_CRCS as i32 {
            body.p4(crc - 17);
        }
        let len = body.pos & !7;
        TinyKey::encrypt(&mut body.data[..len], &SEED.map(|key| key as u32)).unwrap();
        let mut buf = Packet::new(9 + rsa.pos + body.pos);
        buf.p4(env!("REV").parse().unwrap());
        #[cfg(since_742)]
        {
            buf.p4(1);
            if !lobby {
                buf.p1(1);
            }
        }
        buf.pdata(&rsa.data, 0, rsa.pos);
        buf.pdata(&body.data, 0, body.pos);
        buf.data.truncate(buf.pos);
        buf.pos = 0;
        buf
    }

    #[cfg(before_530)]
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
                Err(LoginResponse::OutOfDateClientVersion)
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
                assert_eq!(login.archive_crcs, array::from_fn(|i| i as i32 - 17));
                #[cfg(since_742)]
                assert_eq!(login.reconnect, !lobby);
                if !lobby {
                    assert_eq!(login.settings.window_mode, 2);
                    assert_eq!(login.settings.screen_width, 800);
                    assert_eq!(login.settings.screen_height, 600);
                    assert_eq!(login.settings.detail, 4);
                }
            }
        }
    }

    #[cfg(all(since_530, before_647))]
    #[test]
    fn game_login_keeps_settings_crcs_and_the_base37_name_in_the_rsa_tail() {
        let login =
            LoginBlock::decode(login_payload(530, to_userhash("Player") as i64), &KEY).unwrap();
        assert_eq!(login.username, "player");
        assert_eq!(login.password, "Password£");
        assert_eq!(login.seed, SEED);
        assert!(!login.low_memory);
        assert_eq!(login.uid, None);
        assert_eq!(login.archive_crcs, array::from_fn(|i| i as i32 - 13));
        assert_eq!(login.settings.window_mode, 0);
        assert_eq!(login.settings.screen_width, 765);
        assert_eq!(login.settings.screen_height, 503);
        assert_eq!(login.settings.detail, 2);
        assert!(matches!(
            LoginBlock::decode(login_payload(529, to_userhash("Player") as i64), &KEY),
            Err(LoginResponse::OutOfDateClientVersion)
        ));
        assert!(matches!(
            LoginBlock::decode(login_payload(530, 0), &KEY),
            Err(LoginResponse::BadCredentials)
        ));
    }

    #[cfg(all(since_530, before_647))]
    #[test]
    fn world_select_reply_is_the_selected_code_and_world() {
        assert_eq!(world_select_reply(0x0102), [101, 1, 2]);
    }

    #[test]
    fn login_rejects_truncated_headers_and_rsa_frames() {
        #[cfg(before_530)]
        let payload = login_payload(123);
        #[cfg(all(since_530, before_647))]
        let payload = login_payload(530, to_userhash("Player") as i64);
        #[cfg(since_647)]
        let payload = login_payload(false, 0);
        #[cfg(before_647)]
        let end = payload.len();
        #[cfg(all(since_647, before_742))]
        let rsa_at = 4;
        #[cfg(since_742)]
        let rsa_at = 9;
        #[cfg(since_647)]
        let end = rsa_at
            + 2
            + u16::from_be_bytes([payload.data[rsa_at], payload.data[rsa_at + 1]]) as usize;
        for len in 0..end {
            let result = LoginBlock::decode(
                Packet::from(payload.data[..len].to_vec()),
                &KEY,
                #[cfg(before_530)]
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

    #[cfg(all(since_530, before_647))]
    #[test]
    fn game_login_reply_matches_the_client_fields() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, 0x123, true, false),
                [2, level, 0, 0, 0, 0, 0, 0, 1, 0x23, 1, 0]
            );
        }
    }

    #[cfg(all(since_647, before_742))]
    #[test]
    fn game_login_reply_matches_the_client_fields() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, PLAYER_MOD_LEVEL, 0x123, true, false),
                [2, 13, level, 5, 0, 0, 1, 0, 1, 0x23, 1, 0, 0, 0, 0]
            );
        }
    }

    #[cfg(since_742)]
    #[test]
    fn game_login_reply_ends_with_the_display_name() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, PLAYER_MOD_LEVEL, 0x123, true, false, "Zezima"),
                [
                    2, 20, level, 5, 0, 0, 1, 0, 1, 0x23, 1, 0, 0, 0, 0, b'Z', b'e', b'z', b'i',
                    b'm', b'a', 0
                ]
            );
        }
    }

    #[cfg(since_647)]
    #[test]
    fn lobby_reply_fields_land_where_the_client_reads_them() {
        let reply = lobby_login_reply(&LobbyAccount {
            display_name: "Zezima",
            staff_mod_level: 2,
            player_mod_level: 5,
            underage: true,
            chat_consent: true,
            advert_consent: false,
            #[cfg(since_742)]
            date_of_birth: -4440,
            #[cfg(since_742)]
            gender: 1,
            members_until: 1_400_000_000_000,
            members_left: 0x01_0000_0002,
            member: true,
            subscribed: false,
            jcoins: 7,
            #[cfg(since_742)]
            loyalty_points: 8,
            recovery_day: 3000,
            unread_messages: 4,
            last_login_day: 9000,
            last_login_ip: Ipv4Addr::new(192, 0, 2, 1),
            email_status: 2,
            card_expiry_day: 9100,
            grace_expiry_day: 9200,
            dob_requested: true,
            play_age: 525_600,
            world: Some((2, "world2.example")),
        });
        let mut buf = Packet::from(reply);
        assert_eq!(buf.g1(), LoginResponse::LoginOk as u8);
        let len = buf.g1() as usize;
        let body = buf.pos;
        assert_eq!((buf.g1(), buf.g1(), buf.g1()), (2, 5, 1));
        #[cfg(since_742)]
        assert_eq!((buf.g3() - 16_777_216, buf.g1()), (-4440, 1));
        assert_eq!((buf.g1(), buf.g1(), buf.g8s()), (1, 0, 1_400_000_000_000));
        assert_eq!((buf.g1(), buf.g4s()), (1, 2));
        assert_eq!((buf.g1(), buf.g4s()), (1, 7));
        #[cfg(since_742)]
        assert_eq!((buf.g1(), buf.g4s()), (LOYALTY_FLAG, 8));
        assert_eq!((buf.g2(), buf.g2(), buf.g2()), (3000, 4, 9000));
        assert_eq!(buf.g4s() as u32, u32::from(Ipv4Addr::new(192, 0, 2, 1)));
        assert_eq!((buf.g1(), buf.g2(), buf.g2(), buf.g1()), (2, 9100, 9200, 1));
        assert_eq!(buf.g1(), 0);
        assert_eq!(buf.gjstr(0), "Zezima");
        assert_eq!(
            (buf.g1(), buf.g4s(), buf.g1()),
            (MEMBERS_STATS, 525_600, NAME_CHECKED)
        );
        assert_eq!(buf.g2(), 2);
        assert_eq!(buf.g1(), 0);
        assert_eq!(buf.gjstr(0), "world2.example");
        assert_eq!(buf.pos - body, len);
        assert_eq!(buf.pos, buf.len());
    }

    #[cfg(since_647)]
    #[test]
    fn reconnect_reply_is_the_state_the_client_resumes_from() {
        let mut regions = vec![0u32; 2048];
        regions[7] = (1 << 16) | (50 << 8) | 51;
        regions[2047] = (3 << 16) | (255 << 8) | 254;
        let reply = reconnect_reply(9, 3222, 3218, 2, &regions);
        assert_eq!(reply[0], 15);
        let len = u16::from_be_bytes([reply[1], reply[2]]) as usize;
        assert_eq!(
            reply.len(),
            3 + len,
            "the client checks it read exactly the block"
        );
        let block = &reply[3..];
        let mut pos = 0;
        let mut gbit = |n: usize| {
            let mut v = 0u32;
            for _ in 0..n {
                v = (v << 1) | ((block[pos >> 3] >> (7 - (pos & 7))) & 1) as u32;
                pos += 1;
            }
            v
        };
        let own = gbit(30);
        assert_eq!(
            (own >> 28, (own >> 14) & 0x3FFF, own & 0x3FFF),
            (2, 3222, 3218)
        );
        let mut read = vec![0u32; 2048];
        for (slot, region) in read.iter_mut().enumerate().skip(1) {
            if slot != 9 {
                *region = gbit(18);
            }
        }
        assert_eq!(read, regions);
        assert_eq!((pos + 7) >> 3, len);
    }
}
