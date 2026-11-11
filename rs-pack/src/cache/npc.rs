use super::decode::{read_array, read_pairs};
#[cfg(since_647)]
use super::decode::{read_id, read_optional_id, read_transforms};
use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
#[cfg(since_647)]
use crate::types::MultiVar;
use crate::types::{BlockWalk, MoveRestrict, NpcMode};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type NpcTypeProvider = TypeProvider<NpcType>;

#[cfg_attr(since_647, derive(Debug, Clone))]
pub struct NpcPatrol {
    pub coord: i32,
    pub delay: u8,
}

pub struct NpcType {
    pub id: u16,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub category: Option<u16>,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub regenrate: u16,
    pub moverestrict: MoveRestrict,
    pub blockwalk: BlockWalk,
    pub defaultmode: NpcMode,
    pub huntmode: Option<u16>,
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
    #[cfg(since_647)]
    pub multivar: Option<MultiVar>,
    #[cfg(since_647)]
    pub multinpc: Option<Box<[i32]>>,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    pub debugname: Box<str>,
}

impl NpcType {
    /// The npc this one shows as when its var is `var_value`: the entry at
    /// that index, else the default (the list's last), as
    /// [`LocType::multi_loc`](super::loc::LocType::multi_loc) picks. `None`
    /// when it shows as nothing; its own id when it is no multi-npc.
    #[cfg(since_647)]
    pub fn multi_npc(&self, var_value: i32) -> Option<u16> {
        let Some(multi) = self.multinpc.as_deref() else {
            return Some(self.id);
        };
        multi
            .get(var_value as usize)
            .copied()
            .filter(|&id| id != -1)
            .or_else(|| multi.last().copied())
            .and_then(|id| u16::try_from(id).ok())
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
            #[cfg(since_647)]
            multivar: raw
                .transform_varbit
                .map(MultiVar::Varbit)
                .or_else(|| raw.transform_varp.map(MultiVar::Varp)),
            #[cfg(since_647)]
            multinpc: raw.transforms,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

#[cfg_attr(since_647, derive(Debug, Clone))]
pub struct NpcTypeRaw {
    pub id: u16,
    pub models: Option<Box<[u16]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub readyanim: Option<u16>,
    pub walkanim: Option<u16>,
    #[cfg(before_254)]
    pub hasalpha: bool,
    pub walkanim_b: Option<u16>,
    pub walkanim_r: Option<u16>,
    pub walkanim_l: Option<u16>,
    pub category: Option<u16>,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub regenrate: u16,
    pub moverestrict: MoveRestrict,
    pub blockwalk: BlockWalk,
    pub defaultmode: NpcMode,
    pub huntmode: Option<u16>,
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
    #[cfg(since_647)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub colour_palette: Option<Box<[u8]>>,
    pub heads: Option<Box<[u16]>>,
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
    #[cfg(since_647)]
    pub transform_varbit: Option<u16>,
    #[cfg(since_647)]
    pub transform_varp: Option<u16>,
    #[cfg(since_647)]
    pub transforms: Option<Box<[i32]>>,
    #[cfg(since_647)]
    pub clickable: bool,
    #[cfg(since_647)]
    pub slowwalk: bool,
    #[cfg(since_647)]
    pub animateidle: bool,
    #[cfg(since_647)]
    pub primary_shadow_colour: i16,
    #[cfg(since_647)]
    pub secondary_shadow_colour: i16,
    #[cfg(since_647)]
    pub primary_shadow_modifier: i8,
    #[cfg(since_647)]
    pub secondary_shadow_modifier: i8,
    #[cfg(since_647)]
    pub walkmask: i8,
    #[cfg(since_647)]
    #[allow(clippy::type_complexity)]
    pub translations: Option<Box<[(u8, [i8; 3])]>>,
    #[cfg(since_647)]
    pub hitbar_sprite: Option<u16>,
    #[cfg(since_647)]
    pub height: Option<u16>,
    #[cfg(since_647)]
    pub respawn_direction: i8,
    #[cfg(since_647)]
    pub render_emote: Option<u16>,
    #[cfg(since_647)]
    pub op128: u8,
    #[cfg(since_647)]
    pub idle_sound: Option<u16>,
    #[cfg(since_647)]
    pub crawl_sound: Option<u16>,
    #[cfg(since_647)]
    pub walk_sound: Option<u16>,
    #[cfg(since_647)]
    pub run_sound: Option<u16>,
    #[cfg(since_647)]
    pub sound_distance: u8,
    #[cfg(since_647)]
    pub primary_cursor_op: Option<u8>,
    #[cfg(since_647)]
    pub primary_cursor: Option<u16>,
    #[cfg(since_647)]
    pub secondary_cursor_op: Option<u8>,
    #[cfg(since_647)]
    pub secondary_cursor: Option<u16>,
    #[cfg(since_647)]
    pub attack_cursor: Option<u16>,
    #[cfg(since_647)]
    pub army_icon: Option<u16>,
    #[cfg(since_647)]
    pub sprite_id: Option<u16>,
    #[cfg(since_647)]
    pub ambient_sound_volume: u8,
    #[cfg(since_647)]
    pub visible_priority: bool,
    #[cfg(since_647)]
    pub mapfunction: Option<u16>,
    #[cfg(since_647)]
    pub invisible_priority: bool,
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
    pub main_option_index: Option<i8>,
    #[cfg(since_647)]
    pub campaigns: Option<Box<[i16]>>,
    #[cfg(since_647)]
    pub vorbis: bool,
    #[cfg(since_647)]
    pub slayer_type: Option<u8>,
    #[cfg(since_647)]
    pub sound_rate_min: i32,
    #[cfg(since_647)]
    pub sound_rate_max: i32,
    #[cfg(since_647)]
    pub pick_size_shift: u8,
    #[cfg(since_647)]
    pub sound_range_min: u8,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Box<str>,
    pub opcode_order: Vec<u8>,
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for NpcTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
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
            #[cfg(since_647)]
            retex_s: None,
            #[cfg(since_647)]
            retex_d: None,
            #[cfg(since_647)]
            colour_palette: None,
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
            turnspeed: if cfg!(since_647) { 32 } else { 0 },
            #[cfg(since_647)]
            transform_varbit: None,
            #[cfg(since_647)]
            transform_varp: None,
            #[cfg(since_647)]
            transforms: None,
            #[cfg(since_647)]
            clickable: true,
            #[cfg(since_647)]
            slowwalk: true,
            #[cfg(since_647)]
            animateidle: true,
            #[cfg(since_647)]
            primary_shadow_colour: 0,
            #[cfg(since_647)]
            secondary_shadow_colour: 0,
            #[cfg(since_647)]
            primary_shadow_modifier: -96,
            #[cfg(since_647)]
            secondary_shadow_modifier: -16,
            #[cfg(since_647)]
            walkmask: 0,
            #[cfg(since_647)]
            translations: None,
            #[cfg(since_647)]
            hitbar_sprite: None,
            #[cfg(since_647)]
            height: None,
            #[cfg(since_647)]
            respawn_direction: 4,
            #[cfg(since_647)]
            render_emote: None,
            #[cfg(since_647)]
            op128: 0,
            #[cfg(since_647)]
            idle_sound: None,
            #[cfg(since_647)]
            crawl_sound: None,
            #[cfg(since_647)]
            walk_sound: None,
            #[cfg(since_647)]
            run_sound: None,
            #[cfg(since_647)]
            sound_distance: 0,
            #[cfg(since_647)]
            primary_cursor_op: None,
            #[cfg(since_647)]
            primary_cursor: None,
            #[cfg(since_647)]
            secondary_cursor_op: None,
            #[cfg(since_647)]
            secondary_cursor: None,
            #[cfg(since_647)]
            attack_cursor: None,
            #[cfg(since_647)]
            army_icon: None,
            #[cfg(since_647)]
            sprite_id: None,
            #[cfg(since_647)]
            ambient_sound_volume: 255,
            #[cfg(since_647)]
            visible_priority: false,
            #[cfg(since_647)]
            mapfunction: None,
            #[cfg(since_647)]
            invisible_priority: false,
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
            main_option_index: None,
            #[cfg(since_647)]
            campaigns: None,
            #[cfg(since_647)]
            vorbis: false,
            #[cfg(since_647)]
            slayer_type: None,
            #[cfg(since_647)]
            sound_rate_min: 256,
            #[cfg(since_647)]
            sound_rate_max: 256,
            #[cfg(since_647)]
            pick_size_shift: 0,
            #[cfg(since_647)]
            sound_range_min: 0,
            params: None,
            debugname: Box::default(),
            opcode_order: Vec::new(),
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            let start = buf.pos;
            self.opcode_order.push(code);
            match code {
                1 => self.models = Some(read_array(buf, Packet::g2)),
                2 => self.name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                12 => self.size = buf.g1(),
                #[cfg(before_647)]
                13 => self.readyanim = Some(buf.g2()),
                #[cfg(before_647)]
                14 => self.walkanim = Some(buf.g2()),
                #[cfg(before_254)]
                16 => self.hasalpha = true,
                #[cfg(before_647)]
                17 => {
                    self.walkanim = Some(buf.g2());
                    self.walkanim_b = Some(buf.g2());
                    self.walkanim_r = Some(buf.g2());
                    self.walkanim_l = Some(buf.g2());
                }
                18 => self.category = Some(buf.g2()),
                19 => self.timer = Some(buf.g2()),
                20 => self.respawnrate = buf.g2(),
                21 => self.regenrate = buf.g2(),
                22 => self.moverestrict = MoveRestrict::try_from(buf.g1()).unwrap(),
                23 => self.blockwalk = BlockWalk::try_from(buf.g1()).unwrap(),
                24 => self.defaultmode = NpcMode::try_from(buf.g1()).unwrap(),
                25 => self.huntmode = Some(buf.g1() as u16),
                26 => self.wanderrange = buf.g2(),
                27 => self.maxrange = Some(buf.g2()),
                28 => self.huntrange = buf.g1(),
                29 => self.attackrange = buf.g1(),
                30..=34 => {
                    let op = buf.gjstr(STRING_TERMINATOR);
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(op.into_boxed_str());
                }
                35 => self.decode_patrol(buf),
                36 => self.members = true,
                37 => self.givechase = false,
                40 => read_pairs(buf, &mut self.recol_s, &mut self.recol_d),
                #[cfg(since_647)]
                41 => read_pairs(buf, &mut self.retex_s, &mut self.retex_d),
                #[cfg(since_647)]
                42 => self.colour_palette = Some(read_array(buf, Packet::g1)),
                60 => self.heads = Some(read_array(buf, Packet::g2)),
                74 => self.attack = buf.g2(),
                75 => self.defence = buf.g2(),
                76 => self.strength = buf.g2(),
                77 => self.hitpoints = buf.g2(),
                78 => self.ranged = buf.g2(),
                79 => self.magic = buf.g2(),
                #[cfg(before_647)]
                90 => self.resizex = Some(buf.g2()),
                #[cfg(before_647)]
                91 => self.resizey = Some(buf.g2()),
                #[cfg(before_647)]
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
                #[cfg(since_647)]
                106 => self.decode_multinpc(buf, false),
                #[cfg(since_647)]
                107 => self.clickable = false,
                #[cfg(since_647)]
                109 => self.slowwalk = false,
                #[cfg(since_647)]
                111 => self.animateidle = false,
                #[cfg(since_647)]
                113 => {
                    self.primary_shadow_colour = buf.g2s();
                    self.secondary_shadow_colour = buf.g2s();
                }
                #[cfg(since_647)]
                114 => {
                    self.primary_shadow_modifier = buf.g1s();
                    self.secondary_shadow_modifier = buf.g1s();
                }
                #[cfg(since_647)]
                118 => self.decode_multinpc(buf, true),
                #[cfg(since_647)]
                119 => self.walkmask = buf.g1s(),
                #[cfg(since_647)]
                121 => {
                    self.translations = Some(read_array(buf, |buf| {
                        (buf.g1(), [buf.g1s(), buf.g1s(), buf.g1s()])
                    }));
                }
                #[cfg(since_647)]
                122 => self.hitbar_sprite = Some(buf.g2()),
                #[cfg(since_647)]
                123 => self.height = Some(buf.g2()),
                #[cfg(since_647)]
                125 => self.respawn_direction = buf.g1s(),
                #[cfg(since_647)]
                127 => self.render_emote = Some(buf.g2()),
                #[cfg(since_647)]
                128 => self.op128 = buf.g1(),
                #[cfg(since_647)]
                134 => {
                    self.idle_sound = read_optional_id(buf);
                    self.crawl_sound = read_optional_id(buf);
                    self.walk_sound = read_optional_id(buf);
                    self.run_sound = read_optional_id(buf);
                    self.sound_distance = buf.g1();
                }
                #[cfg(since_647)]
                135 => {
                    self.primary_cursor_op = Some(buf.g1());
                    self.primary_cursor = Some(buf.g2());
                }
                #[cfg(since_647)]
                136 => {
                    self.secondary_cursor_op = Some(buf.g1());
                    self.secondary_cursor = Some(buf.g2());
                }
                #[cfg(since_647)]
                137 => self.attack_cursor = Some(buf.g2()),
                #[cfg(since_647)]
                138 => self.army_icon = Some(buf.g2()),
                #[cfg(since_647)]
                139 => self.sprite_id = Some(buf.g2()),
                #[cfg(since_647)]
                140 => self.ambient_sound_volume = buf.g1(),
                #[cfg(since_647)]
                141 => self.visible_priority = true,
                #[cfg(since_647)]
                142 => self.mapfunction = Some(buf.g2()),
                #[cfg(since_647)]
                143 => self.invisible_priority = true,
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
                158 => self.main_option_index = Some(1),
                #[cfg(since_647)]
                159 => self.main_option_index = Some(0),
                #[cfg(since_647)]
                160 => self.campaigns = Some(read_array(buf, Packet::g2s)),
                #[cfg(since_647)]
                162 => self.vorbis = true,
                #[cfg(since_647)]
                163 => self.slayer_type = Some(buf.g1()),
                #[cfg(since_647)]
                164 => {
                    self.sound_rate_min = buf.g2() as i32;
                    self.sound_rate_max = buf.g2() as i32;
                }
                #[cfg(since_647)]
                165 => self.pick_size_shift = buf.g1(),
                #[cfg(since_647)]
                168 => self.sound_range_min = buf.g1(),
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
            self.raw_opcodes
                .push((code, buf.data[start..buf.pos].to_vec()));
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

    #[cfg(since_647)]
    fn decode_multinpc(&mut self, buf: &mut Packet, has_default: bool) {
        self.transform_varbit = read_optional_id(buf);
        self.transform_varp = read_optional_id(buf);
        let default = if has_default { read_id(buf) } else { -1 };
        self.transforms = Some(read_transforms(buf, default));
    }
}
