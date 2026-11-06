#[cfg(since_647)]
use super::decode::read_array;
use super::decode::read_pairs;
use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{DummyItem, WearPos};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type ObjTypeProvider = TypeProvider<ObjType>;

pub struct ObjContext {
    pub members: bool,
    pub autodisable_params: Box<[bool]>,
}

pub struct ObjType {
    pub id: u16,
    #[cfg(since_647)]
    pub wearid: Option<u16>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub stackable: bool,
    pub cost: i32,
    pub wearpos: Option<WearPos>,
    pub wearpos2: Option<WearPos>,
    pub tradeable: bool,
    pub members: bool,
    pub wearpos3: Option<WearPos>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub weight: i16,
    pub category: Option<u16>,
    pub dummyitem: DummyItem,
    pub certlink: Option<u16>,
    pub certtemplate: Option<u16>,
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
            tradeable: raw.tradeable,
            members: raw.members,
            wearpos3: raw.wearpos3,
            #[cfg(since_647)]
            wearid: raw.wearid,
            op: raw.op,
            iop: raw.iop,
            weight: raw.weight,
            category: raw.category,
            dummyitem: raw.dummyitem,
            certlink: raw.certlink,
            certtemplate: raw.certtemplate,
            respawnrate: raw.respawnrate,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

#[cfg_attr(since_647, derive(Debug, Clone))]
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
    pub tradeable: bool,
    pub members: bool,
    #[cfg(since_647)]
    pub multistack: u16,
    pub manwear: Option<u16>,
    pub manweary: i8,
    pub manwear2: Option<u16>,
    pub womanwear: Option<u16>,
    pub womanweary: i8,
    pub womanwear2: Option<u16>,
    pub wearpos3: Option<WearPos>,
    #[cfg(since_647)]
    pub wearid: Option<u16>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_d: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub retex_s: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub recol_palette: Option<Box<[i8]>>,
    #[cfg(since_647)]
    pub stockmarket: bool,
    pub weight: i16,
    pub manwear3: Option<u16>,
    pub womanwear3: Option<u16>,
    pub manhead: Option<u16>,
    pub womanhead: Option<u16>,
    pub manhead2: Option<u16>,
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
    #[cfg(since_647)]
    pub lentlink: u16,
    #[cfg(since_647)]
    pub lenttemplate: u16,
    #[cfg(since_647)]
    pub manwear_xoff: i8,
    #[cfg(since_647)]
    pub manwear_zoff: i8,
    #[cfg(since_647)]
    pub womanwear_xoff: i8,
    #[cfg(since_647)]
    pub womanwear_zoff: i8,
    #[cfg(since_647)]
    pub cursor1: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub cursor2: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub icursor1: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub icursor2: Option<(u8, u16)>,
    #[cfg(since_647)]
    pub quests: Option<Box<[u16]>>,
    #[cfg(since_647)]
    pub picksizeshift: u8,
    #[cfg(since_647)]
    pub boughtlink: u16,
    #[cfg(since_647)]
    pub boughttemplate: u16,
    pub respawnrate: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
    pub raw_opcodes: Vec<(u8, Vec<u8>)>,
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
            #[cfg(since_647)]
            {
                self.name = Some("Members object".into());
                self.stockmarket = false;
                self.team = 0;
                self.quests = None;
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
            tradeable: true,
            members: false,
            #[cfg(since_647)]
            multistack: 0,
            manwear: None,
            manweary: 0,
            manwear2: None,
            womanwear: None,
            womanweary: 0,
            womanwear2: None,
            wearpos3: None,
            #[cfg(since_647)]
            wearid: None,
            op: Some(Box::new([None, None, Some("Take".into()), None, None])),
            iop: Some(Box::new([None, None, None, None, Some("Drop".into())])),
            recol_s: None,
            recol_d: None,
            #[cfg(since_647)]
            retex_d: None,
            #[cfg(since_647)]
            retex_s: None,
            #[cfg(since_647)]
            recol_palette: None,
            #[cfg(since_647)]
            stockmarket: false,
            weight: 0,
            manwear3: None,
            womanwear3: None,
            manhead: None,
            womanhead: None,
            manhead2: None,
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
            #[cfg(since_647)]
            lentlink: 0,
            #[cfg(since_647)]
            lenttemplate: 0,
            #[cfg(since_647)]
            manwear_xoff: 0,
            #[cfg(since_647)]
            manwear_zoff: 0,
            #[cfg(since_647)]
            womanwear_xoff: 0,
            #[cfg(since_647)]
            womanwear_zoff: 0,
            #[cfg(since_647)]
            cursor1: None,
            #[cfg(since_647)]
            cursor2: None,
            #[cfg(since_647)]
            icursor1: None,
            #[cfg(since_647)]
            icursor2: None,
            #[cfg(since_647)]
            quests: None,
            #[cfg(since_647)]
            picksizeshift: 0,
            #[cfg(since_647)]
            boughtlink: 0,
            #[cfg(since_647)]
            boughttemplate: 0,
            respawnrate: 100,
            params: None,
            debugname: None,
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
                1 => self.model = buf.g2(),
                2 => self.name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                4 => self.zoom2d = buf.g2(),
                5 => self.xan2d = buf.g2(),
                6 => self.yan2d = buf.g2(),
                7 => self.xof2d = buf.g2s(),
                8 => self.yof2d = buf.g2s(),
                #[cfg(before_647)]
                9 => self.code9 = true, // animHasAlpha from code10?
                #[cfg(before_647)]
                10 => self.code10 = Some(buf.g2()), // seq?
                11 => self.stackable = true,
                12 => self.cost = buf.g4s(),
                13 => self.wearpos = Some(WearPos::try_from(buf.g1()).unwrap()),
                14 => self.wearpos2 = Some(WearPos::try_from(buf.g1()).unwrap()),
                15 => self.tradeable = false,
                16 => self.members = true,
                #[cfg(since_647)]
                18 => self.multistack = buf.g2(),
                23 => {
                    self.manwear = Some(buf.g2());
                    #[cfg(before_647)]
                    {
                        self.manweary = buf.g1s();
                    }
                }
                24 => self.manwear2 = Some(buf.g2()),
                25 => {
                    self.womanwear = Some(buf.g2());
                    #[cfg(before_647)]
                    {
                        self.womanweary = buf.g1s();
                    }
                }
                26 => self.womanwear2 = Some(buf.g2()),
                27 => self.wearpos3 = Some(WearPos::try_from(buf.g1()).unwrap()),
                30..=34 => {
                    let op = buf.gjstr(STRING_TERMINATOR);
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(op.into_boxed_str());
                }
                35..=39 => {
                    let op = buf.gjstr(STRING_TERMINATOR);
                    self.iop
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 35] = Some(op.into_boxed_str());
                }
                40 => read_pairs(buf, &mut self.recol_s, &mut self.recol_d),
                #[cfg(since_647)]
                41 => read_pairs(buf, &mut self.retex_d, &mut self.retex_s),
                #[cfg(since_647)]
                42 => self.recol_palette = Some(read_array(buf, Packet::g1s)),
                #[cfg(since_647)]
                65 => self.stockmarket = true,
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
                #[cfg(since_647)]
                121 => self.lentlink = buf.g2(),
                #[cfg(since_647)]
                122 => self.lenttemplate = buf.g2(),
                #[cfg(since_647)]
                125 => {
                    self.manwear_xoff = buf.g1() as i8;
                    self.manweary = buf.g1() as i8;
                    self.manwear_zoff = buf.g1() as i8;
                }
                #[cfg(since_647)]
                126 => {
                    self.womanwear_xoff = buf.g1() as i8;
                    self.womanweary = buf.g1() as i8;
                    self.womanwear_zoff = buf.g1() as i8;
                }
                #[cfg(since_647)]
                127 => self.cursor1 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                128 => self.cursor2 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                129 => self.icursor1 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                130 => self.icursor2 = Some((buf.g1(), buf.g2())),
                #[cfg(since_647)]
                132 => self.quests = Some(read_array(buf, Packet::g2)),
                #[cfg(since_647)]
                134 => self.picksizeshift = buf.g1(),
                #[cfg(since_647)]
                139 => self.boughtlink = buf.g2(),
                #[cfg(since_647)]
                140 => self.boughttemplate = buf.g2(),
                201 => self.respawnrate = buf.g2(),
                249 => {
                    ParamType::decode_params(
                        buf,
                        self.params
                            .get_or_insert_with(|| Box::new(FxHashMap::default())),
                    );
                }
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("obj", code),
            }
            self.raw_opcodes
                .push((code, buf.data[start..buf.pos].to_vec()));
        }
    }

    fn post_decode(objs: &mut Vec<Self>, ctx: &Self::Context) {
        #[cfg(since_647)]
        let mut wearid = 0;
        for id in 0..objs.len() {
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
                #[cfg(since_647)]
                let retex_s = template.retex_s.clone();
                #[cfg(since_647)]
                let retex_d = template.retex_d.clone();
                #[cfg(since_647)]
                let recol_palette = template.recol_palette.clone();

                if let Some(obj) = objs.get_mut(id) {
                    obj.cert_template(
                        model, zoom2d, xan2d, yan2d, zan2d, xof2d, yof2d, recol_s, recol_d,
                    );
                    #[cfg(since_647)]
                    {
                        obj.retex_s = retex_s;
                        obj.retex_d = retex_d;
                        obj.recol_palette = recol_palette;
                    }
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
            #[cfg(since_647)]
            {
                if objs[id].lenttemplate != 0 {
                    let template_id = objs[id].lenttemplate as usize;
                    let link_id = objs[id].lentlink as usize;
                    if template_id < objs.len() && link_id < objs.len() {
                        let template = objs[template_id].clone();
                        let link = objs[link_id].clone();
                        gen_lent(&mut objs[id], &template, &link);
                    }
                }
                if objs[id].boughttemplate != 0 {
                    let template_id = objs[id].boughttemplate as usize;
                    let link_id = objs[id].boughtlink as usize;
                    if template_id < objs.len() && link_id < objs.len() {
                        let template = objs[template_id].clone();
                        let link = objs[link_id].clone();
                        gen_bought(&mut objs[id], &template, &link);
                    }
                }
                if objs[id].manwear.is_some() || objs[id].womanwear.is_some() {
                    objs[id].wearid = Some(wearid);
                    wearid += 1;
                }
            }
            objs[id].disable(ctx);
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

#[cfg(since_647)]
fn gen_lent(obj: &mut ObjTypeRaw, template: &ObjTypeRaw, link: &ObjTypeRaw) {
    obj.params = link.params.clone();
    obj.womanweary = link.womanweary;
    obj.zan2d = template.zan2d;
    obj.retex_s = link.retex_s.clone();
    obj.womanwear3 = link.womanwear3;
    obj.name = link.name.clone();
    obj.model = template.model;
    obj.recol_s = link.recol_s.clone();
    obj.womanwear_xoff = link.womanwear_xoff;
    obj.yan2d = template.yan2d;
    obj.xof2d = template.xof2d;
    obj.womanhead2 = link.womanhead2;
    obj.recol_d = link.recol_d.clone();
    obj.xan2d = template.xan2d;
    obj.manhead2 = link.manhead2;
    obj.womanwear = link.womanwear;
    obj.womanwear_zoff = link.womanwear_zoff;
    obj.cost = 0;
    obj.yof2d = template.yof2d;
    obj.op = link.op.clone();
    obj.manweary = link.manweary;
    obj.members = link.members;
    obj.team = link.team;
    obj.recol_palette = link.recol_palette.clone();
    obj.manwear2 = link.manwear2;
    obj.manwear_xoff = link.manwear_xoff;
    obj.iop = Some(Box::new([const { None }; 5]));
    obj.womanhead = link.womanhead;
    obj.manwear_zoff = link.manwear_zoff;
    obj.zoom2d = template.zoom2d;
    obj.manwear3 = link.manwear3;
    obj.retex_d = link.retex_d.clone();
    obj.manwear = link.manwear;
    obj.womanwear2 = link.womanwear2;
    obj.manhead = link.manhead;
    obj.resizex = link.resizex;
    obj.resizey = link.resizey;
    obj.resizez = link.resizez;
    obj.ambient = link.ambient;
    obj.contrast = link.contrast;
}
#[cfg(since_647)]
fn gen_bought(obj: &mut ObjTypeRaw, template: &ObjTypeRaw, link: &ObjTypeRaw) {
    obj.xof2d = template.xof2d;
    obj.womanwear3 = link.womanwear3;
    obj.model = template.model;
    obj.womanhead2 = link.womanhead2;
    obj.womanwear_zoff = link.womanwear_zoff;
    obj.stackable = link.stackable;
    obj.manweary = link.manweary;
    obj.yan2d = template.yan2d;
    obj.womanwear = link.womanwear;
    obj.retex_d = link.retex_d.clone();
    obj.manwear_zoff = link.manwear_zoff;
    obj.manwear3 = link.manwear3;
    obj.recol_d = link.recol_d.clone();
    obj.manwear = link.manwear;
    obj.params = link.params.clone();
    obj.manwear2 = link.manwear2;
    obj.yof2d = template.yof2d;
    obj.womanwear2 = link.womanwear2;
    obj.members = link.members;
    obj.name = link.name.clone();
    obj.cost = 0;
    obj.womanhead = link.womanhead;
    obj.womanweary = link.womanweary;
    obj.zan2d = template.zan2d;
    obj.recol_s = link.recol_s.clone();
    obj.manhead = link.manhead;
    obj.manwear_xoff = link.manwear_xoff;
    obj.recol_palette = link.recol_palette.clone();
    obj.womanwear_xoff = link.womanwear_xoff;
    obj.zoom2d = template.zoom2d;
    obj.xan2d = template.xan2d;
    obj.retex_s = link.retex_s.clone();
    obj.manhead2 = link.manhead2;
    obj.resizex = link.resizex;
    obj.resizey = link.resizey;
    obj.resizez = link.resizez;
    obj.ambient = link.ambient;
    obj.contrast = link.contrast;
    obj.op = link.op.clone();
    obj.iop = link.iop.clone();
    obj.team = link.team;
}
