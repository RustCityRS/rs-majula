use crate::LoginResponse;
#[cfg(since_647)]
use crate::network::game::server::player_info::player_info_init;
use rs_crypto::rsa::RsaKey;
#[cfg(since_647)]
use rs_crypto::xtea::Xtea;
use rs_io::Packet;
#[cfg(before_530)]
use rs_io::crc::getcrc;
use rs_io::packet::RsaFrame;
#[cfg(all(since_530, before_647))]
use rs_util::base37::{is_valid_userhash, to_raw_username};
use rs_ver::STRING_TERMINATOR;

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
#[cfg(all(since_647, before_742))]
pub const CREATE_ACCOUNT_MAX_PAYLOAD: usize =
    2 + RSA_BLOCK_MAX + 2 + 8 + 1 + 1 + 24 + 1 + 1 + 1 + 7 + 321 + 129 + 257;
#[cfg(all(since_647, before_742))]
pub const CHECK_NAME_MAX_PAYLOAD: usize = 2 + RSA_BLOCK_MAX + 1 + 7 + 321;
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
#[cfg(all(since_530, before_647))]
const NAME_SUGGESTIONS: u8 = 21;

pub struct LoginBlock {
    pub username: String,
    pub password: String,
    pub seed: [i32; 4],
    pub low_memory: bool,
    pub uid: Option<i32>,
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

