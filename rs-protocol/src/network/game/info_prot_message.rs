use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub trait InfoMessage {
    fn encode(&self, buf: &mut Packet);
    fn test(&self) -> usize;
}

// ---- players

pub struct PlayerInfoIdk {
    bytes: Box<[u8]>,
}

impl PlayerInfoIdk {
    #[inline]
    pub const fn new(bytes: Box<[u8]>) -> PlayerInfoIdk {
        PlayerInfoIdk { bytes }
    }
}

impl InfoMessage for PlayerInfoIdk {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.bytes.len() as u8);
        buf.pdata(&self.bytes, 0, self.bytes.len());
    }

    #[inline]
    fn test(&self) -> usize {
        1 + self.bytes.len()
    }
}

// ----

pub struct PlayerInfoFaceEntity {
    entity: u16,
}

impl PlayerInfoFaceEntity {
    #[inline]
    pub const fn new(entity: u16) -> PlayerInfoFaceEntity {
        PlayerInfoFaceEntity { entity }
    }
}

impl InfoMessage for PlayerInfoFaceEntity {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.entity);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.entity)
    }
}

// ----

pub struct PlayerInfoFaceCoord {
    x: u16,
    z: u16,
}

impl PlayerInfoFaceCoord {
    #[inline]
    pub const fn new(x: u16, z: u16) -> PlayerInfoFaceCoord {
        PlayerInfoFaceCoord { x, z }
    }
}

impl InfoMessage for PlayerInfoFaceCoord {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.x);
        buf.p2(self.z);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

// ----

pub struct PlayerInfoAnim {
    anim: u16,
    delay: u8,
}

impl PlayerInfoAnim {
    #[inline]
    pub const fn new(anim: u16, delay: u8) -> PlayerInfoAnim {
        PlayerInfoAnim { anim, delay }
    }
}

impl InfoMessage for PlayerInfoAnim {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.anim);
        buf.p1(self.delay);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.anim) + size_of_val(&self.delay)
    }
}

// ----

pub struct PlayerInfoSay {
    say: Box<str>,
}

impl PlayerInfoSay {
    #[inline]
    pub const fn new(say: Box<str>) -> PlayerInfoSay {
        PlayerInfoSay { say }
    }
}

impl InfoMessage for PlayerInfoSay {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.say, STRING_TERMINATOR);
    }

    #[inline]
    fn test(&self) -> usize {
        1 + self.say.len()
    }
}

// ----

pub struct PlayerInfoDamage {
    damage: u8,
    damage_type: u8,
    current_hitpoints: u8,
    base_hitpoints: u8,
}

impl PlayerInfoDamage {
    #[inline]
    pub const fn new(
        damage: u8,
        damage_type: u8,
        current_hitpoints: u8,
        base_hitpoints: u8,
    ) -> PlayerInfoDamage {
        PlayerInfoDamage {
            damage,
            damage_type,
            current_hitpoints,
            base_hitpoints,
        }
    }
}

impl InfoMessage for PlayerInfoDamage {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.damage);
        buf.p1(self.damage_type);
        buf.p1(self.current_hitpoints);
        buf.p1(self.base_hitpoints);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.damage)
            + size_of_val(&self.damage_type)
            + size_of_val(&self.current_hitpoints)
            + size_of_val(&self.base_hitpoints)
    }
}

// ----

pub struct PlayerInfoChat {
    bytes: Box<[u8]>,
    color: u8,
    effect: u8,
    ignored: u8,
}

impl PlayerInfoChat {
    #[inline]
    pub const fn new(bytes: Box<[u8]>, color: u8, effect: u8, ignored: u8) -> PlayerInfoChat {
        PlayerInfoChat {
            bytes,
            color,
            effect,
            ignored,
        }
    }
}

impl InfoMessage for PlayerInfoChat {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.color);
        buf.p1(self.effect);
        buf.p1(self.ignored);
        buf.p1(self.bytes.len() as u8);
        buf.pdata(&self.bytes, 0, self.bytes.len());
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.color)
            + size_of_val(&self.effect)
            + size_of_val(&self.ignored)
            + 1
            + self.bytes.len()
    }
}

// ----

pub struct PlayerInfoSpotanim {
    graphic_id: u16,
    graphic_height: u16,
    graphic_delay: u16,
}

impl PlayerInfoSpotanim {
    #[inline]
    pub const fn new(
        graphic_id: u16,
        graphic_height: u16,
        graphic_delay: u16,
    ) -> PlayerInfoSpotanim {
        PlayerInfoSpotanim {
            graphic_id,
            graphic_height,
            graphic_delay,
        }
    }
}

impl InfoMessage for PlayerInfoSpotanim {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.graphic_id);
        buf.p4(((self.graphic_height as i32) << 16) | self.graphic_delay as i32);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.graphic_id)
            + size_of_val(&self.graphic_height)
            + size_of_val(&self.graphic_delay)
    }
}

// ----

