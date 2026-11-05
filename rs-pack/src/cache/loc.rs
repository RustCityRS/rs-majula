#[cfg(since_647)]
use super::decode::read_id;
#[cfg(since_289)]
use super::decode::read_transforms;
use super::decode::{read_array, read_pairs};
use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{ForceApproach, LocShape, MultiVar};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type LocTypeProvider = TypeProvider<LocType>;

#[cfg_attr(since_647, derive(Debug, Clone, serde::Serialize))]
pub struct LocModelShape {
    #[cfg(before_647)]
    pub model: u16,
    #[cfg(since_647)]
    pub models: Box<[u16]>,
    pub shape: u8,
}

pub struct LocType {
    pub id: u16,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    pub blockrange: bool,
    pub active: Option<bool>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub category: Option<u16>,
    pub forceapproach: ForceApproach,
    pub multivar: Option<MultiVar>,
    pub multiloc: Option<Box<[i32]>>,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl LocType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }

    #[cfg(since_289)]
    pub fn multi_loc(&self, var_value: i32) -> Option<u16> {
        let Some(multi) = self.multiloc.as_deref() else {
            return Some(self.id);
        };
        let pick = multi
            .get(var_value as usize)
            .copied()
            .filter(|&id| id != -1);
        #[cfg(since_647)]
        let pick = pick.or_else(|| multi.last().copied());
        pick.and_then(|id| u16::try_from(id).ok())
    }
}