    #[cfg(all(since_530, before_647))]
    pub fn decode(mut buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        require(&buf, 4)?;
        if buf.g4s() != 530 {
            return Err(LoginResponse::RuneScapeUpdated);
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
            return Err(LoginResponse::InvalidCredentials);
        }
        #[cfg(since_742)]
        let reconnect = {
            require(&buf, 4 + usize::from(!lobby))?;
            if buf.g4s() != 1 {
                return Err(LoginResponse::InvalidCredentials);
            }
            !lobby && buf.g1() == 1
        };
        let (mut rsa_buf, seed) = decrypt_rsa(&mut buf, rsa)?;
        require(&rsa_buf, 8)?;
        rsa_buf.pos += 8;
        let password = read_string(&mut rsa_buf, 128)?.to_lowercase();
        require(&rsa_buf, 16)?;
        let mut buf = decrypt_xtea(buf, seed)?;
        #[cfg(since_742)]
        {
            require(&buf, 1)?;
            if buf.g1() != 1 {
                return Err(LoginResponse::InvalidCredentials);
            }
        }
        let username = read_string(&mut buf, MAX_USERNAME_LEN)?.to_lowercase();
        if username.is_empty() || username.len() > MAX_USERNAME_LEN {
            return Err(LoginResponse::InvalidCredentials);
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
        .ok_or(LoginResponse::InvalidCredentials)?;
    if len > max {
        return Err(LoginResponse::InvalidCredentials);
    }
    Ok(buf.gjstr(STRING_TERMINATOR))
}

#[cfg(all(since_530, before_647))]
fn read_username37(buf: &mut Packet) -> Result<String, LoginResponse> {
    require(buf, 8)?;
    let username37 = buf.g8s() as u64;
    if !is_valid_userhash(username37) {
        return Err(LoginResponse::InvalidCredentials);
    }
    Ok(to_raw_username(username37))
}

fn decrypt_rsa(
    buf: &mut Packet,
    rsa: &'static RsaKey,
) -> Result<(Packet, [i32; 4]), LoginResponse> {
    let mut decrypted = decrypt_rsa_block(buf, rsa, 17)?;
    let seed = [
        decrypted.g4s(),
        decrypted.g4s(),
        decrypted.g4s(),
        decrypted.g4s(),
    ];
    Ok((decrypted, seed))
}

fn decrypt_rsa_block(
    buf: &mut Packet,
    rsa: &'static RsaKey,
    min: usize,
) -> Result<Packet, LoginResponse> {
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
    require(&decrypted, min)?;
    if decrypted.g1() != 10 {
        return Err(LoginResponse::RuneScapeUpdated);
    }
    Ok(decrypted)
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

#[cfg(all(since_647, before_742))]
fn decode_account(mut buf: Packet, rsa: &'static RsaKey) -> Result<Packet, LoginResponse> {
    require(&buf, 2)?;
    if buf.g2() != 647 {
        return Err(LoginResponse::RuneScapeUpdated);
    }
    let (_, seed) = decrypt_rsa(&mut buf, rsa)?;
    decrypt_xtea(buf, seed)
}

#[cfg(all(since_530, before_647))]
fn decode_account(
    mut buf: Packet,
    rsa: &'static RsaKey,
) -> Result<(Packet, String, String), LoginResponse> {
    let mut buf = decrypt_rsa_block(&mut buf, rsa, 1 + 2 + 2)?;
    buf.pos += 2;
    if buf.g2() != 530 {
        return Err(LoginResponse::RuneScapeUpdated);
    }
    let username = read_username37(&mut buf)?;
    require(&buf, 4)?;
    buf.pos += 4;
    let password = read_string(&mut buf, 20)?;
    require(&buf, 4)?;
    buf.pos += 4;
    Ok((buf, username, password))
}

#[cfg(since_530)]
#[derive(Default)]
pub struct ClientSettings {
    pub window_mode: u8,
    pub screen_width: u16,
    pub screen_height: u16,
    pub detail: u8,
}

#[cfg(all(since_530, before_742))]
pub struct CreateAccountRequest {
    pub username: String,
    pub password: String,
    #[cfg(all(since_530, before_647))]
    pub day: u8,
    #[cfg(all(since_530, before_647))]
    pub month: u8,
    #[cfg(all(since_530, before_647))]
    pub year: u16,
    #[cfg(all(since_530, before_647))]
    pub country: u16,
    #[cfg(since_647)]
    pub age: u8,
}

#[cfg(all(since_530, before_647))]
impl CreateAccountRequest {
    pub fn decode(buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        let (mut buf, username, password) = decode_account(buf, rsa)?;
        require(&buf, 2 + 1 + 1 + 4 + 2 + 2)?;
        buf.pos += 2;
        let day = buf.g1();
        let month = buf.g1();
        buf.pos += 4;
        let year = buf.g2();
        let country = buf.g2();
        Ok(Self {
            username,
            password,
            day,
            month,
            year,
            country,
        })
    }
}

#[cfg(all(since_647, before_742))]
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

#[cfg(all(since_530, before_742))]
pub struct CheckNameRequest {
    pub name: String,
}

#[cfg(all(since_530, before_647))]
impl CheckNameRequest {
    pub fn decode(mut buf: Packet) -> Result<Self, LoginResponse> {
        Ok(Self {
            name: read_username37(&mut buf)?,
        })
    }
}

#[cfg(all(since_530, before_647))]
pub fn name_suggestions_reply(names: &[i64]) -> Vec<u8> {
    let mut buf = Packet::new(2 + (names.len() << 3));
    buf.p1(NAME_SUGGESTIONS);
    buf.p1(names.len() as u8);
    for &name in names {
        buf.p8(name);
    }
    buf.data
}

#[cfg(all(since_530, before_647))]
pub struct WorldSelectRequest {
    pub username: String,
    pub password: String,
}

#[cfg(all(since_530, before_647))]
impl WorldSelectRequest {
    pub fn decode(buf: Packet, rsa: &'static RsaKey) -> Result<Self, LoginResponse> {
        let (_, username, password) = decode_account(buf, rsa)?;
        Ok(Self { username, password })
    }
}

#[cfg(all(since_530, before_647))]
pub fn world_select_reply(world: u16) -> [u8; 3] {
    let [hi, lo] = world.to_be_bytes();
    [WORLD_SELECTED, hi, lo]
}

#[cfg(all(since_530, before_647))]
pub struct WorldListRequest {
    pub checksum: i32,
}

#[cfg(all(since_530, before_647))]
impl WorldListRequest {
    pub fn decode(mut buf: Packet) -> Result<Self, LoginResponse> {
        require(&buf, 4)?;
        Ok(Self {
            checksum: buf.g4s(),
        })
    }
}

#[cfg(all(since_530, before_647))]
pub struct Country<'a> {
    pub flag: u16,
    pub name: &'a str,
}

#[cfg(all(since_530, before_647))]
pub struct World<'a> {
    pub id: u16,
    pub country: u8,
    pub flags: i32,
    pub activity: &'a str,
    pub host: &'a str,
    pub players: u16,
}

#[cfg(all(since_530, before_647))]
pub struct WorldList<'a> {
    pub countries: &'a [Country<'a>],
    pub worlds: &'a [World<'a>],
    pub checksum: i32,
}

