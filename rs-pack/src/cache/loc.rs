use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::ForceApproach;
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub type LocTypeProvider = TypeProvider<LocType>;

pub struct LocModelShape {
    pub model: u16,
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
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl LocType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<LocTypeRaw> for LocType {
    fn from(raw: LocTypeRaw) -> Self {
        LocType {
            id: raw.id,
            name: raw.name,
            desc: raw.desc,
            width: raw.width,
            length: raw.length,
            blockwalk: raw.blockwalk,
            blockrange: raw.blockrange,
            active: raw.active,
            op: raw.op,
            category: raw.category,
            forceapproach: raw.forceapproach,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

pub struct LocTypeRaw {
    pub id: u16,
    pub models: Option<Box<[LocModelShape]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub width: u8,
    pub length: u8,
    pub blockwalk: bool,
    pub blockrange: bool,
    pub active: Option<bool>,
    pub hillskew: bool,
    pub sharelight: bool,
    pub occlude: bool,
    pub anim: Option<u16>,
    pub hasalpha: bool,
    pub wallwidth: u8,
    pub ambient: i8,
    pub contrast: i8,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
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
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl CacheType for LocTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        LocTypeRaw {
            id,
            models: None,
            name: None,
            desc: None,
            width: 1,
            length: 1,
            blockwalk: true,
            blockrange: true,
            active: None,
            hillskew: false,
            sharelight: false,
            occlude: false,
            anim: None,
            hasalpha: false,
            wallwidth: 16,
            ambient: 0,
            contrast: 0,
            op: None,
            recol_s: None,
            recol_d: None,
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
            params: None,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    let count = buf.g1() as usize;
                    let mut models = Vec::with_capacity(count);
                    for _ in 0..count {
                        let model = buf.g2();
                        let shape = buf.g1();
                        models.push(LocModelShape { model, shape });
                    }
                    self.models = Some(models.into_boxed_slice());
                }
                2 => self.name = Some(buf.gjstr(10).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(10).into_boxed_str()),
                14 => self.width = buf.g1(),
                15 => self.length = buf.g1(),
                17 => self.blockwalk = false,
                18 => self.blockrange = false,
                19 => self.active = Some(buf.g1() == 1),
                21 => self.hillskew = true,
                22 => self.sharelight = true,
                23 => self.occlude = true,
                24 => self.anim = Some(buf.g2()),
                25 => self.hasalpha = true,
                28 => self.wallwidth = buf.g1(),
                29 => self.ambient = buf.g1s(),
                30..=34 => {
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(buf.gjstr(10).into_boxed_str())
                }
                39 => self.contrast = buf.g1s(),
                40 => {
                    let count = buf.g1() as usize;
                    let mut recol_s = vec![0; count];
                    let mut recol_d = vec![0; count];
                    for i in 0..count {
                        recol_s[i] = buf.g2();
                        recol_d[i] = buf.g2();
                    }
                    self.recol_s = Some(recol_s.into_boxed_slice());
                    self.recol_d = Some(recol_d.into_boxed_slice());
                }
                60 => self.mapfunction = Some(buf.g2()),
                61 => self.category = Some(buf.g2()),
                62 => self.mirror = true,
                64 => self.shadow = false,
                65 => self.resizex = buf.g2(),
                66 => self.resizey = buf.g2(),
                67 => self.resizez = buf.g2(),
                68 => self.mapscene = Some(buf.g2()),
                69 => self.forceapproach = ForceApproach::try_from(buf.g1()).unwrap(),
                70 => self.offsetx = buf.g2s(),
                71 => self.offsety = buf.g2s(),
                72 => self.offsetz = buf.g2s(),
                73 => self.forcedecor = true,
                249 => ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                ),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized loc config code: {code}"),
            }
        }
    }

    fn post_decode(locs: &mut Vec<Self>, _ctx: &()) {
        for loc in locs.iter_mut() {
            if loc.active.is_none() {
                let mut active = false;

                if let Some(models) = &loc.models
                    && models.len() == 1
                    && models[0].shape == 10
                {
                    active = true;
                }

                if loc.op.is_some() {
                    active = true;
                }

                loc.active = Some(active);
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
