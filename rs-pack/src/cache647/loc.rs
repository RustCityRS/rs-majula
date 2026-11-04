#![allow(clippy::type_complexity)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! LocType — CacheType implementation for location/object definitions.
//!
//! Decoded from archive 16, rev 647. Mirrors the client's `LocType.decode`.

use std::collections::HashMap;

use rs_io::Packet;

use super::provider::{CacheType, ParamValue};

/// Loc type definition.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocType {
    pub id: u16,
    pub name: Option<Box<str>>,
    /// Examine text: server data at 647, packed as opcode 3, which the
    /// client's decoder does not have.
    pub desc: Option<Box<str>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    pub blockrange: bool,
    pub interact_type: i32,
    pub active: i32,
    pub hillskew: i32,
    pub hillskew_param: i32,
    pub sharelight: bool,
    pub occlude: bool,
    pub anim: i32,
    pub hasalpha: bool,
    pub wallwidth: i32,
    pub ambient: i8,
    pub contrast: i32,
    pub op: [Option<Box<str>>; 5],
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    pub retex_s: Option<Box<[u16]>>,
    pub retex_d: Option<Box<[u16]>>,
    pub recol_palette: Option<Box<[i8]>>,
    pub mirror: bool,
    pub shadow: bool,
    pub resizex: u16,
    pub resizey: u16,
    pub resizez: u16,
    pub forceapproach: u8,
    pub offsetx: i32,
    pub offsety: i32,
    pub offsetz: i32,
    pub forcedecor: bool,
    pub breakroutefinding: bool,
    pub raiseobject: i32,
    pub ambient_sfx_id: i32,
    pub ambient_sfx_range: u8,
    pub sfx_random_pri: i32,
    pub sfx_random_sec: i32,
    pub sfx_random_ids: Option<Box<[u16]>>,
    pub hide_minimap: bool,
    pub minimap_visible: bool,
    pub blocks_npcs: bool,
    pub unknown_90: bool,
    pub members_only: bool,
    pub interaction_side_flags: u8,
    pub unknown_105: bool,
    pub random_anim_ids: Option<Box<[u16]>>,
    pub random_anim_weights: Option<Box<[u8]>>,
    pub map_element: i32,
    pub members_op: [Option<Box<str>>; 5],
    pub transform_varbit: i32,
    pub transform_varp: i32,
    pub transforms: Option<Box<[i32]>>,
    pub model_types: Option<Box<[i8]>>,
    pub models: Option<Box<[Box<[u16]>]>>,
    pub hd_model_types: Option<Box<[i8]>>,
    pub hd_models: Option<Box<[Box<[u16]>]>>,
    pub quest_ids: Option<Box<[u16]>>,
    pub category: i32,
    pub msi_id: i32,
    pub unknown_97: bool,
    pub unknown_98: bool,
    pub cursor_op1: Option<(u8, u16)>,
    pub cursor_op2: Option<(u8, u16)>,
    pub mapfunction: u8,
    pub head_icon: i32,
    pub unknown_168: bool,
    pub unknown_169: bool,
    pub unknown_177: bool,
    pub params: Option<HashMap<i32, ParamValue>>,
    /// Param keys in original insertion order (for byte-perfect re-emission).
    pub param_order: Vec<i32>,
    pub op163: [i8; 4],
    pub op164: i16,
    pub op165: i16,
    pub op166: i16,
    pub op167: i32,
    pub op170: i32,
    pub op171: i32,
    pub op173: (u16, u16),
    pub op178: u8,
    /// Opcodes in decode order, for byte-perfect re-emission.
    #[serde(skip)]
    pub opcode_order: Vec<u8>,
    /// Raw opcode payloads recorded during decode, for faithful re-emission of
    /// duplicate opcodes whose earlier values were overwritten in the struct.
    #[serde(skip)]
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

