use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use crate::types::{BlockWalk, MoveRestrict, NpcMode};
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub type NpcTypeProvider = TypeProvider<NpcType>;

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
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub attack: u16,
    pub defence: u16,
    pub strength: u16,
    pub hitpoints: u16,
    pub ranged: u16,
    pub magic: u16,
    pub vislevel: Option<u16>,
    pub wanderrange: u16,
    pub maxrange: u16,
    pub huntrange: u8,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub moverestrict: MoveRestrict,
    pub attackrange: u16,
    pub blockwalk: BlockWalk,
    pub huntmode: Option<u16>,
    pub defaultmode: NpcMode,
    pub members: bool,
    pub patrol: Option<Box<[NpcPatrol]>>,
    pub givechase: bool,
    pub regenrate: u16,
    #[cfg(since_289)]
    pub turnspeed: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

impl NpcType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
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
            op: raw.op,
            attack: raw.attack,
            defence: raw.defence,
            strength: raw.strength,
            hitpoints: raw.hitpoints,
            ranged: raw.ranged,
            magic: raw.magic,
            vislevel: raw.vislevel,
            wanderrange: raw.wanderrange,
            maxrange: raw.maxrange,
            huntrange: raw.huntrange,
            timer: raw.timer,
            respawnrate: raw.respawnrate,
            moverestrict: raw.moverestrict,
            attackrange: raw.attackrange,
            blockwalk: raw.blockwalk,
            huntmode: raw.huntmode,
            defaultmode: raw.defaultmode,
            members: raw.members,
            patrol: raw.patrol,
            givechase: raw.givechase,
            regenrate: raw.regenrate,
            #[cfg(since_289)]
            turnspeed: raw.turnspeed,
            params: raw.params,
            debugname: raw.debugname,
        }
    }
}

pub struct NpcTypeRaw {
    pub id: u16,
    pub models: Option<Box<[u16]>>,
    pub name: Option<Box<str>>,
    pub desc: Option<Box<str>>,
    pub size: u8,
    pub readyanim: Option<u16>,
    pub walkanim: Option<u16>,
    pub hasalpha: bool,
    pub walkanim_b: Option<u16>,
    pub walkanim_l: Option<u16>,
    pub walkanim_r: Option<u16>,
    pub category: Option<u16>,
    pub op: Option<Box<[Option<Box<str>>]>>,
    pub recol_s: Option<Box<[u16]>>,
    pub recol_d: Option<Box<[u16]>>,
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
    pub wanderrange: u16,
    pub maxrange: u16,
    pub huntrange: u8,
    pub timer: Option<u16>,
    pub respawnrate: u16,
    pub moverestrict: MoveRestrict,
    pub attackrange: u16,
    pub blockwalk: BlockWalk,
    pub huntmode: Option<u16>,
    pub defaultmode: NpcMode,
    pub members: bool,
    pub patrol: Option<Box<[NpcPatrol]>>,
    pub givechase: bool,
    pub regenrate: u16,
    #[cfg(since_289)]
    pub turnspeed: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
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
            hasalpha: false,
            walkanim_b: None,
            walkanim_l: None,
            walkanim_r: None,
            category: None,
            op: None,
            recol_s: None,
            recol_d: None,
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
            wanderrange: 5,
            maxrange: 7,
            huntrange: 0,
            timer: None,
            respawnrate: 100,
            moverestrict: MoveRestrict::Normal,
            attackrange: 0,
            blockwalk: BlockWalk::Npc,
            huntmode: None,
            defaultmode: NpcMode::Wander,
            members: false,
            patrol: None,
            givechase: true,
            regenrate: 100,
            #[cfg(since_289)]
            turnspeed: 0,
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
                    self.models = Some(
                        (0..count)
                            .map(|_| buf.g2())
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    );
                }
                2 => self.name = Some(buf.gjstr(10).into_boxed_str()),
                3 => self.desc = Some(buf.gjstr(10).into_boxed_str()),
                12 => self.size = buf.g1(),
                13 => self.readyanim = Some(buf.g2()),
                14 => self.walkanim = Some(buf.g2()),
                16 => self.hasalpha = true,
                17 => {
                    self.walkanim = Some(buf.g2());
                    self.walkanim_b = Some(buf.g2());
                    self.walkanim_r = Some(buf.g2());
                    self.walkanim_l = Some(buf.g2());
                }
                18 => self.category = Some(buf.g2()),
                30..=34 => {
                    self.op
                        .get_or_insert_with(|| vec![None; 5].into_boxed_slice())
                        [code as usize - 30] = Some(buf.gjstr(10).into_boxed_str())
                }
                40 => {
                    let count = buf.g1() as usize;
                    let mut recol_s = vec![0u16; count];
                    let mut recol_d = vec![0u16; count];
                    for i in 0..count {
                        recol_s[i] = buf.g2();
                        recol_d[i] = buf.g2();
                    }
                    self.recol_s = Some(recol_s.into_boxed_slice());
                    self.recol_d = Some(recol_d.into_boxed_slice());
                }
                60 => {
                    let count = buf.g1() as usize;
                    self.heads = Some(
                        (0..count)
                            .map(|_| buf.g2())
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    );
                }
                74 => self.attack = buf.g2(),
                75 => self.defence = buf.g2(),
                76 => self.strength = buf.g2(),
                77 => self.hitpoints = buf.g2(),
                78 => self.ranged = buf.g2(),
                79 => self.magic = buf.g2(),
                90 => self.resizex = Some(buf.g2()),
                91 => self.resizey = Some(buf.g2()),
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
                200 => self.wanderrange = buf.g2(),
                201 => self.maxrange = buf.g2(),
                202 => self.huntrange = buf.g1(),
                203 => self.timer = Some(buf.g2()),
                204 => self.respawnrate = buf.g2(),
                206 => self.moverestrict = MoveRestrict::try_from(buf.g1()).unwrap(),
                207 => self.attackrange = buf.g2(),
                208 => self.blockwalk = BlockWalk::try_from(buf.g1()).unwrap(),
                209 => self.huntmode = Some(buf.g1() as u16),
                210 => self.defaultmode = NpcMode::try_from(buf.g1()).unwrap(),
                211 => self.members = true,
                212 => {
                    let count = buf.g1() as usize;
                    let mut patrol = Vec::with_capacity(count);
                    for _ in 0..count {
                        let coord = buf.g4s();
                        let delay = buf.g1();
                        patrol.push(NpcPatrol { coord, delay });
                    }
                    self.patrol = Some(patrol.into_boxed_slice());
                }
                213 => self.givechase = false,
                214 => self.regenrate = buf.g2(),
                #[cfg(since_289)]
                103 => self.turnspeed = buf.g2(),
                249 => ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                ),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized npc config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
