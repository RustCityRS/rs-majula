//! ObjType — CacheType implementation for item/obj configs (archive 19).
//!
//! Decoded from the rev-647 client's `ObjType.decode(Packet, opcode)`.
//! This replaces the old `ItemDefinition` from rs-definitions with a clean
//! CacheType-based decoder using `Packet` directly.

use std::collections::HashMap;

use rs_io::Packet;

use super::provider::{CacheType, ParamValue};

/// Context passed to `ObjType::post_decode` for members-gating.
pub struct ObjContext {
    pub members: bool,
}

/// Item ("Obj") type definition decoded from the client cache.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ObjType {
    pub id: u16,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub model: u16,
    pub zoom_2d: u16,
    pub xan_2d: u16,
    pub yan_2d: u16,
    pub xof_2d: i16,
    pub yof_2d: i16,
    pub zan_2d: u16,
    pub stackable: bool,
    pub cost: i32,
    pub members: bool,
    pub multistack: u16,
    pub manwear: u16,
    pub manwear2: u16,
    pub manwear3: u16,
    pub womanwear: u16,
    pub womenwear2: u16,
    pub womanwear3: u16,
    pub manhead: u16,
    pub manhead2: u16,
    pub womanhead: u16,
    pub womenhead2: u16,
    pub manwear_xoff: i8,
    pub manwear_yoff: i8,
    pub manwear_zoff: i8,
    pub womanwear_xoff: i8,
    pub womanwear_yoff: i8,
    pub womanwear_zoff: i8,
    pub options: [Option<Box<str>>; 5],
    pub interface_options: [Option<Box<str>>; 5],
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    pub retex_s: Option<Box<[u16]>>,
    pub retex_d: Option<Box<[u16]>>,
    pub recol_palette: Option<Box<[i8]>>,
    pub stockmarket: bool,
    pub certlink: u16,
    pub certtemplate: u16,
    pub lentlink: u16,
    pub lenttemplate: u16,
    pub boughtlink: u16,
    pub boughttemplate: u16,
    pub countobj: Option<Box<[u16]>>,
    pub countco: Option<Box<[u16]>>,
    pub resize_x: u16,
    pub resize_y: u16,
    pub resize_z: u16,
    pub ambient: i8,
    pub contrast: i32,
    pub team: u8,
    pub dummy_obj: u8,
    pub picksizeshift: u8,
    pub cursor1: Option<(u8, u16)>,
    pub cursor2: Option<(u8, u16)>,
    pub icursor1: Option<(u8, u16)>,
    pub icursor2: Option<(u8, u16)>,
    pub quests: Option<Box<[u16]>>,
    pub params: Option<Box<HashMap<i32, ParamValue>>>,
    /// Param keys in original insertion order (for byte-perfect re-emission).
    pub param_order: Vec<i32>,

    // ─── Server-side overlay fields (set by .obj/.dat decoders, not cache) ────
    /// Wearpos 1-3 (server opcodes 13/14/27). Default -1 = not equippable.
    pub wearpos1: i8,
    pub wearpos2: i8,
    pub wearpos3: i8,
    /// Weight in kg (server opcode 75, encoded as i16 / 1000).
    pub weight: f64,
    /// Server-side category (server opcode 94).
    pub category: u16,
    /// Server-side tradeable flag: true unless the config says
    /// `tradeable=no` (server opcode 15), as in Jordan's ObjType.
    pub tradeable: bool,
    /// Integer params from server opcode 249 (partitioned for fast lookup).
    pub iparams: HashMap<u32, i32>,
    /// String params from server opcode 249 (partitioned for fast lookup).
    pub sparams: HashMap<u32, String>,
    pub debugname: Option<Box<str>>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
    /// Raw opcode payloads recorded during decode, for faithful re-emission of
    /// duplicate opcodes whose earlier values were overwritten in the struct.
    #[serde(skip)]
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for ObjType {
    type Context = ObjContext;

    fn new(id: u16) -> Self {
        Self {
            id,
            name: None,
            desc: None,
            model: 0,
            zoom_2d: 2000,
            xan_2d: 0,
            yan_2d: 0,
            xof_2d: 0,
            yof_2d: 0,
            zan_2d: 0,
            stackable: false,
            cost: 1,
            members: false,
            multistack: 0,
            manwear: 0,
            manwear2: 0,
            manwear3: 0,
            womanwear: 0,
            womenwear2: 0,
            womanwear3: 0,
            manhead: 0,
            manhead2: 0,
            womanhead: 0,
            womenhead2: 0,
            manwear_xoff: 0,
            manwear_yoff: 0,
            manwear_zoff: 0,
            womanwear_xoff: 0,
            womanwear_yoff: 0,
            womanwear_zoff: 0,
            options: [const { None }; 5],
            interface_options: [const { None }; 5],
            recol_s: None,
            recol_d: None,
            retex_s: None,
            retex_d: None,
            recol_palette: None,
            stockmarket: false,
            certlink: 0,
            certtemplate: 0,
            lentlink: 0,
            lenttemplate: 0,
            boughtlink: 0,
            boughttemplate: 0,
            countobj: None,
            countco: None,
            resize_x: 128,
            resize_y: 128,
            resize_z: 128,
            ambient: 0,
            contrast: 0,
            team: 0,
            dummy_obj: 0,
            picksizeshift: 0,
            cursor1: None,
            cursor2: None,
            icursor1: None,
            icursor2: None,
            quests: None,
            params: None,
            param_order: Vec::new(),
            wearpos1: -1,
            wearpos2: -1,
            wearpos3: -1,
            weight: 0.0,
            category: 0,
            tradeable: true,
            iparams: HashMap::new(),
            sparams: HashMap::new(),
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

    fn post_decode(types: &mut Vec<Self>, ctx: &Self::Context) {
        // Resolve cert/lent/bought templates, matching the client's
        // ObjTypeList loading order: gen_cert, gen_lent, gen_bought,
        // then members-gate.
        for i in 0..types.len() {
            if types[i].certtemplate != 0 {
                let template_id = types[i].certtemplate as usize;
                let link_id = types[i].certlink as usize;
                if template_id < types.len() && link_id < types.len() {
                    let template = types[template_id].clone();
                    let link = types[link_id].clone();
                    gen_cert(&mut types[i], &template, &link);
                }
            }
            if types[i].lenttemplate != 0 {
                let template_id = types[i].lenttemplate as usize;
                let link_id = types[i].lentlink as usize;
                if template_id < types.len() && link_id < types.len() {
                    let template = types[template_id].clone();
                    let link = types[link_id].clone();
                    gen_lent(&mut types[i], &template, &link);
                }
            }
            if types[i].boughttemplate != 0 {
                let template_id = types[i].boughttemplate as usize;
                let link_id = types[i].boughtlink as usize;
                if template_id < types.len() && link_id < types.len() {
                    let template = types[template_id].clone();
                    let link = types[link_id].clone();
                    gen_bought(&mut types[i], &template, &link);
                }
            }
            if !ctx.members && types[i].members {
                types[i].name = Some("Members object".into());
                types[i].stockmarket = false;
                types[i].tradeable = false;
                types[i].interface_options = [const { None }; 5];
                types[i].options = [const { None }; 5];
                types[i].team = 0;
                types[i].quests = None;
                types[i].params = None;
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

/// Copy cert (noted) fields from template + link onto `self`.
/// Matches client `ObjType.gen_cert(template, link)`.
fn gen_cert(obj: &mut ObjType, template: &ObjType, link: &ObjType) {
    obj.xan_2d = template.xan_2d;
    obj.zan_2d = template.zan_2d;
    obj.name = link.name.clone();
    obj.retex_d = template.retex_d.clone();
    obj.xof_2d = template.xof_2d;
    obj.recol_d = template.recol_d.clone();
    obj.cost = link.cost;
    obj.members = link.members;
    // Jordan's cert_link: a cert trades as its obj does.
    obj.tradeable = link.tradeable;
    obj.zoom_2d = template.zoom_2d;
    obj.model = template.model;
    obj.recol_palette = template.recol_palette.clone();
    obj.retex_s = template.retex_s.clone();
    obj.stackable = true;
    obj.recol_s = template.recol_s.clone();
    obj.yof_2d = template.yof_2d;
    obj.yan_2d = template.yan_2d;
    // Generate cert description from link name.
    if let Some(name) = &obj.name
        && let Some(ch) = name.chars().next()
    {
        let article = if "AEIOU".contains(ch) { "an" } else { "a" };
        obj.desc = Some(format!("Swap this note at any bank for {article} {name}.").into());
    }
}

/// Copy lent fields from template + link onto `self`.
/// Matches client `ObjType.gen_lent(template, link)`.
fn gen_lent(obj: &mut ObjType, template: &ObjType, link: &ObjType) {
    obj.params = link.params.clone();
    obj.womanwear_yoff = link.womanwear_yoff;
    obj.zan_2d = template.zan_2d;
    obj.retex_s = link.retex_s.clone();
    obj.womanwear3 = link.womanwear3;
    obj.name = link.name.clone();
    obj.model = template.model;
    obj.recol_s = link.recol_s.clone();
    obj.womanwear_xoff = link.womanwear_xoff;
    obj.yan_2d = template.yan_2d;
    obj.xof_2d = template.xof_2d;
    obj.womenhead2 = link.womenhead2;
    obj.recol_d = link.recol_d.clone();
    obj.xan_2d = template.xan_2d;
    obj.manhead2 = link.manhead2;
    obj.womanwear = link.womanwear;
    obj.womanwear_zoff = link.womanwear_zoff;
    obj.cost = 0;
    obj.yof_2d = template.yof_2d;
    obj.options = link.options.clone();
    obj.manwear_yoff = link.manwear_yoff;
    obj.members = link.members;
    obj.team = link.team;
    obj.recol_palette = link.recol_palette.clone();
    obj.manwear2 = link.manwear2;
    obj.manwear_xoff = link.manwear_xoff;
    obj.interface_options = [const { None }; 5];
    obj.womanhead = link.womanhead;
    obj.manwear_zoff = link.manwear_zoff;
    obj.zoom_2d = template.zoom_2d;
    obj.manwear3 = link.manwear3;
    obj.retex_d = link.retex_d.clone();
    obj.manwear = link.manwear;
    obj.womenwear2 = link.womenwear2;
    obj.manhead = link.manhead;
    obj.resize_x = link.resize_x;
    obj.resize_y = link.resize_y;
    obj.resize_z = link.resize_z;
    obj.ambient = link.ambient;
    obj.contrast = link.contrast;
}

/// Copy bought fields from template + link onto `self`.
/// Matches client `ObjType.gen_bought(template, link)`.
fn gen_bought(obj: &mut ObjType, template: &ObjType, link: &ObjType) {
    obj.xof_2d = template.xof_2d;
    obj.womanwear3 = link.womanwear3;
    obj.model = template.model;
    obj.womenhead2 = link.womenhead2;
    obj.womanwear_zoff = link.womanwear_zoff;
    obj.stackable = link.stackable;
    obj.manwear_yoff = link.manwear_yoff;
    obj.yan_2d = template.yan_2d;
    obj.womanwear = link.womanwear;
    obj.retex_d = link.retex_d.clone();
    obj.manwear_zoff = link.manwear_zoff;
    obj.manwear3 = link.manwear3;
    obj.recol_d = link.recol_d.clone();
    obj.manwear = link.manwear;
    obj.params = link.params.clone();
    obj.manwear2 = link.manwear2;
    obj.yof_2d = template.yof_2d;
    obj.womenwear2 = link.womenwear2;
    obj.members = link.members;
    obj.name = link.name.clone();
    obj.cost = 0;
    obj.womanhead = link.womanhead;
    obj.womanwear_yoff = link.womanwear_yoff;
    obj.zan_2d = template.zan_2d;
    obj.recol_s = link.recol_s.clone();
    obj.manhead = link.manhead;
    obj.manwear_xoff = link.manwear_xoff;
    obj.recol_palette = link.recol_palette.clone();
    obj.womanwear_xoff = link.womanwear_xoff;
    obj.zoom_2d = template.zoom_2d;
    obj.xan_2d = template.xan_2d;
    obj.retex_s = link.retex_s.clone();
    obj.manhead2 = link.manhead2;
    obj.resize_x = link.resize_x;
    obj.resize_y = link.resize_y;
    obj.resize_z = link.resize_z;
    obj.ambient = link.ambient;
    obj.contrast = link.contrast;
    obj.options = link.options.clone();
    obj.interface_options = link.interface_options.clone();
    obj.team = link.team;
}

impl ObjType {
    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => {
                self.model = buf.g2();
            }
            2 => {
                self.name = Some(buf.gjstr(0).into_boxed_str());
            }
            3 => {
                self.desc = Some(buf.gjstr(0).into_boxed_str());
            }
            4 => {
                self.zoom_2d = buf.g2();
            }
            5 => {
                self.xan_2d = buf.g2();
            }
            6 => {
                self.yan_2d = buf.g2();
            }
            7 => {
                self.xof_2d = buf.g2() as i16;
            }
            8 => {
                self.yof_2d = buf.g2() as i16;
            }
            11 => {
                self.stackable = true;
            }
            12 => {
                self.cost = buf.g4s();
            }
            16 => {
                self.members = true;
            }
            18 => {
                self.multistack = buf.g2();
            }
            23 => {
                self.manwear = buf.g2();
            }
            24 => {
                self.manwear2 = buf.g2();
            }
            25 => {
                self.womanwear = buf.g2();
            }
            26 => {
                self.womenwear2 = buf.g2();
            }
            30..=34 => {
                let slot = (code - 30) as usize;
                let s = buf.gjstr(0);
                self.options[slot] = if s == "Hidden" {
                    None
                } else {
                    Some(s.into_boxed_str())
                };
            }
            35..=39 => {
                let slot = (code - 35) as usize;
                let s = buf.gjstr(0);
                self.interface_options[slot] = if s == "Hidden" {
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
                // Client reads dst FIRST, then src.
                let count = buf.g1() as usize;
                let mut dst = Vec::with_capacity(count);
                let mut src = Vec::with_capacity(count);
                for _ in 0..count {
                    dst.push(buf.g2());
                    src.push(buf.g2());
                }
                self.retex_d = Some(dst.into_boxed_slice());
                self.retex_s = Some(src.into_boxed_slice());
            }
            42 => {
                let count = buf.g1() as usize;
                let mut palette = Vec::with_capacity(count);
                for _ in 0..count {
                    palette.push(buf.g1() as i8);
                }
                self.recol_palette = Some(palette.into_boxed_slice());
            }
            65 => {
                self.stockmarket = true;
            }
            78 => {
                self.manwear3 = buf.g2();
            }
            79 => {
                self.womanwear3 = buf.g2();
            }
            90 => {
                self.manhead = buf.g2();
            }
            91 => {
                self.womanhead = buf.g2();
            }
            92 => {
                self.manhead2 = buf.g2();
            }
            93 => {
                self.womenhead2 = buf.g2();
            }
            95 => {
                self.zan_2d = buf.g2();
            }
            96 => {
                self.dummy_obj = buf.g1();
            }
            97 => {
                self.certlink = buf.g2();
            }
            98 => {
                self.certtemplate = buf.g2();
            }
            100..=109 => {
                let slot = (code - 100) as usize;
                let obj = buf.g2();
                let count = buf.g2();
                let countobj = self
                    .countobj
                    .get_or_insert_with(|| vec![0u16; 10].into_boxed_slice());
                let countco = self
                    .countco
                    .get_or_insert_with(|| vec![0u16; 10].into_boxed_slice());
                countobj[slot] = obj;
                countco[slot] = count;
            }
            110 => {
                self.resize_x = buf.g2();
            }
            111 => {
                self.resize_y = buf.g2();
            }
            112 => {
                self.resize_z = buf.g2();
            }
            113 => {
                self.ambient = buf.g1() as i8;
            }
            114 => {
                self.contrast = buf.g1() as i8 as i32 * 5;
            }
            115 => {
                self.team = buf.g1();
            }
            121 => {
                self.lentlink = buf.g2();
            }
            122 => {
                self.lenttemplate = buf.g2();
            }
            125 => {
                self.manwear_xoff = buf.g1() as i8;
                self.manwear_yoff = buf.g1() as i8;
                self.manwear_zoff = buf.g1() as i8;
            }
            126 => {
                self.womanwear_xoff = buf.g1() as i8;
                self.womanwear_yoff = buf.g1() as i8;
                self.womanwear_zoff = buf.g1() as i8;
            }
            127 => {
                self.cursor1 = Some((buf.g1(), buf.g2()));
            }
            128 => {
                self.cursor2 = Some((buf.g1(), buf.g2()));
            }
            129 => {
                self.icursor1 = Some((buf.g1(), buf.g2()));
            }
            130 => {
                self.icursor2 = Some((buf.g1(), buf.g2()));
            }
            132 => {
                let count = buf.g1() as usize;
                let mut quests = Vec::with_capacity(count);
                for _ in 0..count {
                    quests.push(buf.g2());
                }
                self.quests = Some(quests.into_boxed_slice());
            }
            134 => {
                self.picksizeshift = buf.g1();
            }
            139 => {
                self.boughtlink = buf.g2();
            }
            140 => {
                self.boughttemplate = buf.g2();
            }
            // ── Server opcodes (merged into server .dat) ──
            13 => self.wearpos1 = buf.g1() as i8,
            15 => self.tradeable = false,
            14 => self.wearpos2 = buf.g1() as i8,
            27 => self.wearpos3 = buf.g1() as i8,
            75 => self.weight = buf.g2s() as f64 / 1000.0,
            94 => self.category = buf.g2(),
            201 => {
                let _respawnrate = buf.g2();
            }
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
            250 => {
                self.debugname = Some(buf.gjstr(0).into_boxed_str());
            }
            _ => {
                buf.pos = buf.data.len();
            }
        }
    }

    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or("null")
    }

    /// Certlink as Option (None when 0). The "noted" item id.
    pub fn noted_id(&self) -> Option<u16> {
        if self.certlink != 0 {
            Some(self.certlink)
        } else {
            None
        }
    }

    /// Whether this item is a note template (has a certtemplate binding).
    /// Legacy alias: `linked_note_id.is_some()`.
    pub fn is_noted(&self) -> bool {
        self.certtemplate != 0
    }

    /// Certtemplate as Option (None when 0). `Some` means this item IS a note.
    pub fn linked_note_id(&self) -> Option<u16> {
        if self.certtemplate != 0 {
            Some(self.certtemplate)
        } else {
            None
        }
    }

    /// Get an integer param value from the server iparams map.
    pub fn get_iparam(&self, key: u32) -> Option<i32> {
        self.iparams.get(&key).copied()
    }

    /// Get a string param value from the server sparams map.
    pub fn get_sparam(&self, key: u32) -> Option<&str> {
        self.sparams.get(&key).map(|s| s.as_str())
    }
}
