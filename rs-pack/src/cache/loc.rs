use super::decode::{read_array, read_id, read_pairs};
#[cfg(since_289)]
use super::decode::{read_big_id, read_transforms};
use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{ForceApproach, LocShape, MultiVar};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;
use rustc_hash::FxHashMap;

pub type LocTypeProvider = TypeProvider<LocType>;

#[cfg_attr(since_647, derive(Debug, Clone, PartialEq))]
pub struct LocModelShape {
    #[cfg(before_647)]
    pub model: TypeId,
    #[cfg(since_647)]
    pub models: Box<[TypeId]>,
    pub shape: u8,
}

pub struct LocType {
    pub id: TypeId,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    pub blockrange: bool,
    pub active: Option<bool>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub category: Option<TypeId>,
    pub forceapproach: ForceApproach,
    pub breakroutefinding: bool,
    pub multivar: Option<MultiVar>,
    pub multiloc: Option<Box<[TypeId]>>,
    pub params: Option<Box<FxHashMap<TypeId, ParamValue>>>,
    pub debugname: Box<str>,
    #[cfg(since_647)]
    look: Option<Box<Look>>,
}

#[cfg(since_647)]
struct Look {
    animated: bool,
    shapes: bool,
    two_model_sets: bool,
    models: Box<[LocModelShape]>,
    recol: Box<[u16]>,
    retex: Box<[u16]>,
}

/// What a 647 LocMerge copies from a loc type or checks on the loc it merges.
/// The client only merges a loc it builds `animated`, looks every merge's
/// shape up in the model list when it has `shapes`, and picks one of
/// `two_model_sets` by a detail option the server never sees.
#[cfg(since_647)]
#[derive(Default)]
pub struct LocLook<'a> {
    pub animated: bool,
    pub shapes: bool,
    pub two_model_sets: bool,
    models: &'a [LocModelShape],
    pub recol: &'a [u16],
    pub retex: &'a [u16],
}

#[cfg(since_647)]
impl<'a> LocLook<'a> {
    pub fn models(&self, shape: LocShape) -> Option<&'a [TypeId]> {
        self.models
            .iter()
            .find(|group| group.shape == shape as u8)
            .map(|group| &*group.models)
    }
}