#[cfg(all(since_530, before_647))]
impl WorldList<'_> {
    pub fn encode(&self, full: bool) -> Vec<u8> {
        let min = self.worlds.iter().map(|world| world.id).min().unwrap_or(0);
        let max = self.worlds.iter().map(|world| world.id).max().unwrap_or(0);
        let mut len = 2 + self.sizeof_players(min);
        if full {
            len += self.sizeof_list(min, max);
        }
        let mut buf = Packet::new(1 + 2 + len);
        buf.p1(0);
        buf.p2(len as u16);
        buf.p1(1);
        buf.p1(full as u8);
        if full {
            buf.psmart1or2(self.countries.len() as i32);
            for country in self.countries {
                buf.psmart1or2(i32::from(country.flag));
                buf.p1(0);
                buf.pjstr(country.name, 0);
            }
            buf.psmart1or2(i32::from(min));
            buf.psmart1or2(i32::from(max));
            buf.psmart1or2(self.worlds.len() as i32);
            for world in self.worlds {
                buf.psmart1or2(i32::from(world.id - min));
                buf.p1(world.country);
                buf.p4(world.flags);
                buf.p1(0);
                buf.pjstr(world.activity, 0);
                buf.p1(0);
                buf.pjstr(world.host, 0);
            }
            buf.p4(self.checksum);
        }
        for world in self.worlds {
            buf.psmart1or2(i32::from(world.id - min));
            buf.p2(world.players);
        }
        buf.data
    }

    fn sizeof_list(&self, min: u16, max: u16) -> usize {
        let countries: usize = self
            .countries
            .iter()
            .map(|country| sizeof_smart(country.flag) + 1 + Packet::sizejstr(country.name))
            .sum();
        let worlds: usize = self
            .worlds
            .iter()
            .map(|world| {
                sizeof_smart(world.id - min)
                    + size_of_val(&world.country)
                    + size_of_val(&world.flags)
                    + 1
                    + Packet::sizejstr(world.activity)
                    + 1
                    + Packet::sizejstr(world.host)
            })
            .sum();
        sizeof_smart(self.countries.len() as u16)
            + countries
            + sizeof_smart(min)
            + sizeof_smart(max)
            + sizeof_smart(self.worlds.len() as u16)
            + worlds
            + size_of_val(&self.checksum)
    }

    fn sizeof_players(&self, min: u16) -> usize {
        self.worlds
            .iter()
            .map(|world| sizeof_smart(world.id - min) + size_of_val(&world.players))
            .sum()
    }
}

#[cfg(all(since_530, before_647))]
const fn sizeof_smart(value: u16) -> usize {
    if value < 128 { 1 } else { 2 }
}

#[cfg(all(since_647, before_742))]
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

#[cfg(all(since_530, before_647))]
pub fn game_login_reply(staffmodlevel: u8, pid: u16) -> Vec<u8> {
    let mut buf = Packet::new(12);
    buf.p1(LoginResponse::SuccessNormal as u8);
    buf.p1(staffmodlevel);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p1(0);
    buf.p2(pid);
    buf.p1(0);
    buf.p1(1);
    buf.data
}

