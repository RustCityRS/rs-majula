use super::decode::{read_array, read_big_id, read_id, read_pairs};
#[cfg(since_530)]
use super::decode::{read_optional_id, read_transforms};
use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
#[cfg(since_530)]
use crate::types::MultiVar;
use crate::types::{BlockWalk, MoveRestrict, NpcMode};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;
use rustc_hash::FxHashMap;

pub type NpcTypeProvider = TypeProvider<NpcType>;

#[cfg_attr(since_530, derive(Debug, Clone))]
pub struct NpcPatrol {
    pub coord: i32,
    pub delay: u8,
}

pub struct NpcType {
    pub id: TypeId,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub category: Option<TypeId>,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub regenrate: u16,
    pub moverestrict: MoveRestrict,
    pub blockwalk: BlockWalk,
    pub defaultmode: NpcMode,
    pub huntmode: Option<TypeId>,
    pub wanderrange: u16,
    pub maxrange: u16,
    pub huntrange: u8,
    pub attackrange: u8,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub patrol: Option<Box<[NpcPatrol]>>,
    pub members: bool,
    pub givechase: bool,
    pub attack: u16,
    pub defence: u16,
    pub strength: u16,
    pub hitpoints: u16,
    pub ranged: u16,
    pub magic: u16,
    pub vislevel: Option<u16>,
    #[cfg(since_289)]
    pub turnspeed: u16,
    #[cfg(since_530)]
    pub multivar: Option<MultiVar>,
    #[cfg(since_530)]
    pub multinpc: Option<Box<[TypeId]>>,
    pub params: Option<Box<FxHashMap<TypeId, ParamValue>>>,
    pub debugname: Box<str>,
}

impl NpcType {
    /// The npc this one shows as when its var is `var_value`: the entry at
    /// that index, else the default (the list's last), as
    /// [`LocType::multi_loc`](super::loc::LocType::multi_loc) picks. `None`
    /// when it shows as nothing; its own id when it is no multi-npc.
    #[cfg(since_530)]
    pub fn multi_npc(&self, var_value: i32) -> Option<TypeId> {
        let Some(multi) = self.multinpc.as_deref() else {
            return Some(self.id);
        };
        multi
            .get(var_value as usize)
            .copied()
            .filter(|id| !id.is_null())
            .or_else(|| multi.last().copied())
            .and_then(TypeId::non_null)
    }
}