impl LocType {
    #[cfg(since_647)]
    pub fn look(&self) -> LocLook<'_> {
        let Some(look) = self.look.as_deref() else {
            return LocLook::default();
        };
        LocLook {
            animated: look.animated,
            shapes: look.shapes,
            two_model_sets: look.two_model_sets,
            models: &look.models,
            recol: &look.recol,
            retex: &look.retex,
        }
    }

    pub fn multi_loc(&self, var_value: i32) -> Option<TypeId> {
        let Some(multi) = self.multiloc.as_deref() else {
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

#[cfg(since_647)]
fn pack_look(raw: &LocTypeRaw) -> Option<Box<Look>> {
    let animated =
        raw.anim.is_some() || raw.random_anim_ids.is_some() || raw.hasparticles || raw.unknown_177;
    let two_model_sets = raw.hd_models.is_some() && raw.hd_models != raw.models;
    let recol = raw.recol_d.clone().unwrap_or_default();
    let retex = raw.retex_d.clone().unwrap_or_default();
    if !animated && !two_model_sets && raw.models.is_none() && recol.is_empty() && retex.is_empty()
    {
        return None;
    }
    Some(Box::new(Look {
        animated,
        shapes: raw.models.is_some(),
        two_model_sets,
        models: raw.models.clone().unwrap_or_default(),
        recol,
        retex,
    }))
}

impl From<LocTypeRaw> for LocType {
    fn from(raw: LocTypeRaw) -> Self {
        #[cfg(since_647)]
        let look = pack_look(&raw);
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
            #[cfg(since_245_2)]
            breakroutefinding: raw.breakroutefinding,
            #[cfg(before_245_2)]
            breakroutefinding: false,
            multivar: MultiVar::new(raw.transform_varpbit, raw.transform_varp),
            multiloc: raw.multiloc,
            params: raw.params,
            debugname: raw.debugname,
            #[cfg(since_647)]
            look,
        }
    }
}

#[cfg_attr(since_647, derive(Debug, Clone))]
pub struct LocTypeRaw {
    pub id: TypeId,
    pub models: Option<Box<[LocModelShape]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    #[cfg(since_530)]
    pub hd_models: Option<Box<[LocModelShape]>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    #[cfg(since_530)]
    pub interact_type: i32,
    pub blockrange: bool,
    pub active: Option<u8>,
    pub hillskew: u8,
    pub sharelight: bool,
    pub occlude: bool,
    pub anim: Option<TypeId>,
    pub hasalpha: bool,
    pub wallwidth: u8,
    pub ambient: i8,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub contrast: i8,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_530)]
    pub recol_palette: Option<Box<[i8]>>,
    #[cfg(since_742)]
    pub recolindices: Option<u16>,
    #[cfg(since_742)]
    pub retexindices: Option<u16>,
    pub mapfunction: Option<TypeId>,
    pub category: Option<TypeId>,
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
    pub transform_varpbit: TypeId,
    pub transform_varp: TypeId,
    pub multiloc: Option<Box<[TypeId]>>,
    #[cfg(since_530)]
    pub background_sound: Option<TypeId>,
    #[cfg(since_530)]
    pub background_sound_range: u8,
    #[cfg(since_530)]
    pub random_sound_min: i32,
    #[cfg(since_530)]
    pub random_sound_max: i32,
    #[cfg(since_530)]
    pub random_sounds: Option<Box<[TypeId]>>,
    #[cfg(since_530)]
    pub hillskew_param: Option<i32>,
    #[cfg(since_530)]
    pub hide_minimap: bool,
    #[cfg(since_530)]
    pub minimap_visible: bool,
    #[cfg(since_530)]
    pub use_random_anim_frame: bool,
    #[cfg(since_530)]
    pub unknown_90: bool,
    #[cfg(since_530)]
    pub members_only: bool,
    #[cfg(all(since_530, before_647))]
    pub unknown_96: bool,
    #[cfg(since_530)]
    pub msirotate: bool,
    #[cfg(since_530)]
    pub hasparticles: bool,
    #[cfg(all(since_530, before_742))]
    pub cursor_op1: Option<(u8, TypeId)>,
    #[cfg(all(since_530, before_742))]
    pub cursor_op2: Option<(u8, TypeId)>,
    #[cfg(since_530)]
    pub msi_rotate_offset: Option<u8>,
    #[cfg(since_530)]
    pub msi_id: Option<TypeId>,
    #[cfg(since_647)]
    pub background_sound_volume: u8,
    #[cfg(since_647)]
    pub msimirror: bool,
    #[cfg(since_647)]
    pub random_anim_ids: Option<Box<[TypeId]>>,
    #[cfg(since_647)]
    pub random_anim_weights: Option<Box<[u8]>>,
    #[cfg(since_647)]
    pub map_element: Option<TypeId>,
    #[cfg(since_647)]
    pub members_op: [Option<Box<str>>; 5],
    #[cfg(since_647)]
    pub quest_ids: Option<Box<[TypeId]>>,
    #[cfg(since_647)]
    pub tint: [i8; 4],
    #[cfg(since_647)]
    pub postoffsetx: i16,
    #[cfg(since_647)]
    pub postoffsety: i16,
    #[cfg(since_647)]
    pub postoffsetz: i16,
    #[cfg(since_647)]
    pub raise_object_ground_decor: i32,
    #[cfg(since_647)]
    pub bgsound_vorbis: bool,
    #[cfg(since_647)]
    pub randomsound_vorbis: bool,
    #[cfg(since_647)]
    pub op170: i32,
    #[cfg(since_647)]
    pub op171: i32,
    #[cfg(since_647)]
    pub background_sound_rate: (u16, u16),
    #[cfg(since_647)]
    pub unknown_177: bool,
    #[cfg(since_647)]
    pub background_sound_dropoff_range: u8,
    #[cfg(since_742)]
    pub antimacro: bool,
    #[cfg(since_742)]
    pub cursor: [Option<TypeId>; 6],
    pub params: Option<Box<FxHashMap<TypeId, ParamValue>>>,
    debugname: Box<str>,
}

impl CacheType for LocTypeRaw {
    type Context = ();

