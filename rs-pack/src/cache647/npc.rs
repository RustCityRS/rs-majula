#![allow(clippy::type_complexity)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! NpcType — CacheType implementation for NPC configs (archive 18).
//!
//! Decoded from the rev-647 client's `NpcType.decode(Packet, int)`.
//! This replaces the old `NpcDefinition` from rs-definitions with a clean
//! CacheType-based decoder using `Packet` directly.

use std::collections::HashMap;

use rs_io::Packet;

use super::provider::{CacheType, ParamValue};

/// NPC type definition decoded from the client cache.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NpcType {
    pub id: u16,
    pub name: Option<Box<str>>,
    /// Examine text: server data at 647, packed as opcode 3, which the
    /// client's decoder does not have.
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub models: Option<Box<[u16]>>,
    pub options: [Option<Box<str>>; 5],
    pub members_options: [Option<Box<str>>; 5],
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    pub retex_s: Option<Box<[u16]>>,
    pub retex_d: Option<Box<[u16]>>,
    pub colour_palette: Option<Box<[u8]>>,
    pub head_models: Option<Box<[u16]>>,
    pub minimap: bool,
    pub combat: i32,
    pub resizeh: i32,
    pub resizev: i32,
    pub alwaysontop: bool,
    pub ambient: i8,
    pub contrast: i32,
    pub headicon: i32,
    pub turnspeed: i32,
    pub transform_varbit: i32,
    pub transform_varp: i32,
    pub transforms: Option<Box<[i32]>>,
    pub clickable: bool,
    pub slowwalk: bool,
    pub animateidle: bool,
    pub primary_shadow_colour: i16,
    pub secondary_shadow_colour: i16,
    pub primary_shadow_modifier: i8,
    pub secondary_shadow_modifier: i8,
    pub walkmask: i8,
    pub translations: Option<Box<[(u8, [i8; 3])]>>,
    pub hitbar_sprite: i32,
    pub height: i32,
    pub respawn_direction: i8,
    pub render_emote: i32,
    pub op128: u8,
    pub idle_sound: i32,
    pub crawl_sound: i32,
    pub walk_sound: i32,
    pub run_sound: i32,
    pub sound_distance: u8,
    pub primary_cursor_op: u8,
    pub primary_cursor: i32,
    pub secondary_cursor_op: u8,
    pub secondary_cursor: i32,
    pub attack_cursor: i32,
    pub army_icon: i32,
    pub sprite_id: i32,
    pub ambient_sound_volume: u8,
    pub visible_priority: bool,
    pub mapfunction: i32,
    pub invisible_priority: bool,
    pub hue: i8,
    pub saturation: i8,
    pub lightness: i8,
    pub opacity: i8,
    pub main_option_index: i8,
    pub campaigns: Option<Box<[i16]>>,
    pub vorbis: bool,
    pub slayer_type: u8,
    pub sound_rate_min: i32,
    pub sound_rate_max: i32,
    pub pick_size_shift: u8,
    pub sound_range_min: u8,
    pub params: Option<Box<HashMap<i32, ParamValue>>>,
    /// Param keys in original insertion order (for byte-perfect re-emission).
    pub param_order: Vec<i32>,

    // ── Server-only fields (opcodes 18, 74-79, 200-214, 250) ──
    pub category: i32,
    pub attack: u16,
    pub defence: u16,
    pub strength: u16,
    pub hitpoints: u16,
    pub ranged: u16,
    pub magic: u16,
    pub wander_range: u16,
    pub max_range: u16,
    pub hunt_range: u8,
    pub timer: u16,
    pub respawn_rate: u16,
    pub move_restrict: u8,
    pub attack_range: u16,
    pub block_walk: u8,
    pub hunt_mode: u8,
    pub default_mode: u8,
    pub members: bool,
    pub patrol: Option<Box<[(i32, u8)]>>,
    pub give_chase: bool,
    pub regen_rate: u16,
    debugname: Option<Box<str>>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
    /// Raw opcode payloads recorded during decode, for faithful re-emission of
    /// duplicate opcodes whose earlier values were overwritten in the struct.
    #[serde(skip)]
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for NpcType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            name: None,
            desc: None,
            size: 1,
            models: None,
            options: [const { None }; 5],
            members_options: [const { None }; 5],
            recol_s: None,
            recol_d: None,
            retex_s: None,
            retex_d: None,
            colour_palette: None,
            head_models: None,
            minimap: true,
            combat: -1,
            resizeh: -1,
            resizev: -1,
            alwaysontop: false,
            ambient: 0,
            contrast: 0,
            headicon: -1,
            turnspeed: 32,
            transform_varbit: -1,
            transform_varp: -1,
            transforms: None,
            clickable: true,
            slowwalk: true,
            animateidle: true,
            primary_shadow_colour: 0,
            secondary_shadow_colour: 0,
            primary_shadow_modifier: 0,
            secondary_shadow_modifier: 0,
            walkmask: 0,
            translations: None,
            hitbar_sprite: -1,
            height: -1,
            respawn_direction: 0,
            render_emote: -1,
            op128: 0,
            idle_sound: -1,
            crawl_sound: -1,
            walk_sound: -1,
            run_sound: -1,
            sound_distance: 0,
            primary_cursor_op: 0,
            primary_cursor: 0,
            secondary_cursor_op: 0,
            secondary_cursor: 0,
            attack_cursor: -1,
            army_icon: -1,
            sprite_id: -1,
            ambient_sound_volume: 0,
            visible_priority: false,
            mapfunction: -1,
            invisible_priority: false,
            hue: 0,
            saturation: 0,
            lightness: 0,
            opacity: 0,
            main_option_index: -1,
            campaigns: None,
            vorbis: false,
            slayer_type: 0,
            sound_rate_min: 0,
            sound_rate_max: 0,
            pick_size_shift: 0,
            sound_range_min: 0,
            params: None,
            param_order: Vec::new(),
            category: -1,
            attack: 0,
            defence: 0,
            strength: 0,
            hitpoints: 0,
            ranged: 0,
            magic: 0,
            wander_range: 0,
            max_range: 0,
            hunt_range: 0,
            timer: 0,
            // Jagex/2004scape default when opcode 204 is absent: 100 ticks
            // (~60s). A 0 default makes world NPCs respawn the same tick they
            // die (they appear never to despawn). Matches rs-majula NpcType.
            respawn_rate: 100,
            move_restrict: 0,
            attack_range: 0,
            block_walk: 2,
            hunt_mode: 0,
            default_mode: 0,
            members: false,
            patrol: None,
            give_chase: true,
            regen_rate: 0,
            debugname: None,
            opcode_order: Vec::new(),
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            self.opcode_order.push(code);
            let start = buf.pos;
            self.decode_opcode(buf, code);
            let end = buf.pos;
            self.raw_opcodes.push((code, buf.data[start..end].to_vec()));
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref().or(self.name.as_deref())
    }
}