impl From<LocTypeRaw> for LocType {
    fn from(raw: LocTypeRaw) -> Self {
        let op = raw.op;
        #[cfg(since_647)]
        let op = {
            let mut op = op;
            for (index, members_op) in raw.members_op.into_iter().enumerate() {
                if members_op.is_some() {
                    op.get_or_insert_with(|| Box::new([const { None }; 5]))[index] = members_op;
                }
            }
            op
        };
        LocType {
            id: raw.id,
            name: raw.name,
            desc: raw.desc,
            width: raw.width,
            length: raw.length,
            blockwalk: raw.blockwalk,
            blockrange: raw.blockrange,
            active: raw.active.and_then(|value| match value {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            }),
            op,
            category: raw.category,
            forceapproach: raw.forceapproach,
            #[cfg(all(since_289, before_647))]
            multivar: raw.multivar.map(MultiVar::Varbit),
            #[cfg(since_647)]
            multivar: u16::try_from(raw.transform_varbit)
                .ok()
                .map(MultiVar::Varbit)
                .or_else(|| u16::try_from(raw.transform_varp).ok().map(MultiVar::Varp)),
            #[cfg(before_289)]
            multivar: None,
            #[cfg(since_289)]
            multiloc: raw.multiloc,
            #[cfg(before_289)]
            multiloc: None,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

#[cfg_attr(since_647, derive(Debug, Clone, serde::Serialize))]
pub struct LocTypeRaw {
    pub id: u16,
    pub models: Option<Box<[LocModelShape]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    #[cfg(since_647)]
    pub hd_models: Option<Box<[LocModelShape]>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    #[cfg(since_647)]
    pub interact_type: i32,
    pub blockrange: bool,
    pub active: Option<u8>,
    pub hillskew: u8,
    pub sharelight: bool,
    pub occlude: bool,
    pub anim: Option<u16>,
    pub hasalpha: bool,
    pub wallwidth: u8,
    pub ambient: i8,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub contrast: i8,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub recol_palette: Option<Box<[i8]>>,
    pub mapfunction: Option<u16>,
    pub category: Option<u16>,
    pub mirror: bool,
    pub shadow: bool,
    pub resizex: u16,
    pub resizey: u16,
    pub resizez: u16,
    pub mapscene: Option<u16>,
    pub forceapproach: ForceApproach,
    pub offsetx: i16,
    pub offsety: i16,
    pub offsetz: i16,
    pub forcedecor: bool,
    #[cfg(since_245_2)]
    pub breakroutefinding: bool,
    #[cfg(since_254)]
    pub raiseobject: Option<u8>,
    #[cfg(all(since_289, before_647))]
    pub multivar: Option<u16>,
    #[cfg(since_647)]
    pub transform_varbit: i32,
    #[cfg(since_647)]
    pub transform_varp: i32,
    #[cfg(since_289)]
    pub multiloc: Option<Box<[i32]>>,
    #[cfg(since_647)]
    pub ambient_sfx_id: i32,
    #[cfg(since_647)]
    pub ambient_sfx_range: u8,
    #[cfg(since_647)]
    pub sfx_random_pri: i32,
    #[cfg(since_647)]
    pub sfx_random_sec: i32,
    #[cfg(since_647)]
    pub sfx_random_ids: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub hillskew_param: i32,
    #[cfg(since_647)]
    pub hide_minimap: bool,
    #[cfg(since_647)]
    pub minimap_visible: bool,
    #[cfg(since_647)]
    pub blocks_npcs: bool,
    #[cfg(since_647)]
    pub unknown_90: bool,
    #[cfg(since_647)]
    pub members_only: bool,
    #[cfg(since_647)]
    pub unknown_97: bool,
    #[cfg(since_647)]
    pub unknown_98: bool,
    #[cfg(since_647)]
    pub cursor_op1: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub cursor_op2: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub msi_id: i32,
    #[cfg(since_647)]
    pub interaction_side_flags: u8,
    #[cfg(since_647)]
    pub unknown_105: bool,
    #[cfg(since_647)]
    pub random_anim_ids: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub random_anim_weights: Option<Box<[u8]>>,
    #[cfg(since_647)]
    pub map_element: i32,
    #[cfg(since_647)]
    pub members_op: [Option<Box<str>>; 5],
    #[cfg(since_647)]
    pub quest_ids: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub op163: [i8; 4],
    #[cfg(since_647)]
    pub op164: i16,
    #[cfg(since_647)]
    pub op165: i16,
    #[cfg(since_647)]
    pub op166: i16,
    #[cfg(since_647)]
    pub op167: i32,
    #[cfg(since_647)]
    pub unknown_168: bool,
    #[cfg(since_647)]
    pub unknown_169: bool,
    #[cfg(since_647)]
    pub op170: i32,
    #[cfg(since_647)]
    pub op171: i32,
    #[cfg(since_647)]
    pub op173: (u16, u16),
    #[cfg(since_647)]
    pub unknown_177: bool,
    #[cfg(since_647)]
    pub op178: u8,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
    #[cfg(since_647)]
    pub param_order: Vec<i32>,
    #[cfg_attr(since_647, serde(skip))]
    pub opcode_order: Vec<u8>,
    #[cfg(since_647)]
    #[serde(skip)]
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
}

impl CacheType for LocTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        LocTypeRaw {
            id,
            models: None,
            name: None,
            desc: None,
            #[cfg(since_647)]
            hd_models: None,
            width: 1,
            length: 1,
            blockwalk: true,
            #[cfg(since_647)]
            interact_type: 2,
            blockrange: true,
            active: None,
            hillskew: 0,
            sharelight: false,
            occlude: false,
            anim: None,
            hasalpha: false,
            wallwidth: 16,
            ambient: 0,
            op: None,
            contrast: 0,
            recol_s: None,
            recol_d: None,
            #[cfg(since_647)]
            retex_s: None,
            #[cfg(since_647)]
            retex_d: None,
            #[cfg(since_647)]
            recol_palette: None,
            mapfunction: None,
            category: None,
            mirror: false,
            shadow: true,
            resizex: 128,
            resizey: 128,
            resizez: 128,
            mapscene: None,
            forceapproach: ForceApproach::None,
            offsetx: 0,
            offsety: 0,
            offsetz: 0,
            forcedecor: false,
            #[cfg(since_245_2)]
            breakroutefinding: false,
            #[cfg(since_254)]
            raiseobject: None,
            #[cfg(all(since_289, before_647))]
            multivar: None,
            #[cfg(since_647)]
            transform_varbit: -1,
            #[cfg(since_647)]
            transform_varp: -1,
            #[cfg(since_289)]
            multiloc: None,
            #[cfg(since_647)]
            ambient_sfx_id: 0,
            #[cfg(since_647)]
            ambient_sfx_range: 0,
            #[cfg(since_647)]
            sfx_random_pri: 0,
            #[cfg(since_647)]
            sfx_random_sec: 0,
            #[cfg(since_647)]
            sfx_random_ids: None,
            #[cfg(since_647)]
            hillskew_param: -1,
            #[cfg(since_647)]
            hide_minimap: false,
            #[cfg(since_647)]
            minimap_visible: true,
            #[cfg(since_647)]
            blocks_npcs: true,
            #[cfg(since_647)]
            unknown_90: false,
            #[cfg(since_647)]
            members_only: false,
            #[cfg(since_647)]
            unknown_97: false,
            #[cfg(since_647)]
            unknown_98: false,
            #[cfg(since_647)]
            cursor_op1: None,
            #[cfg(since_647)]
            cursor_op2: None,
            #[cfg(since_647)]
            msi_id: -1,
            #[cfg(since_647)]
            interaction_side_flags: 255,
            #[cfg(since_647)]
            unknown_105: false,
            #[cfg(since_647)]
            random_anim_ids: None,
            #[cfg(since_647)]
            random_anim_weights: None,
            #[cfg(since_647)]
            map_element: -1,
            #[cfg(since_647)]
            members_op: [const { None }; 5],
            #[cfg(since_647)]
            quest_ids: None,
            #[cfg(since_647)]
            op163: [0; 4],
            #[cfg(since_647)]
            op164: 0,
            #[cfg(since_647)]
            op165: 0,
            #[cfg(since_647)]
            op166: 0,
            #[cfg(since_647)]
            op167: 0,
            #[cfg(since_647)]
            unknown_168: false,
            #[cfg(since_647)]
            unknown_169: false,
            #[cfg(since_647)]
            op170: 0,
            #[cfg(since_647)]
            op171: 0,
            #[cfg(since_647)]
            op173: (0, 0),
            #[cfg(since_647)]
            unknown_177: false,
            #[cfg(since_647)]
            op178: 0,
            params: None,
            debugname: None,
            #[cfg(since_647)]
            param_order: Vec::new(),
            opcode_order: Vec::new(),
            #[cfg(since_647)]
            raw_opcodes: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            #[cfg(since_647)]
            let start = buf.pos;
            self.opcode_order.push(code);
            match code {
                1 => self.models = Some(read_models(buf, code)),
                2 => self.name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                #[cfg(since_254)]
                5 => {
                    self.models = Some(read_models(buf, code));
                    #[cfg(since_647)]
                    {
                        self.hd_models = Some(read_models(buf, code));
                    }
                }
                14 => self.width = buf.g1(),
                15 => self.length = buf.g1(),
                17 => {
                    self.blockwalk = false;
                    #[cfg(since_647)]
                    {
                        self.interact_type = 0;
                    }
                }
                18 => self.blockrange = false,
                19 => {
                    let active = buf.g1();
                    self.active = Some(if cfg!(before_647) {
                        (active == 1) as u8
                    } else {
                        active
                    });
                }
                21 => self.hillskew = 1,
                22 => self.sharelight = true,
                23 => self.occlude = true,
                24 => {
                    let anim = buf.g2();
                    self.anim = (cfg!(before_647) || anim != u16::MAX).then_some(anim);
                }
                25 => self.hasalpha = true,
                #[cfg(since_647)]
                27 => self.interact_type = 1,
                28 => self.wallwidth = buf.g1(),
                29 => self.ambient = buf.g1s(),
                30..=34 => {
                    let op = buf.gjstr(STRING_TERMINATOR);
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(op.into_boxed_str());
                }
                39 => self.contrast = buf.g1s(),
                40 => read_pairs(buf, &mut self.recol_s, &mut self.recol_d),
                #[cfg(since_647)]
                41 => read_pairs(buf, &mut self.retex_s, &mut self.retex_d),
                #[cfg(since_647)]
                42 => self.recol_palette = Some(read_array(buf, Packet::g1s)),
                #[cfg(before_647)]
                60 => self.mapfunction = Some(buf.g2()),
                61 => self.category = Some(buf.g2()),
                62 => self.mirror = true,
                64 => self.shadow = false,
                65 => self.resizex = buf.g2(),
                66 => self.resizey = buf.g2(),
                67 => self.resizez = buf.g2(),
                #[cfg(before_647)]
                68 => self.mapscene = Some(buf.g2()),
                69 => self.forceapproach = ForceApproach::try_from(buf.g1()).unwrap(),
                70 => self.offsetx = buf.g2s(),
                71 => self.offsety = buf.g2s(),
                72 => self.offsetz = buf.g2s(),
                73 => self.forcedecor = true,
                #[cfg(since_245_2)]
                74 => self.breakroutefinding = true,
                #[cfg(since_254)]
                75 => self.raiseobject = Some(buf.g1()),
                #[cfg(since_289)]
                77 => self.decode_multiloc(buf, false),
                #[cfg(since_647)]
                78 => {
                    self.ambient_sfx_id = buf.g2() as i32;
                    self.ambient_sfx_range = buf.g1();
                }
                #[cfg(since_647)]
                79 => {
                    self.sfx_random_pri = buf.g2() as i32;
                    self.sfx_random_sec = buf.g2() as i32;
                    self.ambient_sfx_range = buf.g1();
                    self.sfx_random_ids = Some(read_array(buf, Packet::g2));
                }
                #[cfg(since_647)]
                81 => {
                    self.hillskew = 2;
                    self.hillskew_param = (buf.g1() as i32) * 256;
                }
                #[cfg(since_647)]
                82 => self.hide_minimap = true,
                #[cfg(since_647)]
                88 => self.minimap_visible = false,
                #[cfg(since_647)]
                89 => self.blocks_npcs = false,
                #[cfg(since_647)]
                90 => self.unknown_90 = true,
                #[cfg(since_647)]
                91 => self.members_only = true,
                #[cfg(since_647)]
                92 => self.decode_multiloc(buf, true),
                #[cfg(since_647)]
                93 => {
                    self.hillskew = 3;
                    self.hillskew_param = buf.g2() as i32;
                }
                #[cfg(since_647)]
                94 => self.hillskew = 4,
                #[cfg(since_647)]
                95 => {
                    self.hillskew = 5;
                    self.hillskew_param = buf.g2s() as i32;
                }
                #[cfg(since_647)]
                97 => self.unknown_97 = true,
                #[cfg(since_647)]
                98 => self.unknown_98 = true,
                #[cfg(since_647)]
                99 => self.cursor_op1 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                100 => self.cursor_op2 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                101 => self.mapfunction = Some(buf.g1() as u16),
                #[cfg(since_647)]
                102 => self.msi_id = buf.g2() as i32,
                #[cfg(since_647)]
                103 => self.occlude = false,
                #[cfg(since_647)]
                104 => self.interaction_side_flags = buf.g1(),
                #[cfg(since_647)]
                105 => self.unknown_105 = true,
                #[cfg(since_647)]
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
                #[cfg(since_647)]
                107 => self.map_element = buf.g2() as i32,
                #[cfg(since_647)]
                150..=154 => {
                    let slot = (code - 150) as usize;
                    let s = buf.gjstr(STRING_TERMINATOR);
                    self.members_op[slot] = if s.is_empty() {
                        None
                    } else {
                        Some(s.into_boxed_str())
                    };
                }
                #[cfg(since_647)]
                160 => self.quest_ids = Some(read_array(buf, Packet::g2)),
                #[cfg(since_647)]
                162 => {
                    self.hillskew = 3;
                    self.hillskew_param = buf.g4s();
                }
                #[cfg(since_647)]
                163 => self.op163 = [buf.g1s(), buf.g1s(), buf.g1s(), buf.g1s()],
                #[cfg(since_647)]
                164 => self.op164 = buf.g2s(),
                #[cfg(since_647)]
                165 => self.op165 = buf.g2s(),
                #[cfg(since_647)]
                166 => self.op166 = buf.g2s(),
                #[cfg(since_647)]
                167 => self.op167 = buf.g2() as i32,
                #[cfg(since_647)]
                168 => self.unknown_168 = true,
                #[cfg(since_647)]
                169 => self.unknown_169 = true,
                #[cfg(since_647)]
                170 => self.op170 = buf.gsmart1or2(),
                #[cfg(since_647)]
                171 => self.op171 = buf.gsmart1or2(),
                #[cfg(since_647)]
                173 => self.op173 = (buf.g2(), buf.g2()),
                #[cfg(since_647)]
                177 => self.unknown_177 = true,
                #[cfg(since_647)]
                178 => self.op178 = buf.g1(),
                249 => {
                    ParamType::decode_params(
                        buf,
                        self.params
                            .get_or_insert_with(|| Box::new(FxHashMap::default())),
                        #[cfg(since_647)]
                        &mut self.param_order,
                    );
                }
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("loc", code),
            }
            #[cfg(since_647)]
            self.raw_opcodes
                .push((code, buf.data[start..buf.pos].to_vec()));
        }
    }

    fn post_decode(locs: &mut Vec<Self>, _ctx: &Self::Context) {
        for loc in locs.iter_mut() {
            if loc.active.is_none() {
                let active = loc.models.as_deref().is_some_and(|models| {
                    (cfg!(before_647) || models.len() == 1)
                        && models
                            .first()
                            .is_some_and(|m| m.shape == LocShape::CentrepieceStraight as u8)
                }) || loc
                    .op
                    .as_deref()
                    .is_some_and(|op| cfg!(before_647) || op.iter().any(Option::is_some));
                loc.active = Some(active as u8);
            }
            #[cfg(since_647)]
            if loc.raiseobject.is_none() {
                loc.raiseobject = Some((loc.interact_type != 0) as u8);
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl LocTypeRaw {
    #[cfg(since_289)]
    fn decode_multiloc(&mut self, buf: &mut Packet, _has_default: bool) {
        #[cfg(before_647)]
        {
            self.multivar = Some(buf.g2());
        }
        #[cfg(since_647)]
        {
            self.transform_varbit = read_id(buf);
            self.transform_varp = read_id(buf);
        }
        #[cfg(before_647)]
        let default = None;
        #[cfg(since_647)]
        let default = Some(if _has_default { read_id(buf) } else { -1 });
        self.multiloc = Some(read_transforms(buf, default));
    }

    #[cfg(since_647)]
    pub(crate) fn merge(&mut self, overlay: Self) {
        if overlay.debugname.is_some() {
            self.debugname = overlay.debugname;
        }
        if overlay.category.is_some() {
            self.category = overlay.category;
        }
        if overlay.desc.is_some() {
            self.desc = overlay.desc;
        }
        if let Some(op) = overlay.op {
            let base_op = self.op.get_or_insert_with(|| Box::new([const { None }; 5]));
            for (base_op, op) in base_op.iter_mut().zip(op) {
                if op.is_some() {
                    *base_op = op;
                }
            }
        }
        if let Some(params) = overlay.params {
            self.params
                .get_or_insert_with(|| Box::new(FxHashMap::default()))
                .extend(*params);
        }
    }
}

#[cfg(all(test, since_647))]
mod multiloc_tests {
    use super::*;

    /// 2011's Mill Lane flour bin (36880): on varp 695, 0 the empty bin
    /// (36879), 1-30 the full one (36878); its default is none.
    fn flour_bin() -> LocType {
        let mut multi = vec![36879, 36878, 36878, 36878];
        multi.push(-1);
        LocType::from(LocTypeRaw::new(36880)).with_multi(MultiVar::Varp(695), multi)
    }

    impl LocType {
        fn with_multi(mut self, var: MultiVar, multi: Vec<i32>) -> Self {
            self.multivar = Some(var);
            self.multiloc = Some(multi.into_boxed_slice());
            self
        }
    }

    #[test]
    fn a_multiloc_is_the_loc_its_var_picks() {
        let bin = flour_bin();
        assert_eq!(bin.multi_loc(0), Some(36879), "empty");
        assert_eq!(bin.multi_loc(1), Some(36878), "full");
        assert_eq!(bin.multi_loc(3), Some(36878), "full");
    }

    /// Out of range, or -1, the client takes the default: none here.
    #[test]
    fn past_its_locs_a_multiloc_is_its_default() {
        let bin = flour_bin();
        assert_eq!(bin.multi_loc(4), None);
        assert_eq!(bin.multi_loc(-1), None);
    }

    #[test]
    fn a_loc_that_is_no_multiloc_is_itself() {
        let loc = LocType::from(LocTypeRaw::new(2718));
        assert_eq!(loc.multi_loc(7), Some(2718));
    }
}

fn read_models(buf: &mut Packet, _code: u8) -> Box<[LocModelShape]> {
    read_array(buf, |buf| {
        #[cfg(before_647)]
        let model = buf.g2();
        #[cfg(before_647)]
        let shape = if _code == 1 {
            buf.g1()
        } else {
            LocShape::CentrepieceStraight as u8
        };
        #[cfg(since_647)]
        let shape = buf.g1();
        #[cfg(since_647)]
        let models = read_array(buf, Packet::g2);
        LocModelShape {
            #[cfg(before_647)]
            model,
            #[cfg(since_647)]
            models,
            shape,
        }
    })
}