pub struct PlayerInfoExactMove {
    start_x: u8,
    start_z: u8,
    end_x: u8,
    end_z: u8,
    begin: u16,
    finish: u16,
    dir: u8,
}

impl PlayerInfoExactMove {
    #[inline]
    pub const fn new(
        start_x: u8,
        start_z: u8,
        end_x: u8,
        end_z: u8,
        begin: u16,
        finish: u16,
        dir: u8,
    ) -> PlayerInfoExactMove {
        PlayerInfoExactMove {
            start_x,
            start_z,
            end_x,
            end_z,
            begin,
            finish,
            dir,
        }
    }
}

impl InfoMessage for PlayerInfoExactMove {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.start_x);
        buf.p1(self.start_z);
        buf.p1(self.end_x);
        buf.p1(self.end_z);
        buf.p2(self.begin);
        buf.p2(self.finish);
        buf.p1(self.dir);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.start_x)
            + size_of_val(&self.start_z)
            + size_of_val(&self.end_x)
            + size_of_val(&self.end_z)
            + size_of_val(&self.begin)
            + size_of_val(&self.finish)
            + size_of_val(&self.dir)
    }
}

// ---- npcs

pub struct NpcInfoFaceEntity {
    entity: u16,
}

impl NpcInfoFaceEntity {
    #[inline]
    pub const fn new(entity: u16) -> NpcInfoFaceEntity {
        NpcInfoFaceEntity { entity }
    }
}

impl InfoMessage for NpcInfoFaceEntity {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.entity);
    }

    #[inline]
    fn test(&self) -> usize {
        2
    }
}

// ----

pub struct NpcInfoFaceCoord {
    x: u16,
    z: u16,
}

impl NpcInfoFaceCoord {
    #[inline]
    pub const fn new(x: u16, z: u16) -> NpcInfoFaceCoord {
        NpcInfoFaceCoord { x, z }
    }
}

impl InfoMessage for NpcInfoFaceCoord {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.x);
        buf.p2(self.z);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

// ----

pub struct NpcInfoAnim {
    anim: u16,
    delay: u8,
}

impl NpcInfoAnim {
    #[inline]
    pub const fn new(anim: u16, delay: u8) -> NpcInfoAnim {
        NpcInfoAnim { anim, delay }
    }
}

impl InfoMessage for NpcInfoAnim {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.anim);
        buf.p1(self.delay);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.anim) + size_of_val(&self.delay)
    }
}

// ----

pub struct NpcInfoSay {
    say: Box<str>,
}

impl NpcInfoSay {
    #[inline]
    pub const fn new(say: Box<str>) -> NpcInfoSay {
        NpcInfoSay { say }
    }
}

impl InfoMessage for NpcInfoSay {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.say, STRING_TERMINATOR);
    }

    #[inline]
    fn test(&self) -> usize {
        1 + self.say.len()
    }
}

// ----

pub struct NpcInfoDamage {
    damage: u8,
    damage_type: u8,
    current_hitpoints: u8,
    base_hitpoints: u8,
}

impl NpcInfoDamage {
    #[inline]
    pub const fn new(
        damage: u8,
        damage_type: u8,
        current_hitpoints: u8,
        base_hitpoints: u8,
    ) -> NpcInfoDamage {
        NpcInfoDamage {
            damage,
            damage_type,
            current_hitpoints,
            base_hitpoints,
        }
    }
}

impl InfoMessage for NpcInfoDamage {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.damage);
        buf.p1(self.damage_type);
        buf.p1(self.current_hitpoints);
        buf.p1(self.base_hitpoints);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.damage)
            + size_of_val(&self.damage_type)
            + size_of_val(&self.current_hitpoints)
            + size_of_val(&self.base_hitpoints)
    }
}

// ----

pub struct NpcInfoChangeType {
    change_type: u16,
}

impl NpcInfoChangeType {
    #[inline]
    pub const fn new(change_type: u16) -> NpcInfoChangeType {
        NpcInfoChangeType { change_type }
    }
}

impl InfoMessage for NpcInfoChangeType {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.change_type);
    }

    fn test(&self) -> usize {
        size_of_val(&self.change_type)
    }
}

// ----

pub struct NpcInfoSpotanim {
    graphic_id: u16,
    graphic_height: u16,
    graphic_delay: u16,
}

impl NpcInfoSpotanim {
    #[inline]
    pub const fn new(graphic_id: u16, graphic_height: u16, graphic_delay: u16) -> NpcInfoSpotanim {
        NpcInfoSpotanim {
            graphic_id,
            graphic_height,
            graphic_delay,
        }
    }
}

impl InfoMessage for NpcInfoSpotanim {
    #[inline]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.graphic_id);
        buf.p4(((self.graphic_height as i32) << 16) | self.graphic_delay as i32);
    }

    #[inline]
    fn test(&self) -> usize {
        size_of_val(&self.graphic_id)
            + size_of_val(&self.graphic_height)
            + size_of_val(&self.graphic_delay)
    }
}