impl NpcType {
    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => {
                let count = buf.g1() as usize;
                let mut ids = Vec::with_capacity(count);
                for _ in 0..count {
                    ids.push(buf.g2());
                }
                self.models = Some(ids.into_boxed_slice());
            }
            2 => {
                self.name = Some(buf.gjstr(0).into_boxed_str());
            }
            3 => {
                self.desc = Some(buf.gjstr(0).into_boxed_str());
            }
            12 => {
                self.size = buf.g1();
            }
            30..=34 => {
                let slot = (code - 30) as usize;
                let s = buf.gjstr(0);
                self.options[slot] = if s.is_empty() {
                    None
                } else {
                    Some(s.into_boxed_str())
                };
            }
            40 => {
                let count = buf.g1() as usize;
                let mut src = Vec::with_capacity(count);
                let mut dst = Vec::with_capacity(count);
                for _ in 0..count {
                    src.push(buf.g2());
                    dst.push(buf.g2());
                }
                self.recol_s = Some(src.into_boxed_slice());
                self.recol_d = Some(dst.into_boxed_slice());
            }
            41 => {
                let count = buf.g1() as usize;
                let mut src = Vec::with_capacity(count);
                let mut dst = Vec::with_capacity(count);
                for _ in 0..count {
                    src.push(buf.g2());
                    dst.push(buf.g2());
                }
                self.retex_s = Some(src.into_boxed_slice());
                self.retex_d = Some(dst.into_boxed_slice());
            }
            42 => {
                let count = buf.g1() as usize;
                let mut palette = Vec::with_capacity(count);
                for _ in 0..count {
                    palette.push(buf.g1());
                }
                self.colour_palette = Some(palette.into_boxed_slice());
            }
            60 => {
                let count = buf.g1() as usize;
                let mut ids = Vec::with_capacity(count);
                for _ in 0..count {
                    ids.push(buf.g2());
                }
                self.head_models = Some(ids.into_boxed_slice());
            }
            93 => {
                self.minimap = false;
            }
            95 => {
                self.combat = buf.g2() as i32;
            }
            97 => {
                self.resizeh = buf.g2() as i32;
            }
            98 => {
                self.resizev = buf.g2() as i32;
            }
            99 => {
                self.alwaysontop = true;
            }
            100 => {
                self.ambient = buf.g1s();
            }
            101 => {
                self.contrast = buf.g1s() as i32 * 5;
            }
            102 => {
                self.headicon = buf.g2() as i32;
            }
            103 => {
                self.turnspeed = buf.g2() as i32;
            }
            106 | 118 => {
                let varbit_raw = buf.g2();
                self.transform_varbit = if varbit_raw == 65535 {
                    -1
                } else {
                    varbit_raw as i32
                };
                let varp_raw = buf.g2();
                self.transform_varp = if varp_raw == 65535 {
                    -1
                } else {
                    varp_raw as i32
                };

                let mut default_transform = -1i32;
                if code == 118 {
                    let v = buf.g2();
                    default_transform = if v == 65535 { -1 } else { v as i32 };
                }

                let count = buf.g1() as usize;
                let mut transforms = Vec::with_capacity(count + 2);
                for _ in 0..=count {
                    let v = buf.g2();
                    transforms.push(if v == 65535 { -1 } else { v as i32 });
                }
                transforms.push(default_transform);
                self.transforms = Some(transforms.into_boxed_slice());
            }
            107 => {
                self.clickable = false;
            }
            109 => {
                self.slowwalk = false;
            }
            111 => {
                self.animateidle = false;
            }
            113 => {
                self.primary_shadow_colour = buf.g2s();
                self.secondary_shadow_colour = buf.g2s();
            }
            114 => {
                self.primary_shadow_modifier = buf.g1s();
                self.secondary_shadow_modifier = buf.g1s();
            }
            119 => {
                self.walkmask = buf.g1s();
            }
            121 => {
                let count = buf.g1() as usize;
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let index = buf.g1();
                    let x = buf.g1s();
                    let y = buf.g1s();
                    let z = buf.g1s();
                    entries.push((index, [x, y, z]));
                }
                self.translations = Some(entries.into_boxed_slice());
            }
            122 => {
                self.hitbar_sprite = buf.g2() as i32;
            }
            123 => {
                self.height = buf.g2() as i32;
            }
            125 => {
                self.respawn_direction = buf.g1s();
            }
            127 => {
                self.render_emote = buf.g2() as i32;
            }
            128 => {
                self.op128 = buf.g1();
            }
            134 => {
                let raw_idle = buf.g2();
                self.idle_sound = if raw_idle == 65535 {
                    -1
                } else {
                    raw_idle as i32
                };
                let raw_crawl = buf.g2();
                self.crawl_sound = if raw_crawl == 65535 {
                    -1
                } else {
                    raw_crawl as i32
                };
                let raw_walk = buf.g2();
                self.walk_sound = if raw_walk == 65535 {
                    -1
                } else {
                    raw_walk as i32
                };
                let raw_run = buf.g2();
                self.run_sound = if raw_run == 65535 { -1 } else { raw_run as i32 };
                self.sound_distance = buf.g1();
            }
            135 => {
                self.primary_cursor_op = buf.g1();
                self.primary_cursor = buf.g2() as i32;
            }
            136 => {
                self.secondary_cursor_op = buf.g1();
                self.secondary_cursor = buf.g2() as i32;
            }
            137 => {
                self.attack_cursor = buf.g2() as i32;
            }
            138 => {
                self.army_icon = buf.g2() as i32;
            }
            139 => {
                self.sprite_id = buf.g2() as i32;
            }
            140 => {
                self.ambient_sound_volume = buf.g1();
            }
            141 => {
                self.visible_priority = true;
            }
            142 => {
                self.mapfunction = buf.g2() as i32;
            }
            143 => {
                self.invisible_priority = true;
            }
            150..=154 => {
                let slot = (code - 150) as usize;
                let s = buf.gjstr(0);
                self.members_options[slot] = if s.is_empty() {
                    None
                } else {
                    Some(s.into_boxed_str())
                };
            }
            155 => {
                self.hue = buf.g1s();
                self.saturation = buf.g1s();
                self.lightness = buf.g1s();
                self.opacity = buf.g1s();
            }
            158 => {
                self.main_option_index = 1;
            }
            159 => {
                self.main_option_index = 0;
            }
            160 => {
                let count = buf.g1() as usize;
                let mut campaigns = Vec::with_capacity(count);
                for _ in 0..count {
                    campaigns.push(buf.g2() as i16);
                }
                self.campaigns = Some(campaigns.into_boxed_slice());
            }
            162 => {
                self.vorbis = true;
            }
            163 => {
                self.slayer_type = buf.g1();
            }
            164 => {
                self.sound_rate_min = buf.g2() as i32;
                self.sound_rate_max = buf.g2() as i32;
            }
            165 => {
                self.pick_size_shift = buf.g1();
            }
            168 => {
                self.sound_range_min = buf.g1();
            }
            // ── Server opcodes (merged into server .dat) ──
            18 => self.category = buf.g2() as i32,
            74 => self.attack = buf.g2(),
            75 => self.defence = buf.g2(),
            76 => self.strength = buf.g2(),
            77 => self.hitpoints = buf.g2(),
            78 => self.ranged = buf.g2(),
            79 => self.magic = buf.g2(),
            200 => self.wander_range = buf.g2(),
            201 => self.max_range = buf.g2(),
            202 => self.hunt_range = buf.g1(),
            203 => self.timer = buf.g2(),
            204 => self.respawn_rate = buf.g2(),
            206 => self.move_restrict = buf.g1(),
            207 => self.attack_range = buf.g2(),
            208 => self.block_walk = buf.g1(),
            209 => self.hunt_mode = buf.g1(),
            210 => self.default_mode = buf.g1(),
            211 => self.members = true,
            212 => {
                let count = buf.g1() as usize;
                let mut entries = Vec::with_capacity(count);
                for _ in 0..count {
                    let coord = buf.g4s();
                    let delay = buf.g1();
                    entries.push((coord, delay));
                }
                self.patrol = Some(entries.into_boxed_slice());
            }
            213 => self.give_chase = false,
            214 => self.regen_rate = buf.g2(),
            249 => {
                let count = buf.g1() as usize;
                let mut map = HashMap::with_capacity(count);
                for _ in 0..count {
                    let is_string = buf.g1() == 1;
                    let key = buf.g3();
                    let value = if is_string {
                        ParamValue::String(buf.gjstr(0).into_boxed_str())
                    } else {
                        ParamValue::Int(buf.g4s())
                    };
                    map.insert(key, value);
                    self.param_order.push(key);
                }
                self.params = Some(Box::new(map));
            }
            250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
            _ => {
                buf.pos = buf.data.len();
            }
        }
    }

    /// Whether this NpcType was decoded from a merged server .dat
    /// (i.e. has server-side fields populated via opcode 250 debugname).
    pub fn has_server_data(&self) -> bool {
        self.debugname.is_some()
    }

    /// Get the server-side debugname (opcode 250).
    pub fn server_debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("null")
    }
}