    fn new(id: TypeId) -> Self {
        LocTypeRaw {
            id,
            models: None,
            name: None,
            desc: None,
            #[cfg(since_530)]
            hd_models: None,
            width: 1,
            length: 1,
            blockwalk: true,
            #[cfg(since_530)]
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
            #[cfg(since_530)]
            retex_s: None,
            #[cfg(since_530)]
            retex_d: None,
            #[cfg(since_530)]
            recol_palette: None,
            #[cfg(since_742)]
            recolindices: None,
            #[cfg(since_742)]
            retexindices: None,
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
            transform_varpbit: TypeId::NULL,
            transform_varp: TypeId::NULL,
            multiloc: None,
            #[cfg(since_530)]
            background_sound: None,
            #[cfg(since_530)]
            background_sound_range: 0,
            #[cfg(since_530)]
            random_sound_min: 0,
            #[cfg(since_530)]
            random_sound_max: 0,
            #[cfg(since_530)]
            random_sounds: None,
            #[cfg(since_530)]
            hillskew_param: None,
            #[cfg(since_530)]
            hide_minimap: false,
            #[cfg(since_530)]
            minimap_visible: true,
            #[cfg(since_530)]
            use_random_anim_frame: true,
            #[cfg(since_530)]
            unknown_90: false,
            #[cfg(since_530)]
            members_only: false,
            #[cfg(all(since_530, before_647))]
            unknown_96: false,
            #[cfg(since_530)]
            msirotate: false,
            #[cfg(since_530)]
            hasparticles: false,
            #[cfg(all(since_530, before_742))]
            cursor_op1: None,
            #[cfg(all(since_530, before_742))]
            cursor_op2: None,
            #[cfg(since_530)]
            msi_rotate_offset: None,
            #[cfg(since_530)]
            msi_id: None,
            #[cfg(since_647)]
            background_sound_volume: 255,
            #[cfg(since_647)]
            msimirror: false,
            #[cfg(since_647)]
            random_anim_ids: None,
            #[cfg(since_647)]
            random_anim_weights: None,
            #[cfg(since_647)]
            map_element: None,
            #[cfg(since_647)]
            members_op: [const { None }; 5],
            #[cfg(since_647)]
            quest_ids: None,
            #[cfg(since_647)]
            tint: [0; 4],
            #[cfg(since_647)]
            postoffsetx: 0,
            #[cfg(since_647)]
            postoffsety: 0,
            #[cfg(since_647)]
            postoffsetz: 0,
            #[cfg(since_647)]
            raise_object_ground_decor: 0,
            #[cfg(since_647)]
            bgsound_vorbis: false,
            #[cfg(since_647)]
            randomsound_vorbis: false,
            #[cfg(since_647)]
            op170: 960,
            #[cfg(since_647)]
            op171: 0,
            #[cfg(since_647)]
            background_sound_rate: (256, 256),
            #[cfg(since_647)]
            unknown_177: false,
            #[cfg(since_647)]
            background_sound_dropoff_range: 0,
            #[cfg(since_742)]
            antimacro: false,
            #[cfg(since_742)]
            cursor: [None; 6],
            params: None,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.set_models(read_models(buf, code)),
            2 => self.name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
            3 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
            #[cfg(since_254)]
            5 => {
                self.set_models(read_models(buf, code));
                #[cfg(since_647)]
                {
                    self.hd_models = Some(read_models(buf, code));
                }
            }
            14 => self.width = buf.g1(),
            15 => self.length = buf.g1(),
            17 => {
                self.blockwalk = false;
                #[cfg(since_530)]
                {
                    self.interact_type = 0;
                }
            }
            18 => self.blockrange = false,
            19 => {
                let active = buf.g1();
                self.active = Some(if cfg!(before_530) {
                    (active == 1) as u8
                } else {
                    active
                });
            }
            21 => self.hillskew = 1,
            22 => self.sharelight = true,
            23 => self.occlude = true,
            #[cfg(before_742)]
            24 => {
                let anim = read_id(buf);
                self.anim = (cfg!(before_530) || !anim.is_null()).then_some(anim);
            }
            #[cfg(since_742)]
            24 => self.anim = read_big_id(buf).non_null(),
            25 => self.hasalpha = true,
            #[cfg(since_530)]
            27 => self.interact_type = 1,
            28 => self.wallwidth = buf.g1(),
            29 => self.ambient = buf.g1s(),
            30..=34 => {
                let op = buf.gjstr(STRING_TERMINATOR);
                self.op
                    .get_or_insert_with(|| vec![None; 5].into_boxed_slice())[code as usize - 30] =
                    Some(op.into_boxed_str());
            }
            39 => self.contrast = buf.g1s(),
            40 => read_pairs(buf, &mut self.recol_s, &mut self.recol_d),
            #[cfg(since_530)]
            41 => read_pairs(buf, &mut self.retex_s, &mut self.retex_d),
            #[cfg(since_530)]
            42 => self.recol_palette = Some(read_array(buf, Packet::g1s)),
            #[cfg(since_742)]
            44 => self.recolindices = Some(buf.g2()),
            #[cfg(since_742)]
            45 => self.retexindices = Some(buf.g2()),
            #[cfg(before_647)]
            60 => self.mapfunction = Some(read_id(buf)),
            61 => self.category = Some(read_id(buf)),
            62 => self.mirror = true,
            64 => self.shadow = false,
            65 => self.resizex = buf.g2(),
            66 => self.resizey = buf.g2(),
            67 => self.resizez = buf.g2(),
            #[cfg(before_530)]
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
            #[cfg(since_530)]
            78 => {
                self.background_sound = Some(read_id(buf));
                self.background_sound_range = buf.g1();
            }
            #[cfg(since_530)]
            79 => {
                self.random_sound_min = buf.g2() as i32;
                self.random_sound_max = buf.g2() as i32;
                self.background_sound_range = buf.g1();
                self.random_sounds = Some(read_array(buf, read_id));
            }
            #[cfg(since_530)]
            81 => {
                self.hillskew = 2;
                self.hillskew_param = Some((buf.g1() as i32) * 256);
            }
            #[cfg(since_530)]
            82 => self.hide_minimap = true,
            #[cfg(since_530)]
            88 => self.minimap_visible = false,
            #[cfg(since_530)]
            89 => self.use_random_anim_frame = false,
            #[cfg(since_530)]
            90 => self.unknown_90 = true,
            #[cfg(since_530)]
            91 => self.members_only = true,
            #[cfg(since_530)]
            92 => self.decode_multiloc(buf, true),
            #[cfg(since_530)]
            93 => {
                self.hillskew = 3;
                self.hillskew_param = Some(buf.g2() as i32);
            }
            #[cfg(since_530)]
            94 => self.hillskew = 4,
            #[cfg(all(since_530, before_647))]
            95 => self.hillskew = 5,
            #[cfg(since_647)]
            95 => {
                self.hillskew = 5;
                let param = buf.g2s() as i32;
                self.hillskew_param = (param != -1).then_some(param);
            }
            #[cfg(all(since_530, before_647))]
            96 => self.unknown_96 = true,
            #[cfg(since_530)]
            97 => self.msirotate = true,
            #[cfg(since_530)]
            98 => self.hasparticles = true,
            #[cfg(all(since_530, before_742))]
            99 => self.cursor_op1 = Some((buf.g1(), read_id(buf))),
            #[cfg(all(since_530, before_742))]
            100 => self.cursor_op2 = Some((buf.g1(), read_id(buf))),
            #[cfg(since_530)]
            101 => self.msi_rotate_offset = Some(buf.g1()),
            #[cfg(since_530)]
            102 => self.msi_id = Some(read_id(buf)),
            #[cfg(since_647)]
            103 => self.occlude = false,
            #[cfg(since_647)]
            104 => self.background_sound_volume = buf.g1(),
            #[cfg(since_647)]
            105 => self.msimirror = true,
            #[cfg(since_647)]
            106 => {
                let count = buf.g1() as usize;
                let mut ids = Vec::with_capacity(count);
                let mut weights = Vec::with_capacity(count);
                for _ in 0..count {
                    ids.push(read_big_id(buf));
                    weights.push(buf.g1());
                }
                self.random_anim_ids = Some(ids.into_boxed_slice());
                self.random_anim_weights = Some(weights.into_boxed_slice());
            }
            #[cfg(since_647)]
            107 => self.map_element = Some(read_id(buf)),
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
            160 => self.quest_ids = Some(read_array(buf, read_id)),
            #[cfg(since_647)]
            162 => {
                self.hillskew = 3;
                let param = buf.g4s();
                self.hillskew_param = (param != -1).then_some(param);
            }
            #[cfg(since_647)]
            163 => self.tint = [buf.g1s(), buf.g1s(), buf.g1s(), buf.g1s()],
            #[cfg(since_647)]
            164 => self.postoffsetx = buf.g2s(),
            #[cfg(since_647)]
            165 => self.postoffsety = buf.g2s(),
            #[cfg(since_647)]
            166 => self.postoffsetz = buf.g2s(),
            #[cfg(since_647)]
            167 => self.raise_object_ground_decor = buf.g2() as i32,
            #[cfg(since_647)]
            168 => self.bgsound_vorbis = true,
            #[cfg(since_647)]
            169 => self.randomsound_vorbis = true,
            #[cfg(since_647)]
            170 => self.op170 = buf.gsmart1or2(),
            #[cfg(since_647)]
            171 => self.op171 = buf.gsmart1or2(),
            #[cfg(since_647)]
            173 => self.background_sound_rate = (buf.g2(), buf.g2()),
            #[cfg(since_647)]
            177 => self.unknown_177 = true,
            #[cfg(since_647)]
            178 => self.background_sound_dropoff_range = buf.g1(),
            #[cfg(since_742)]
            189 => self.antimacro = true,
            #[cfg(since_742)]
            190..=195 => self.cursor[code as usize - 190] = Some(read_id(buf)),
            249 => {
                ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                );
            }
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("loc", code),
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
            #[cfg(since_530)]
            if loc.raiseobject.is_none() {
                loc.raiseobject = Some((loc.interact_type != 0) as u8);
            }
        }
    }

    #[cfg(all(since_530, before_647))]
    fn rewind(&mut self) {
        self.models = None;
        self.hd_models = None;
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

impl LocTypeRaw {
    fn set_models(&mut self, models: Box<[LocModelShape]>) {
        // A repeated model opcode is the low detail set.
        #[cfg(all(since_530, before_647))]
        if self.models.is_some() {
            self.hd_models = Some(models);
            return;
        }
        self.models = Some(models);
    }

    #[cfg(since_289)]
    fn decode_multiloc(&mut self, buf: &mut Packet, has_default: bool) {
        self.transform_varpbit = read_id(buf);
        if cfg!(since_530) {
            self.transform_varp = read_id(buf);
        }
        let default = if has_default {
            read_big_id(buf)
        } else {
            TypeId::NULL
        };
        self.multiloc = Some(read_transforms(buf, default, read_big_id));
    }
}

#[cfg(all(test, since_647))]
mod multiloc_tests {
    use super::*;

    /// 2011's Mill Lane flour bin (36880): on varp 695, 0 the empty bin
    /// (36879), 1-30 the full one (36878); its default is none.
    fn flour_bin() -> LocType {
        let multi = [36879, 36878, 36878, 36878]
            .map(TypeId::new)
            .into_iter()
            .chain([TypeId::NULL])
            .collect();
        LocType::from(LocTypeRaw::new(TypeId::new(36880)))
            .with_multi(MultiVar::Varp(TypeId::new(695)), multi)
    }

    impl LocType {
        fn with_multi(mut self, var: MultiVar, multi: Vec<TypeId>) -> Self {
            self.multivar = Some(var);
            self.multiloc = Some(multi.into_boxed_slice());
            self
        }
    }

    #[test]
    fn a_multiloc_is_the_loc_its_var_picks() {
        let bin = flour_bin();
        assert_eq!(bin.multi_loc(0), Some(TypeId::new(36879)), "empty");
        assert_eq!(bin.multi_loc(1), Some(TypeId::new(36878)), "full");
        assert_eq!(bin.multi_loc(3), Some(TypeId::new(36878)), "full");
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
        let loc = LocType::from(LocTypeRaw::new(TypeId::new(2718)));
        assert_eq!(loc.multi_loc(7), Some(TypeId::new(2718)));
    }
}

fn read_models(buf: &mut Packet, _code: u8) -> Box<[LocModelShape]> {
    read_array(buf, |buf| {
        #[cfg(before_647)]
        let model = read_id(buf);
        #[cfg(before_647)]
        let shape = if _code == 1 {
            buf.g1()
        } else {
            LocShape::CentrepieceStraight as u8
        };
        #[cfg(since_647)]
        let shape = buf.g1();
        #[cfg(since_647)]
        let models = read_array(buf, read_big_id);
        LocModelShape {
            #[cfg(before_647)]
            model,
            #[cfg(since_647)]
            models,
            shape,
        }
    })
}
