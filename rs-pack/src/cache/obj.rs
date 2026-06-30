use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{DummyItem, WearPos};
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub type ObjTypeProvider = TypeProvider<ObjType>;

pub struct ObjContext {
    pub members: bool,
    pub autodisable_params: Box<[bool]>,
}

pub struct ObjType {
    pub id: u16,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub stackable: bool,
    pub cost: i32,
    pub wearpos: Option<WearPos>,
    pub wearpos2: Option<WearPos>,
    pub members: bool,
    pub wearpos3: Option<WearPos>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub weight: i16,
    pub category: Option<u16>,
    pub dummyitem: DummyItem,
    pub certlink: Option<u16>,
    pub certtemplate: Option<u16>,
    pub tradeable: bool,
    pub respawnrate: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl ObjType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<ObjTypeRaw> for ObjType {
    fn from(raw: ObjTypeRaw) -> Self {
        ObjType {
            id: raw.id,
            name: raw.name,
            desc: raw.desc,
            stackable: raw.stackable,
            cost: raw.cost,
            wearpos: raw.wearpos,
            wearpos2: raw.wearpos2,
            members: raw.members,
            wearpos3: raw.wearpos3,
            op: raw.op,
            iop: raw.iop,
            weight: raw.weight,
            category: raw.category,
            dummyitem: raw.dummyitem,
            certlink: raw.certlink,
            certtemplate: raw.certtemplate,
            tradeable: raw.tradeable,
            respawnrate: raw.respawnrate,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

pub struct ObjTypeRaw {
    pub id: u16,
    pub model: u16,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub zoom2d: u16,
    pub xan2d: u16,
    pub yan2d: u16,
    pub xof2d: i16,
    pub yof2d: i16,
    pub code9: bool,
    pub code10: Option<u16>,
    pub stackable: bool,
    pub cost: i32,
    pub wearpos: Option<WearPos>,
    pub wearpos2: Option<WearPos>,
    pub members: bool,
    pub manwear: Option<u16>,
    pub manwear2: Option<u16>,
    pub manweary: i8,
    pub womanwear: Option<u16>,
    pub womanwear2: Option<u16>,
    pub womanweary: i8,
    pub wearpos3: Option<WearPos>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    pub weight: i16,
    pub manwear3: Option<u16>,
    pub womanwear3: Option<u16>,
    pub manhead: Option<u16>,
    pub manhead2: Option<u16>,
    pub womanhead: Option<u16>,
    pub womanhead2: Option<u16>,
    pub category: Option<u16>,
    pub zan2d: u16,
    pub dummyitem: DummyItem,
    pub certlink: Option<u16>,
    pub certtemplate: Option<u16>,
    pub countobj: Option<Box<[u16]>>,
    pub countco: Option<Box<[u16]>>,
    #[cfg(since_244)]
    pub resizex: Option<u16>,
    #[cfg(since_244)]
    pub resizey: Option<u16>,
    #[cfg(since_244)]
    pub resizez: Option<u16>,
    #[cfg(since_244)]
    pub ambient: i8,
    #[cfg(since_244)]
    pub contrast: i8,
    #[cfg(since_289)]
    pub team: u8,
    pub tradeable: bool,
    pub respawnrate: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl ObjTypeRaw {
    #[allow(clippy::too_many_arguments)]
    fn cert_template(
        &mut self,
        model: u16,
        zoom2d: u16,
        xan2d: u16,
        yan2d: u16,
        zan2d: u16,
        xof2d: i16,
        yof2d: i16,
        recol_s: Option<Box<[u16]>>,
        recol_d: Option<Box<[u16]>>,
    ) {
        self.model = model;
        self.zoom2d = zoom2d;
        self.xan2d = xan2d;
        self.yan2d = yan2d;
        self.zan2d = zan2d;
        self.xof2d = xof2d;
        self.yof2d = yof2d;
        self.recol_s = recol_s;
        self.recol_d = recol_d;
    }

    fn cert_link(&mut self, name: Option<Box<str>>, members: bool, cost: i32, tradeable: bool) {
        self.name = name;
        self.members = members;
        self.cost = cost;
        self.tradeable = tradeable;
        self.stackable = true;
        if let Some(name) = &self.name
            && let Some(char) = name.chars().next()
        {
            let article: &str = if "AEIOU".contains(char) { "an" } else { "a" };
            self.desc = Some(format!("Swap this note at any bank for {article} {name}.").into());
        }
    }

    fn disable(&mut self, ctx: &ObjContext) {
        if !ctx.members && self.members {
            self.tradeable = false;
            self.op = Some(Box::new([None, None, Some("Take".into()), None, None]));
            self.iop = Some(Box::new([None, None, None, None, Some("Drop".into())]));
            self.category = None;

            if let Some(params) = &mut self.params {
                params.retain(|key, _| {
                    !usize::try_from(*key)
                        .ok()
                        .and_then(|id| ctx.autodisable_params.get(id))
                        .copied()
                        .unwrap_or(false)
                });
                if params.is_empty() {
                    self.params = None;
                }
            }
        }
    }
}

impl CacheType for ObjTypeRaw {
    type Context = ObjContext;

    fn new(id: u16) -> Self {
        ObjTypeRaw {
            id,
            model: 0,
            name: None,
            desc: None,
            zoom2d: 2000,
            xan2d: 0,
            yan2d: 0,
            xof2d: 0,
            yof2d: 0,
            code9: false,
            code10: None,
            stackable: false,
            cost: 1,
            wearpos: None,
            wearpos2: None,
            members: false,
            manwear: None,
            manwear2: None,
            manweary: 0,
            womanwear: None,
            womanwear2: None,
            womanweary: 0,
            wearpos3: None,
            op: Some(Box::new([None, None, Some("Take".into()), None, None])),
            iop: Some(Box::new([None, None, None, None, Some("Drop".into())])),
            recol_s: None,
            recol_d: None,
            weight: 0,
            manwear3: None,
            womanwear3: None,
            manhead: None,
            manhead2: None,
            womanhead: None,
            womanhead2: None,
            category: None,
            zan2d: 0,
            dummyitem: DummyItem::None,
            certlink: None,
            certtemplate: None,
            countobj: None,
            countco: None,
            #[cfg(since_244)]
            resizex: None,
            #[cfg(since_244)]
            resizey: None,
            #[cfg(since_244)]
            resizez: None,
            #[cfg(since_244)]
            ambient: 0,
            #[cfg(since_244)]
            contrast: 0,
            #[cfg(since_289)]
            team: 0,
            tradeable: true,
            respawnrate: 100,
            params: None,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.model = buf.g2(),
                2 => self.name = Some(buf.gjstr(10).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(10).into_boxed_str()),
                4 => self.zoom2d = buf.g2(),
                5 => self.xan2d = buf.g2(),
                6 => self.yan2d = buf.g2(),
                7 => {
                    let mut xof2d: i32 = buf.g2() as i32;
                    if xof2d > 32767 {
                        xof2d -= 65536;
                    }
                    self.xof2d = xof2d as i16;
                }
                8 => {
                    let mut yof2d: i32 = buf.g2() as i32;
                    if yof2d > 32767 {
                        yof2d -= 65536;
                    }
                    self.yof2d = yof2d as i16;
                }
                9 => self.code9 = true, // animHasAlpha from code10?
                10 => self.code10 = Some(buf.g2()), // seq?
                11 => self.stackable = true,
                12 => self.cost = buf.g4s(),
                13 => self.wearpos = Some(WearPos::try_from(buf.g1()).unwrap()),
                14 => self.wearpos2 = Some(WearPos::try_from(buf.g1()).unwrap()),
                15 => self.tradeable = false,
                16 => self.members = true,
                23 => {
                    self.manwear = Some(buf.g2());
                    self.manweary = buf.g1s();
                }
                24 => self.manwear2 = Some(buf.g2()),
                25 => {
                    self.womanwear = Some(buf.g2());
                    self.womanweary = buf.g1s();
                }
                26 => self.womanwear2 = Some(buf.g2()),
                27 => self.wearpos3 = Some(WearPos::try_from(buf.g1()).unwrap()),
                30..=34 => {
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(buf.gjstr(10).into_boxed_str())
                }
                35..=39 => {
                    self.iop
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 35] = Some(buf.gjstr(10).into_boxed_str())
                }
                40 => {
                    let count: usize = buf.g1() as usize;
                    let mut recol_s: Vec<u16> = vec![0; count];
                    let mut recol_d: Vec<u16> = vec![0; count];
                    for index in 0..count {
                        recol_s[index] = buf.g2();
                        recol_d[index] = buf.g2();
                    }
                    self.recol_s = Some(recol_s.into_boxed_slice());
                    self.recol_d = Some(recol_d.into_boxed_slice());
                }
                75 => self.weight = buf.g2s(),
                78 => self.manwear3 = Some(buf.g2()),
                79 => self.womanwear3 = Some(buf.g2()),
                90 => self.manhead = Some(buf.g2()),
                91 => self.womanhead = Some(buf.g2()),
                92 => self.manhead2 = Some(buf.g2()),
                93 => self.womanhead2 = Some(buf.g2()),
                94 => self.category = Some(buf.g2()),
                95 => self.zan2d = buf.g2(),
                96 => self.dummyitem = DummyItem::try_from(buf.g1()).unwrap(),
                97 => self.certlink = Some(buf.g2()),
                98 => self.certtemplate = Some(buf.g2()),
                100..=109 => {
                    self.countobj
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 100] = buf.g2();
                    self.countco
                        .get_or_insert_with(|| vec![0; 10].into_boxed_slice())
                        [code as usize - 100] = buf.g2();
                }
                #[cfg(since_244)]
                110 => self.resizex = Some(buf.g2()),
                #[cfg(since_244)]
                111 => self.resizey = Some(buf.g2()),
                #[cfg(since_244)]
                112 => self.resizez = Some(buf.g2()),
                #[cfg(since_244)]
                113 => self.ambient = buf.g1s(),
                #[cfg(since_244)]
                114 => self.contrast = buf.g1s(),
                #[cfg(since_289)]
                115 => self.team = buf.g1(),
                201 => self.respawnrate = buf.g2(),
                249 => ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                ),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognised obj config code: {code}"),
            }
        }
    }

    fn post_decode(objs: &mut Vec<Self>, ctx: &ObjContext) {
        let count = objs.len();
        for id in 0..count {
            if let Some(certtemplate) = objs.get(id).and_then(|obj| obj.certtemplate) {
                let template = objs
                    .get(certtemplate as usize)
                    .expect("Obj not found for a certtemplate!");

                let model: u16 = template.model;
                let zoom2d: u16 = template.zoom2d;
                let xan2d: u16 = template.xan2d;
                let yan2d: u16 = template.yan2d;
                let zan2d: u16 = template.zan2d;
                let xof2d: i16 = template.xof2d;
                let yof2d: i16 = template.yof2d;
                let recol_s: Option<Box<[u16]>> = template.recol_s.clone();
                let recol_d: Option<Box<[u16]>> = template.recol_d.clone();

                if let Some(obj) = objs.get_mut(id) {
                    obj.cert_template(
                        model, zoom2d, xan2d, yan2d, zan2d, xof2d, yof2d, recol_s, recol_d,
                    );
                }

                if let Some(certlink) = objs.get(id).and_then(|obj| obj.certlink) {
                    let link = objs
                        .get(certlink as usize)
                        .expect("Obj not found for a certlink!");

                    let name = link.name.clone();
                    let members: bool = link.members;
                    let cost: i32 = link.cost;
                    let tradeable: bool = link.tradeable;

                    if let Some(obj) = objs.get_mut(id) {
                        obj.cert_link(name, members, cost, tradeable);
                    }
                }
            }

            if let Some(obj) = objs.get_mut(id) {
                obj.disable(ctx);
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