#[cfg(since_647)]
pub fn game_login_reply(
    staffmodlevel: u8,
    pid: u16,
    #[cfg(since_742)] display_name: &str,
) -> Vec<u8> {
    #[cfg(before_742)]
    let len = 13;
    #[cfg(since_742)]
    let len = 13 + Packet::sizejstr(display_name);
    let mut buf = Packet::new(2 + len);
    buf.p1(LoginResponse::SuccessNormal as u8);
    buf.p1(len as u8);
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
    #[cfg(since_742)]
    buf.pjstr(display_name, 0);
    buf.data
}

/// The lobby's login reply. Its account details are fixed: on 742 the flags
/// byte says member with an active subscription, so the lobby shows no
/// membership expiry.
#[cfg(since_647)]
pub fn lobby_login_reply(display_name: &str, world: u16) -> Vec<u8> {
    #[cfg(before_742)]
    let len = 59 + Packet::sizejstr(display_name);
    #[cfg(since_742)]
    let len = 68 + Packet::sizejstr(display_name);
    let mut buf = Packet::new(2 + len);
    buf.p1(LoginResponse::SuccessNormal as u8);
    buf.p1(len as u8);
    #[cfg(before_742)]
    {
        buf.p1(2);
        buf.p1(0);
        buf.p1(1);
        buf.p1(0);
        buf.p1(0);
        buf.p4(0);
        buf.p4(0);
        buf.p1(0);
        buf.p4(0);
        buf.p1(3);
        buf.p4(0x7f000001u32 as i32);
        buf.p2(0);
        buf.p2(0);
        buf.p2(0);
        buf.p4(0);
        buf.p1(3);
    }
    #[cfg(since_742)]
    {
        buf.p1(2);
        buf.p1(0);
        buf.p1(0);
        buf.p3(0);
        buf.p1(0);
        buf.p1(0);
        buf.p1(0);
        buf.p8(0);
        buf.p1(0);
        buf.p4(0);
        buf.p1(3);
        buf.p4(0);
        buf.p1(0);
        buf.p4(0);
        buf.p2(0);
        buf.p2(0);
        buf.p2(0);
        buf.p4(0x7f000001u32 as i32);
        buf.p1(3);
    }
    buf.p2(0xD1DF);
    buf.p2(0xD1DF);
    buf.p1(0);
    buf.p1(0);
    buf.pjstr(display_name, 0);
    buf.p1(10);
    buf.p4(0x0046F489);
    buf.p1(0);
    buf.p2(world);
    buf.p1(0);
    buf.pjstr("127.0.0.1", 0);
    buf.data
}