fn read_models(buf: &mut Packet) -> (Box<[i8]>, Box<[Box<[u16]>]>) {
    let count = buf.g1() as usize;
    let mut types = Vec::with_capacity(count);
    let mut models = Vec::with_capacity(count);
    for _ in 0..count {
        let model_type = buf.g1s();
        let sub_count = buf.g1() as usize;
        let mut ids = Vec::with_capacity(sub_count);
        for _ in 0..sub_count {
            ids.push(buf.g2());
        }
        types.push(model_type);
        models.push(ids.into_boxed_slice());
    }
    (types.into_boxed_slice(), models.into_boxed_slice())
}

impl CacheType for LocType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            name: None,
            desc: None,
            width: 1,
            length: 1,
            blockwalk: true,
            blockrange: true,
            interact_type: 2,
            active: -1,
            hillskew: 0,
            hillskew_param: -1,
            sharelight: false,
            occlude: false,
            anim: -1,
            hasalpha: false,
            wallwidth: 64,
            ambient: 0,
            contrast: 0,
            op: [const { None }; 5],
            recol_s: None,
            recol_d: None,
            retex_s: None,
            retex_d: None,
            recol_palette: None,
            mirror: false,
            shadow: true,
            resizex: 128,
            resizey: 128,
            resizez: 128,
            forceapproach: 0,
            offsetx: 0,
            offsety: 0,
            offsetz: 0,
            forcedecor: false,
            breakroutefinding: false,
            raiseobject: -1,
            ambient_sfx_id: 0,
            ambient_sfx_range: 0,
            sfx_random_pri: 0,
            sfx_random_sec: 0,
            sfx_random_ids: None,
            hide_minimap: false,
            minimap_visible: true,
            blocks_npcs: true,
            unknown_90: false,
            members_only: false,
            interaction_side_flags: 255,
            unknown_105: false,
            random_anim_ids: None,
            random_anim_weights: None,
            map_element: -1,
            members_op: [const { None }; 5],
            transform_varbit: -1,
            transform_varp: -1,
            transforms: None,
            model_types: None,
            models: None,
            hd_model_types: None,
            hd_models: None,
            quest_ids: None,
            category: -1,
            msi_id: -1,
            unknown_97: false,
            unknown_98: false,
            cursor_op1: None,
            cursor_op2: None,
            mapfunction: 0,
            head_icon: -1,
            unknown_168: false,
            unknown_169: false,
            unknown_177: false,
            params: None,
            param_order: Vec::new(),
            op163: [0; 4],
            op164: 0,
            op165: 0,
            op166: 0,
            op167: 0,
            op170: 0,
            op171: 0,
            op173: (0, 0),
            op178: 0,
            opcode_order: Vec::new(),
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            self.opcode_order.push(opcode);
            let raw_start = buf.pos;
            match opcode {
                1 => {
                    let (types, ids) = read_models(buf);
                    self.model_types = Some(types);
                    self.models = Some(ids);
                }
                5 => {
                    let (sd_types, sd_ids) = read_models(buf);
                    self.model_types = Some(sd_types);
                    self.models = Some(sd_ids);
                    let (hd_types, hd_ids) = read_models(buf);
                    self.hd_model_types = Some(hd_types);
                    self.hd_models = Some(hd_ids);
                }
                2 => {
                    self.name = Some(buf.gjstr(0).into_boxed_str());
                }
                3 => {
                    self.desc = Some(buf.gjstr(0).into_boxed_str());
                }
                14 => self.width = buf.g1(),
                15 => self.length = buf.g1(),
                17 => {
                    self.interact_type = 0;
                    self.blockwalk = false;
                }
                18 => self.blockrange = false,
                19 => self.active = buf.g1() as i32,
                21 => self.hillskew = 1,
                22 => self.sharelight = true,
                23 => self.occlude = true,
                24 => {
                    let v = buf.g2();
                    self.anim = if v == 0xFFFF { -1 } else { v as i32 };
                }
                25 => self.hasalpha = true,
                27 => self.interact_type = 1,
                28 => self.wallwidth = (buf.g1() as i32) << 2,
                29 => self.ambient = buf.g1s(),
                30..=34 => {
                    let slot = (opcode - 30) as usize;
                    let s = buf.gjstr(0);
                    self.op[slot] = if s.is_empty() {
                        None
                    } else {
                        Some(s.into_boxed_str())
                    };
                }
                39 => self.contrast = (buf.g1s() as i32) * 5,
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
                        palette.push(buf.g1s());
                    }
                    self.recol_palette = Some(palette.into_boxed_slice());
                }
                62 => self.mirror = true,
                64 => self.shadow = false,
                65 => self.resizex = buf.g2(),
                66 => self.resizey = buf.g2(),
                67 => self.resizez = buf.g2(),
                69 => self.forceapproach = buf.g1(),
                70 => {
                    let raw = buf.g2() as i32;
                    self.offsetx = (if raw > 32767 { raw - 65536 } else { raw }) << 2;
                }
                71 => {
                    let raw = buf.g2() as i32;
                    self.offsety = (if raw > 32767 { raw - 65536 } else { raw }) << 2;
                }
                72 => {
                    let raw = buf.g2() as i32;
                    self.offsetz = (if raw > 32767 { raw - 65536 } else { raw }) << 2;
                }
                73 => self.forcedecor = true,
                74 => self.breakroutefinding = true,
                75 => self.raiseobject = buf.g1() as i32,
                77 | 92 => {
                    let varbit = buf.g2();
                    self.transform_varbit = if varbit == 0xFFFF { -1 } else { varbit as i32 };
                    let varp = buf.g2();
                    self.transform_varp = if varp == 0xFFFF { -1 } else { varp as i32 };
                    let default_transform = if opcode == 92 {
                        let d = buf.g2();
                        if d == 0xFFFF { -1 } else { d as i32 }
                    } else {
                        -1
                    };
                    let count = buf.g1() as usize;
                    let mut transforms = Vec::with_capacity(count + 2);
                    for _ in 0..=count {
                        let v = buf.g2();
                        transforms.push(if v == 0xFFFF { -1 } else { v as i32 });
                    }
                    transforms.push(default_transform);
                    self.transforms = Some(transforms.into_boxed_slice());
                }
                78 => {
                    self.ambient_sfx_id = buf.g2() as i32;
                    self.ambient_sfx_range = buf.g1();
                }
                79 => {
                    self.sfx_random_pri = buf.g2() as i32;
                    self.sfx_random_sec = buf.g2() as i32;
                    self.ambient_sfx_range = buf.g1();
                    let count = buf.g1() as usize;
                    let mut ids = Vec::with_capacity(count);
                    for _ in 0..count {
                        ids.push(buf.g2());
                    }
                    self.sfx_random_ids = Some(ids.into_boxed_slice());
                }
                81 => {
                    self.hillskew = 2;
                    self.hillskew_param = (buf.g1() as i32) * 256;
                }
                82 => self.hide_minimap = true,
                88 => self.minimap_visible = false,
                89 => self.blocks_npcs = false,
                90 => self.unknown_90 = true,
                91 => self.members_only = true,
                93 => {
                    self.hillskew = 3;
                    self.hillskew_param = buf.g2() as i32;
                }
                94 => self.hillskew = 4,
                95 => {
                    self.hillskew = 5;
                    let raw = buf.g2() as i32;
                    self.hillskew_param = if raw > 32767 { raw - 65536 } else { raw };
                }
                97 => self.unknown_97 = true,
                98 => self.unknown_98 = true,
                99 => self.cursor_op1 = Some((buf.g1(), buf.g2())),
                100 => self.cursor_op2 = Some((buf.g1(), buf.g2())),
                101 => self.mapfunction = buf.g1(),
                102 => self.msi_id = buf.g2() as i32,
                103 => self.occlude = false,
                104 => self.interaction_side_flags = buf.g1(),
                105 => self.unknown_105 = true,
                106 => {
                    let count = buf.g1() as usize;
                    let mut ids = Vec::with_capacity(count);
                    let mut weights = Vec::with_capacity(count);
                    for _ in 0..count {
                        ids.push(buf.g2());
                        weights.push(buf.g1());
                    }
                    self.random_anim_ids = Some(ids.into_boxed_slice());
                    self.random_anim_weights = Some(weights.into_boxed_slice());
                }
                107 => self.map_element = buf.g2() as i32,
                150..=154 => {
                    let slot = (opcode - 150) as usize;
                    let s = buf.gjstr(0);
                    self.members_op[slot] = if s.is_empty() {
                        None
                    } else {
                        Some(s.into_boxed_str())
                    };
                }
                160 => {
                    let count = buf.g1() as usize;
                    let mut quests = Vec::with_capacity(count);
                    for _ in 0..count {
                        quests.push(buf.g2());
                    }
                    self.quest_ids = Some(quests.into_boxed_slice());
                }
                162 => {
                    self.hillskew = 3;
                    self.hillskew_param = buf.g4s();
                }
                163 => {
                    self.op163 = [buf.g1s(), buf.g1s(), buf.g1s(), buf.g1s()];
                }
                164 => self.op164 = buf.g2s(),
                165 => self.op165 = buf.g2s(),
                166 => self.op166 = buf.g2s(),
                167 => self.op167 = buf.g2() as i32,
                168 => self.unknown_168 = true,
                169 => self.unknown_169 = true,
                170 => self.op170 = buf.gsmart1or2(),
                171 => self.op171 = buf.gsmart1or2(),
                173 => self.op173 = (buf.g2(), buf.g2()),
                177 => self.unknown_177 = true,
                178 => self.op178 = buf.g1(),
                249 => {
                    let count = buf.g1() as usize;
                    let mut map = self.params.take().unwrap_or_default();
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
                    self.params = Some(map);
                }
                // ── Server opcodes (merged into server .dat) ──
                61 => self.category = buf.g2() as i32,
                250 => {
                    let _debugname = buf.gjstr(0);
                }
                _ => panic!("LocType: unknown opcode {opcode} at id {}", self.id),
            }
            let raw_end = buf.pos;
            self.raw_opcodes
                .push((opcode, buf.data[raw_start..raw_end].to_vec()));
        }
    }

    fn post_decode(types: &mut Vec<Self>, _ctx: &Self::Context) {
        for loc in types.iter_mut() {
            if loc.active == -1 {
                loc.active = 0;
                if let Some(ref mt) = loc.model_types {
                    if mt.len() == 1 && mt[0] == 10 {
                        loc.active = 1;
                    }
                }
                if loc.op.iter().any(|o| o.is_some()) {
                    loc.active = 1;
                }
            }
            if loc.raiseobject == -1 {
                loc.raiseobject = if loc.interact_type == 0 { 0 } else { 1 };
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

impl LocType {
    /// Get an integer param value.
    pub fn param_int(&self, key: i32) -> Option<i32> {
        self.params.as_ref()?.get(&key).and_then(|v| match v {
            ParamValue::Int(i) => Some(*i),
            _ => None,
        })
    }

    /// Get a string param value.
    pub fn param_str(&self, key: i32) -> Option<&str> {
        self.params.as_ref()?.get(&key).and_then(|v| match v {
            ParamValue::String(s) => Some(&**s),
            _ => None,
        })
    }

    /// Alias for engine code that uses u32 param keys.
    pub fn get_param_int(&self, key: u32) -> Option<i32> {
        self.param_int(key as i32)
    }

    /// Alias for engine code that uses u32 param keys.
    pub fn get_param_string(&self, key: u32) -> Option<&str> {
        self.param_str(key as i32)
    }
}
