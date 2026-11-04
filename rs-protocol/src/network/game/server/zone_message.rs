use rs_io::Packet;

pub mod zone_prot {
    pub const LOC_CUSTOMISE: u8 = 0;
    pub const OBJ_DEL: u8 = 1;
    pub const SOUND_AREA: u8 = 2;
    pub const LOC_ANIM: u8 = 3;
    pub const OBJ_COUNT: u8 = 4;
    pub const LOC_PREFETCH: u8 = 5;
    pub const MAP_ANIM: u8 = 6;
    pub const OBJ_REVEAL: u8 = 7;
    pub const LOC_DEL: u8 = 8;
    pub const TEXT_COORD: u8 = 9;
    pub const SOUND_VORBIS_AREA: u8 = 10;
    pub const OBJ_ADD: u8 = 11;
    pub const MAP_PROJANIM_HALFSQ: u8 = 12;
    pub const LOC_ADD_CHANGE: u8 = 13;
    pub const MAP_PROJANIM: u8 = 14;
    // There is no 15: `ZoneProt.java` ends at MAP_PROJANIM. 225/289 had
    // LOC_MERGE there; 647 has nothing to carry it.
}

#[inline]
pub fn pack_zone_coord(x: u8, z: u8) -> u8 {
    ((x & 7) << 4) | (z & 7)
}

/// The coord byte of `MAP_PROJANIM`, which is not the usual [`pack_zone_coord`]:
/// `AClass17` reads x from bits 3-5 and z from bits 0-2, under a flag in
/// bit 7 the server leaves clear.
#[inline]
pub fn pack_projanim_coord(x: u8, z: u8) -> u8 {
    ((x & 7) << 3) | (z & 7)
}

#[inline]
pub fn pack_shape_angle(shape: u8, angle: u8) -> u8 {
    ((shape) << 2) | (angle & 3)
}

#[derive(Debug, Clone)]
pub struct ZoneLocAddChange {
    pub coord: u8,
    pub shape_rot: u8,
    pub loc_id: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneLocDel {
    pub coord: u8,
    pub shape_rot: u8,
}

#[derive(Debug, Clone)]
pub struct ZoneLocAnim {
    pub coord: u8,
    pub shape_rot: u8,
    pub seq_id: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneObjAdd {
    pub coord: u8,
    pub obj_id: u16,
    pub count: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneObjDel {
    pub coord: u8,
    pub obj_id: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneObjCount {
    pub coord: u8,
    pub obj_id: u16,
    pub old_count: u16,
    pub new_count: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneObjReveal {
    pub coord: u8,
    pub obj_id: u16,
    pub count: u16,
    pub receiver: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneMapAnim {
    pub coord: u8,
    pub spotanim: u16,
    pub height: u8,
    pub delay: u16,
}

#[derive(Debug, Clone)]
pub struct ZoneMapProjanim {
    pub coord: u8,
    pub dst_dx: i8,
    pub dst_dz: i8,
    pub target_uid: i16,
    pub spotanim_id: u16,
    pub src_height: u8,
    pub dst_height: u8,
    pub start_delay: u16,
    /// When the projectile lands, as a delay from now like `start_delay` --
    /// the client adds both to its current cycle (`AClass17`).
    pub end_delay: u16,
    pub angle: u8,
    pub distance_offset: u16,
}

#[derive(Debug, Clone)]
pub enum ZoneMessage {
    LocAddChange(ZoneLocAddChange),
    LocDel(ZoneLocDel),
    LocAnim(ZoneLocAnim),
    ObjAdd(ZoneObjAdd),
    ObjDel(ZoneObjDel),
    ObjCount(ZoneObjCount),
    ObjReveal(ZoneObjReveal),
    MapAnim(ZoneMapAnim),
    MapProjanim(ZoneMapProjanim),
}

impl ZoneMessage {
    pub fn encode_zone(&self, buf: &mut Packet) {
        match self {
            Self::LocAddChange(m) => {
                buf.p1(zone_prot::LOC_ADD_CHANGE);
                buf.p1_alt2(m.coord);
                buf.p1_alt1(m.shape_rot);
                buf.p2_alt1(m.loc_id);
            }
            Self::LocDel(m) => {
                buf.p1(zone_prot::LOC_DEL);
                buf.p1(m.shape_rot);
                buf.p1_alt2(m.coord);
            }
            Self::LocAnim(m) => {
                buf.p1(zone_prot::LOC_ANIM);
                buf.p1_alt1(m.coord);
                buf.p1(m.shape_rot);
                buf.ip2(m.seq_id);
            }
            Self::ObjAdd(m) => {
                buf.p1(zone_prot::OBJ_ADD);
                buf.ip2_alt1(m.count);
                buf.p1(m.coord);
                buf.ip2_alt1(m.obj_id);
            }
            Self::ObjDel(m) => {
                buf.p1(zone_prot::OBJ_DEL);
                buf.p1_alt1(m.coord);
                buf.ip2_alt1(m.obj_id);
            }
            Self::ObjCount(m) => {
                buf.p1(zone_prot::OBJ_COUNT);
                buf.p1(m.coord);
                buf.p2(m.obj_id);
                buf.p2(m.old_count);
                buf.p2(m.new_count);
            }
            Self::ObjReveal(m) => {
                buf.p1(zone_prot::OBJ_REVEAL);
                buf.ip2_alt1(m.obj_id);
                buf.p2(m.receiver);
                buf.p1_alt1(m.coord);
                buf.ip2_alt1(m.count);
            }
            Self::MapAnim(m) => {
                buf.p1(zone_prot::MAP_ANIM);
                buf.p1(m.coord);
                buf.p2(m.spotanim);
                buf.p1(m.height);
                buf.p2(m.delay);
                buf.p1(0);
            }
            Self::MapProjanim(m) => {
                buf.p1(zone_prot::MAP_PROJANIM);
                buf.p1(m.coord);
                buf.p1(m.dst_dx as u8);
                buf.p1(m.dst_dz as u8);
                buf.p2(m.target_uid as u16);
                buf.p2(m.spotanim_id);
                buf.p1(m.src_height);
                buf.p1(m.dst_height);
                buf.p2(m.start_delay);
                buf.p2(m.end_delay);
                buf.p1(m.angle);
                buf.p2(m.distance_offset);
            }
        }
    }

    pub fn sizeof_zone(&self) -> usize {
        1 + match self {
            Self::LocAddChange(_) => 4,
            Self::LocDel(_) => 2,
            Self::LocAnim(_) => 4,
            Self::ObjAdd(_) => 5,
            Self::ObjDel(_) => 3,
            Self::ObjCount(_) => 7,
            Self::ObjReveal(_) => 7,
            Self::MapAnim(_) => 7,
            Self::MapProjanim(_) => 16,
        }
    }
}