/// A reconnecting 647 client resumes its session from its `PLAYER_INFO`
/// state again: the reply code, then that state behind a two-byte length.
#[cfg(since_647)]
pub fn reconnect_reply(pid: u16, x: u16, z: u16, level: u8, regions: &[u32]) -> Vec<u8> {
    let init = player_info_init(pid, x, z, level, regions);
    let mut buf = Packet::new(1 + 2 + init.pos);
    buf.p1(LoginResponse::Reconnect as u8);
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

    #[cfg(all(since_530, before_647))]
    fn account_block(revision: u16, tail: &[u8]) -> Packet {
        let mut buf = Packet::new(256);
        buf.p1(10);
        buf.p2(31337);
        buf.p2(revision);
        buf.p8(to_userhash("Zezima") as i64);
        buf.p4(99);
        buf.pjstr("Password£", 0);
        buf.p4(98);
        buf.pdata(tail, 0, tail.len());
        buf.rsaencrypt(RsaFrame::Byte, &KEY);
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
        Xtea::encrypt(&mut body.data[..len], &SEED.map(|key| key as u32)).unwrap();
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
            Err(LoginResponse::RuneScapeUpdated)
        ));
        assert!(matches!(
            LoginBlock::decode(login_payload(530, 0), &KEY),
            Err(LoginResponse::InvalidCredentials)
        ));
    }

    #[cfg(all(since_530, before_647))]
    #[test]
    fn account_services_read_the_base37_name_and_password() {
        let name = to_userhash("Zezima") as i64;
        let check = CheckNameRequest::decode(Packet::from(name.to_be_bytes().to_vec())).unwrap();
        assert_eq!(check.name, "zezima");

        let select = WorldSelectRequest::decode(account_block(530, &[]), &KEY).unwrap();
        assert_eq!(select.username, "zezima");
        assert_eq!(select.password, "Password£");
        assert!(matches!(
            WorldSelectRequest::decode(account_block(529, &[]), &KEY),
            Err(LoginResponse::RuneScapeUpdated)
        ));

        let tail = [0, 5, 24, 11, 0, 0, 0, 97, 0x07, 0xC4, 0, 77, 0, 0, 0, 96];
        let create = CreateAccountRequest::decode(account_block(530, &tail), &KEY).unwrap();
        assert_eq!(create.username, "zezima");
        assert_eq!(create.password, "Password£");
        assert_eq!(
            (create.day, create.month, create.year, create.country),
            (24, 11, 1988, 77)
        );
    }

    #[cfg(all(since_530, before_647))]
    #[test]
    fn service_replies_match_the_client_readers() {
        assert_eq!(world_select_reply(0x0102), [101, 1, 2]);
        assert_eq!(
            name_suggestions_reply(&[1, -2]),
            [
                21, 2, 0, 0, 0, 0, 0, 0, 0, 1, 255, 255, 255, 255, 255, 255, 255, 254
            ]
        );
        assert_eq!(
            WorldListRequest::decode(Packet::from(vec![1, 2, 3, 4]))
                .unwrap()
                .checksum,
            0x01020304
        );
        let countries = [Country {
            flag: 0,
            name: "UK",
        }];
        let worlds = [
            World {
                id: 1,
                country: 0,
                flags: 1,
                activity: "",
                host: "127.0.0.1",
                players: 10,
            },
            World {
                id: 200,
                country: 0,
                flags: 0,
                activity: "PvP",
                host: "a",
                players: u16::MAX,
            },
        ];
        let list = WorldList {
            countries: &countries,
            worlds: &worlds,
            checksum: 0x01020304,
        };
        let mut full = vec![
            0, 0, 57, 1, 1, 1, 0, 0, b'U', b'K', 0, 1, 0x80, 200, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0,
        ];
        full.extend_from_slice(b"127.0.0.1\0");
        full.extend_from_slice(&[0x80, 199, 0, 0, 0, 0, 0, 0, b'P', b'v', b'P', 0, 0, b'a', 0]);
        full.extend_from_slice(&[1, 2, 3, 4, 0, 0, 10, 0x80, 199, 255, 255]);
        assert_eq!(list.encode(true), full);
        assert_eq!(
            list.encode(false),
            [0, 0, 9, 1, 0, 0, 0, 10, 0x80, 199, 255, 255]
        );
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

    #[cfg(all(since_647, before_742))]
    #[test]
    fn account_services_share_the_rsa_and_xtea_envelope() {
        for create in [false, true] {
            let mut rsa = Packet::new(256);
            rsa.p1(10);
            for key in SEED {
                rsa.p4(key);
            }
            rsa.pos += 42;
            rsa.rsaencrypt(RsaFrame::Short, &KEY);
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

    #[cfg(all(since_530, before_647))]
    #[test]
    fn game_login_reply_matches_the_client_fields() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, 0x123),
                [2, level, 0, 0, 0, 0, 0, 0, 1, 0x23, 0, 1]
            );
        }
    }

    #[cfg(all(since_647, before_742))]
    #[test]
    fn game_login_reply_matches_the_client_fields() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, 0x123),
                [2, 13, level, 0, 0, 0, 1, 0, 1, 0x23, 1, 0, 0, 0, 1]
            );
        }
    }

    #[cfg(since_742)]
    #[test]
    fn game_login_reply_ends_with_the_display_name() {
        for level in 0..=2 {
            assert_eq!(
                game_login_reply(level, 0x123, "Zezima"),
                [
                    2, 20, level, 0, 0, 0, 1, 0, 1, 0x23, 1, 0, 0, 0, 1, b'Z', b'e', b'z', b'i',
                    b'm', b'a', 0
                ]
            );
        }
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