impl From<NpcTypeRaw> for NpcType {
    fn from(raw: NpcTypeRaw) -> Self {
        NpcType {
            id: raw.id,
            name: raw.name,
            desc: raw.desc,
            size: raw.size,
            category: raw.category,
            timer: raw.timer,
            respawnrate: raw.respawnrate,
            regenrate: raw.regenrate,
            moverestrict: raw.moverestrict,
            blockwalk: raw.blockwalk,
            defaultmode: raw.defaultmode,
            huntmode: raw.huntmode,
            wanderrange: raw.wanderrange,
            maxrange: raw
                .maxrange
                .unwrap_or(raw.wanderrange + 2)
                .max(raw.wanderrange),
            huntrange: raw.huntrange,
            attackrange: raw.attackrange,
            op: raw.op,
            patrol: raw.patrol,
            members: raw.members,
            givechase: raw.givechase,
            attack: raw.attack,
            defence: raw.defence,
            strength: raw.strength,
            hitpoints: raw.hitpoints,
            ranged: raw.ranged,
            magic: raw.magic,
            vislevel: raw.vislevel,
            #[cfg(since_289)]
            turnspeed: raw.turnspeed,
            #[cfg(since_530)]
            multivar: raw
                .transform_varpbit
                .map(MultiVar::VarpBit)
                .or_else(|| raw.transform_varp.map(MultiVar::Varp)),
            #[cfg(since_530)]
            multinpc: raw.multinpc,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

#[cfg_attr(since_530, derive(Debug, Clone))]
pub struct NpcTypeRaw {
    pub id: TypeId,
    pub models: Option<Box<[TypeId]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub readyanim: Option<TypeId>,
    pub walkanim: Option<TypeId>,
    #[cfg(before_254)]
    pub hasalpha: bool,
    pub walkanim_b: Option<TypeId>,
    pub walkanim_r: Option<TypeId>,
    pub walkanim_l: Option<TypeId>,
    pub category: Option<TypeId>,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub regenrate: u16,
    pub moverestrict: MoveRestrict,
    pub blockwalk: BlockWalk,
    pub defaultmode: NpcMode,
    pub huntmode: Option<TypeId>,
    pub wanderrange: u16,
    pub maxrange: Option<u16>,
    pub huntrange: u8,
    pub attackrange: u8,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub patrol: Option<Box<[NpcPatrol]>>,
    pub members: bool,
    pub givechase: bool,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub colour_palette: Option<Box<[u8]>>,
    #[cfg(since_742)]
    pub recolindices: Option<u16>,
    #[cfg(since_742)]
    pub retexindices: Option<u16>,
    pub heads: Option<Box<[TypeId]>>,
    pub attack: u16,
    pub defence: u16,
    pub strength: u16,
    pub hitpoints: u16,
    pub ranged: u16,
    pub magic: u16,
    pub resizex: Option<u16>,
    pub resizey: Option<u16>,
    pub resizez: Option<u16>,
    pub minimap: bool,
    pub vislevel: Option<u16>,
    pub resizeh: u16,
    pub resizev: u16,
    #[cfg(since_244)]
    pub alwaysontop: bool,
    #[cfg(since_244)]
    pub ambient: i8,
    #[cfg(since_244)]
    pub contrast: i8,
    #[cfg(since_244)]
    pub headicon: Option<u16>,
    #[cfg(since_289)]
    pub turnspeed: u16,
    #[cfg(since_530)]
    pub transform_varpbit: Option<TypeId>,
    #[cfg(since_530)]
    pub transform_varp: Option<TypeId>,
    #[cfg(since_530)]
    pub multinpc: Option<Box<[TypeId]>>,
    #[cfg(since_530)]
    pub clickable: bool,
    #[cfg(since_530)]
    pub walk_smoothing: bool,
    #[cfg(since_530)]
    pub animateidle: bool,
    #[cfg(since_530)]
    pub primary_shadow_colour: i16,
    #[cfg(since_530)]
    pub secondary_shadow_colour: i16,
    #[cfg(since_530)]
    pub primary_shadow_modifier: i8,
    #[cfg(since_530)]
    pub secondary_shadow_modifier: i8,
    #[cfg(all(since_530, before_647))]
    pub unknown_115: Option<(u8, u8)>,
    #[cfg(since_530)]
    pub walkmask: i8,
    #[cfg(since_530)]
    #[allow(clippy::type_complexity)]
    pub translations: Option<Box<[(u8, [i8; 3])]>>,
    #[cfg(since_530)]
    pub hitbar_sprite: Option<TypeId>,
    #[cfg(since_530)]
    pub overlay_height: Option<u16>,
    #[cfg(since_530)]
    pub respawn_direction: i8,
    #[cfg(all(since_530, before_647))]
    pub unknown_126: Option<u16>,
    #[cfg(since_530)]
    pub bas: Option<TypeId>,
    #[cfg(since_530)]
    pub move_speed: u8,
    #[cfg(since_530)]
    pub background_sound_ready: Option<TypeId>,
    #[cfg(since_530)]
    pub background_sound_crawl: Option<TypeId>,
    #[cfg(since_530)]
    pub background_sound_walk: Option<TypeId>,
    #[cfg(since_530)]
    pub background_sound_run: Option<TypeId>,
    #[cfg(since_530)]
    pub background_sound_range: u8,
    #[cfg(all(since_530, before_742))]
    pub primary_cursor_op: Option<u8>,
    #[cfg(all(since_530, before_742))]
    pub primary_cursor: Option<TypeId>,
    #[cfg(all(since_530, before_742))]
    pub secondary_cursor_op: Option<u8>,
    #[cfg(all(since_530, before_742))]
    pub secondary_cursor: Option<TypeId>,
    #[cfg(since_530)]
    pub attack_cursor: Option<TypeId>,
    #[cfg(since_647)]
    pub icon: Option<TypeId>,
    #[cfg(since_647)]
    pub sprite_id: Option<TypeId>,
    #[cfg(since_647)]
    pub background_sound_volume: u8,
    #[cfg(since_647)]
    pub familiar: bool,
    #[cfg(since_647)]
    pub map_element: Option<TypeId>,
    #[cfg(since_647)]
    pub always_on_bottom: bool,
    #[cfg(since_647)]
    pub members_op: [Option<Box<str>>; 5],
    #[cfg(since_647)]
    pub hue: i8,
    #[cfg(since_647)]
    pub saturation: i8,
    #[cfg(since_647)]
    pub lightness: i8,
    #[cfg(since_647)]
    pub opacity: i8,
    #[cfg(since_647)]
    pub reprioritise_attack_op: Option<i8>,
    #[cfg(since_647)]
    pub quests: Option<Box<[i16]>>,
    #[cfg(since_647)]
    pub vorbis: bool,
    #[cfg(since_647)]
    pub rough_bounding: Option<u8>,
    #[cfg(since_647)]
    pub background_sound_rate_min: i32,
    #[cfg(since_647)]
    pub background_sound_rate_max: i32,
    #[cfg(since_647)]
    pub pick_size_shift: u8,
    #[cfg(since_647)]
    pub background_sound_dropoff_range: u8,
    #[cfg(since_742)]
    pub antimacro: bool,
    #[cfg(since_742)]
    pub cursor: [Option<TypeId>; 6],
    pub params: Option<Box<FxHashMap<TypeId, ParamValue>>>,
    debugname: Box<str>,
}

impl CacheType for NpcTypeRaw {
    type Context = ();

    fn new(id: TypeId) -> Self {
        NpcTypeRaw {
            id,
            models: None,
            name: None,
            desc: None,
            size: 1,
            readyanim: None,
            walkanim: None,
            #[cfg(before_254)]
            hasalpha: false,
            walkanim_b: None,
            walkanim_r: None,
            walkanim_l: None,
            category: None,
            timer: None,
            respawnrate: 100,
            regenrate: 100,
            moverestrict: MoveRestrict::Normal,
            blockwalk: BlockWalk::Npc,
            defaultmode: NpcMode::Wander,
            huntmode: None,
            wanderrange: 5,
            maxrange: None,
            huntrange: 0,
            attackrange: 0,
            op: None,
            patrol: None,
            members: false,
            givechase: true,
            recol_s: None,
            recol_d: None,
            #[cfg(since_530)]
            retex_s: None,
            #[cfg(since_530)]
            retex_d: None,
            #[cfg(since_530)]
            colour_palette: None,
            #[cfg(since_742)]
            recolindices: None,
            #[cfg(since_742)]
            retexindices: None,
            heads: None,
            attack: 1,
            defence: 1,
            strength: 1,
            hitpoints: 1,
            ranged: 1,
            magic: 1,
            resizex: None,
            resizey: None,
            resizez: None,
            minimap: true,
            vislevel: None,
            resizeh: 128,
            resizev: 128,
            #[cfg(since_244)]
            alwaysontop: false,
            #[cfg(since_244)]
            ambient: 0,
            #[cfg(since_244)]
            contrast: 0,
            #[cfg(since_244)]
            headicon: None,
            #[cfg(since_289)]
            turnspeed: if cfg!(since_530) { 32 } else { 0 },
            #[cfg(since_530)]
            transform_varpbit: None,
            #[cfg(since_530)]
            transform_varp: None,
            #[cfg(since_530)]
            multinpc: None,
            #[cfg(since_530)]
            clickable: true,
            #[cfg(since_530)]
            walk_smoothing: true,
            #[cfg(since_530)]
            animateidle: true,
            #[cfg(since_530)]
            primary_shadow_colour: 0,
            #[cfg(since_530)]
            secondary_shadow_colour: 0,
            #[cfg(since_530)]
            primary_shadow_modifier: -96,
            #[cfg(since_530)]
            secondary_shadow_modifier: -16,
            #[cfg(all(since_530, before_647))]
            unknown_115: None,
            #[cfg(since_530)]
            walkmask: 0,
            #[cfg(since_530)]
            translations: None,
            #[cfg(since_530)]
            hitbar_sprite: None,
            #[cfg(since_530)]
            overlay_height: None,
            #[cfg(since_530)]
            respawn_direction: 4,
            #[cfg(all(since_530, before_647))]
            unknown_126: None,
            #[cfg(since_530)]
            bas: None,
            #[cfg(since_530)]
            move_speed: 0,
            #[cfg(since_530)]
            background_sound_ready: None,
            #[cfg(since_530)]
            background_sound_crawl: None,
            #[cfg(since_530)]
            background_sound_walk: None,
            #[cfg(since_530)]
            background_sound_run: None,
            #[cfg(since_530)]
            background_sound_range: 0,
            #[cfg(all(since_530, before_742))]
            primary_cursor_op: None,
            #[cfg(all(since_530, before_742))]
            primary_cursor: None,
            #[cfg(all(since_530, before_742))]
            secondary_cursor_op: None,
            #[cfg(all(since_530, before_742))]
            secondary_cursor: None,
            #[cfg(since_530)]
            attack_cursor: None,
            #[cfg(since_647)]
            icon: None,
            #[cfg(since_647)]
            sprite_id: None,
            #[cfg(since_647)]
            background_sound_volume: 255,
            #[cfg(since_647)]
            familiar: false,
            #[cfg(since_647)]
            map_element: None,
            #[cfg(since_647)]
            always_on_bottom: false,
            #[cfg(since_647)]
            members_op: [const { None }; 5],
            #[cfg(since_647)]
            hue: 0,
            #[cfg(since_647)]
            saturation: 0,
            #[cfg(since_647)]
            lightness: 0,
            #[cfg(since_647)]
            opacity: 0,
            #[cfg(since_647)]
            reprioritise_attack_op: None,
            #[cfg(since_647)]
            quests: None,
            #[cfg(since_647)]
            vorbis: false,
            #[cfg(since_647)]
            rough_bounding: None,
            #[cfg(since_647)]
            background_sound_rate_min: 256,
            #[cfg(since_647)]
            background_sound_rate_max: 256,
            #[cfg(since_647)]
            pick_size_shift: 0,
            #[cfg(since_647)]
            background_sound_dropoff_range: 0,
            #[cfg(since_742)]
            antimacro: true,
            #[cfg(since_742)]
            cursor: [None; 6],
            params: None,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.models = Some(read_array(buf, read_big_id)),
            2 => self.name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
            3 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
            12 => self.size = buf.g1(),
            #[cfg(before_530)]
            13 => self.readyanim = Some(read_id(buf)),
            #[cfg(before_530)]
            14 => self.walkanim = Some(read_id(buf)),
            #[cfg(before_254)]
            16 => self.hasalpha = true,
            #[cfg(before_530)]
            17 => {
                self.walkanim = Some(read_id(buf));
                self.walkanim_b = Some(read_id(buf));
                self.walkanim_r = Some(read_id(buf));
                self.walkanim_l = Some(read_id(buf));
            }
            18 => self.category = Some(read_id(buf)),
            19 => self.timer = Some(buf.g2()),
            20 => self.respawnrate = buf.g2(),
            21 => self.regenrate = buf.g2(),
            22 => self.moverestrict = MoveRestrict::try_from(buf.g1()).unwrap(),
            23 => self.blockwalk = BlockWalk::try_from(buf.g1()).unwrap(),
            24 => self.defaultmode = NpcMode::try_from(buf.g1()).unwrap(),
            25 => self.huntmode = Some(TypeId::new(buf.g1().into())),
            26 => self.wanderrange = buf.g2(),
            27 => self.maxrange = Some(buf.g2()),
            28 => self.huntrange = buf.g1(),
            29 => self.attackrange = buf.g1(),
            30..=34 => {
                let op = buf.gjstr(STRING_TERMINATOR);
                self.op
                    .get_or_insert_with(|| vec![None; 5].into_boxed_slice())[code as usize - 30] =
                    Some(op.into_boxed_str());
            }
            35 => self.decode_patrol(buf),
            36 => self.members = true,
            37 => self.givechase = false,
            40 => read_pairs(buf, &mut self.recol_s, &mut self.recol_d),
            #[cfg(since_530)]
            41 => read_pairs(buf, &mut self.retex_s, &mut self.retex_d),
            #[cfg(since_530)]
            42 => self.colour_palette = Some(read_array(buf, Packet::g1)),
            #[cfg(since_742)]
            44 => self.recolindices = Some(buf.g2()),
            #[cfg(since_742)]
            45 => self.retexindices = Some(buf.g2()),
            60 => self.heads = Some(read_array(buf, read_big_id)),
            74 => self.attack = buf.g2(),
            75 => self.defence = buf.g2(),
            76 => self.strength = buf.g2(),
            77 => self.hitpoints = buf.g2(),
            78 => self.ranged = buf.g2(),
            79 => self.magic = buf.g2(),
            #[cfg(before_530)]
            90 => self.resizex = Some(buf.g2()),
            #[cfg(before_530)]
            91 => self.resizey = Some(buf.g2()),
            #[cfg(before_530)]
            92 => self.resizez = Some(buf.g2()),
            93 => self.minimap = false,
            95 => self.vislevel = Some(buf.g2()),
            97 => self.resizeh = buf.g2(),
            98 => self.resizev = buf.g2(),
            #[cfg(since_244)]
            99 => self.alwaysontop = true,
            #[cfg(since_244)]
            100 => self.ambient = buf.g1s(),
            #[cfg(since_244)]
            101 => self.contrast = buf.g1s(),
            #[cfg(since_244)]
            102 => self.headicon = Some(buf.g2()),
            #[cfg(since_289)]
            103 => self.turnspeed = buf.g2(),
            #[cfg(since_530)]
            106 => self.decode_multinpc(buf, false),
            #[cfg(since_530)]
            107 => self.clickable = false,
            #[cfg(since_530)]
            109 => self.walk_smoothing = false,
            #[cfg(since_530)]
            111 => self.animateidle = false,
            #[cfg(since_530)]
            113 => {
                self.primary_shadow_colour = buf.g2s();
                self.secondary_shadow_colour = buf.g2s();
            }
            #[cfg(since_530)]
            114 => {
                self.primary_shadow_modifier = buf.g1s();
                self.secondary_shadow_modifier = buf.g1s();
            }
            #[cfg(all(since_530, before_647))]
            115 => self.unknown_115 = Some((buf.g1(), buf.g1())),
            #[cfg(since_530)]
            118 => self.decode_multinpc(buf, true),
            #[cfg(since_530)]
            119 => self.walkmask = buf.g1s(),
            #[cfg(since_530)]
            121 => {
                self.translations = Some(read_array(buf, |buf| {
                    (buf.g1(), [buf.g1s(), buf.g1s(), buf.g1s()])
                }));
            }
            #[cfg(since_530)]
            122 => self.hitbar_sprite = Some(read_id(buf)),
            #[cfg(since_530)]
            123 => self.overlay_height = Some(buf.g2()),
            #[cfg(since_530)]
            125 => self.respawn_direction = buf.g1s(),
            #[cfg(all(since_530, before_647))]
            126 => self.unknown_126 = Some(buf.g2()),
            #[cfg(since_530)]
            127 => self.bas = Some(read_id(buf)),
            #[cfg(since_530)]
            128 => self.move_speed = buf.g1(),
            #[cfg(since_530)]
            134 => {
                self.background_sound_ready = read_optional_id(buf);
                self.background_sound_crawl = read_optional_id(buf);
                self.background_sound_walk = read_optional_id(buf);
                self.background_sound_run = read_optional_id(buf);
                self.background_sound_range = buf.g1();
            }
            #[cfg(all(since_530, before_742))]
            135 => {
                self.primary_cursor_op = Some(buf.g1());
                self.primary_cursor = Some(read_id(buf));
            }
            #[cfg(all(since_530, before_742))]
            136 => {
                self.secondary_cursor_op = Some(buf.g1());
                self.secondary_cursor = Some(read_id(buf));
            }
            #[cfg(since_530)]
            137 => self.attack_cursor = Some(read_id(buf)),
            #[cfg(since_647)]
            138 => self.icon = Some(read_big_id(buf)),
            #[cfg(since_647)]
            139 => self.sprite_id = Some(read_id(buf)),
            #[cfg(since_647)]
            140 => self.background_sound_volume = buf.g1(),
            #[cfg(since_647)]
            141 => self.familiar = true,
            #[cfg(since_647)]
            142 => self.map_element = Some(read_id(buf)),
            #[cfg(since_647)]
            143 => self.always_on_bottom = true,
            #[cfg(since_647)]
            150..=154 => {
                let slot = (code - 150) as usize;
                self.members_op[slot] = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str());
            }
            #[cfg(since_647)]
            155 => {
                self.hue = buf.g1s();
                self.saturation = buf.g1s();
                self.lightness = buf.g1s();
                self.opacity = buf.g1s();
            }
            #[cfg(since_647)]
            158 => self.reprioritise_attack_op = Some(1),
            #[cfg(since_647)]
            159 => self.reprioritise_attack_op = Some(0),
            #[cfg(since_647)]
            160 => self.quests = Some(read_array(buf, Packet::g2s)),
            #[cfg(since_647)]
            162 => self.vorbis = true,
            #[cfg(since_647)]
            163 => self.rough_bounding = Some(buf.g1()),
            #[cfg(since_647)]
            164 => {
                self.background_sound_rate_min = buf.g2() as i32;
                self.background_sound_rate_max = buf.g2() as i32;
            }
            #[cfg(since_647)]
            165 => self.pick_size_shift = buf.g1(),
            #[cfg(since_647)]
            168 => self.background_sound_dropoff_range = buf.g1(),
            #[cfg(since_742)]
            169 => self.antimacro = false,
            #[cfg(since_742)]
            170..=175 => self.cursor[code as usize - 170] = Some(read_id(buf)),
            249 => {
                ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                );
            }
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("npc", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

impl NpcTypeRaw {
    fn decode_patrol(&mut self, buf: &mut Packet) {
        self.patrol = Some(read_array(buf, |buf| NpcPatrol {
            coord: buf.g4s(),
            delay: buf.g1(),
        }));
    }

    #[cfg(since_530)]
    fn decode_multinpc(&mut self, buf: &mut Packet, has_default: bool) {
        self.transform_varpbit = read_optional_id(buf);
        self.transform_varp = read_optional_id(buf);
        let default = if has_default {
            read_id(buf)
        } else {
            TypeId::NULL
        };
        self.multinpc = Some(read_transforms(buf, default, read_id));
    }
}
