use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{ForceApproach, LocShape};
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
    #[cfg(before_647)]
    pub forceapproach: ForceApproach,
    /// The sides the loc can't be reached from (bits north 1, east 2, south
    /// 4, west 8, rotated by the loc's angle), as the client's LocType holds
    /// them: 2011's cache has values 289's single-side `ForceApproach` can't
    /// (a bank booth's 10, open north and south).
    #[cfg(since_647)]
    pub forceapproach: u8,
    /// The varbit it turns into another loc by, and the loc for each of its
    /// values (-1: none, so the client shows nothing). The client shows that
    /// loc's name and options, but clicks on it carry this one's id.
    ///
    /// From rev 647 `multiloc` is laid out as the client's `LocType.multi`:
    /// the locs by var value, then the default (-1 for none) last; and a
    /// multiloc with no varbit turns by a varp (`multivarp`).
    pub multivar: Option<u16>,
    pub multiloc: Option<Box<[i32]>>,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    /// Rev-647: the varp a multiloc turns by when it has no varbit
    /// (`multivar`).
    #[cfg(since_647)]
    pub multivarp: Option<u16>,
    pub(crate) debugname: Option<Box<str>>,
}

impl LocType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }

    /// The sides the loc can't be reached from, as the pathfinder takes them.
    pub fn forceapproach_flags(&self) -> u8 {
        #[cfg(before_647)]
        return self.forceapproach as u8;
        #[cfg(since_647)]
        return self.forceapproach;
    }

    /// The loc a player sees for a rev-647 multiloc, by its var's value
    /// (`LocType.get_multi_loc`): out of range or -1, the default; `None` for
    /// no loc. A loc that is no multiloc is itself.
    #[cfg(since_647)]
    pub fn multi_loc(&self, var_value: i32) -> Option<u16> {
        let Some(multi) = self.multiloc.as_deref() else {
            return Some(self.id);
        };
        let n = multi.len();
        let pick = if var_value < 0 || n as i32 - 1 <= var_value || multi[var_value as usize] == -1
        {
            multi[n - 1]
        } else {
            multi[var_value as usize]
        };
        u16::try_from(pick).ok()
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
            #[cfg(before_647)]
            forceapproach: raw.forceapproach,
            #[cfg(since_647)]
            forceapproach: raw.forceapproach as u8,
            #[cfg(since_289)]
            multivar: raw.multivar,
            #[cfg(since_289)]
            multiloc: raw.multiloc,
            #[cfg(before_289)]
            multivar: None,
            #[cfg(before_289)]
            multiloc: None,
            params: raw.params,
            // At 647 locs come from the JS5 cache (`store647`), which carries them.
            #[cfg(since_647)]
            multivarp: None,
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
    #[cfg(since_245_2)]
    pub breakroutefinding: bool,
    #[cfg(since_254)]
    pub raiseobject: Option<u8>,
    #[cfg(since_289)]
    pub multivar: Option<u16>,
    #[cfg(since_289)]
    pub multiloc: Option<Box<[i32]>>,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    pub(crate) debugname: Option<Box<str>>,
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
            #[cfg(since_245_2)]
            breakroutefinding: false,
            #[cfg(since_254)]
            raiseobject: None,
            #[cfg(since_289)]
            multivar: None,
            #[cfg(since_289)]
            multiloc: None,
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
                #[cfg(since_254)]
                5 => {
                    let count = buf.g1() as usize;
                    let mut models = Vec::with_capacity(count);
                    for _ in 0..count {
                        models.push(LocModelShape {
                            model: buf.g2(),
                            shape: LocShape::CentrepieceStraight as u8,
                        });
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
                #[cfg(since_245_2)]
                74 => self.breakroutefinding = true,
                #[cfg(since_254)]
                75 => self.raiseobject = Some(buf.g1()),
                #[cfg(since_289)]
                77 => {
                    self.multivar = Some(buf.g2());
                    let count = buf.g1() as usize;
                    let mut multiloc = vec![0i32; count + 1];
                    for slot in multiloc.iter_mut() {
                        let id = buf.g2();
                        *slot = if id == u16::MAX { -1 } else { id as i32 };
                    }
                    self.multiloc = Some(multiloc.into_boxed_slice());
                }
                249 => ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                ),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("loc", code),
            }
        }
    }

    fn post_decode(locs: &mut Vec<Self>, _ctx: &()) {
        for loc in locs.iter_mut() {
            if loc.active.is_none() {
                let mut active = false;

                if let Some(models) = &loc.models
                    && models
                        .first()
                        .is_some_and(|m| m.shape == LocShape::CentrepieceStraight as u8)
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

#[cfg(all(test, since_647))]
mod multiloc_tests {
    use super::*;

    /// 2011's Mill Lane flour bin (36880): on varp 695, 0 the empty bin
    /// (36879), 1-30 the full one (36878); its default is none.
    fn flour_bin() -> LocType {
        let mut multi = vec![36879, 36878, 36878, 36878];
        multi.push(-1);
        LocType::from(LocTypeRaw::new(36880)).with_multi(None, Some(695), multi)
    }

    impl LocType {
        fn with_multi(mut self, varbit: Option<u16>, varp: Option<u16>, multi: Vec<i32>) -> Self {
            self.multivar = varbit;
            self.multivarp = varp;
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
