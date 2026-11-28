use super::decode::{read_big_id, read_id, read_versioned_str};
use super::param::ParamType;
use super::provider::CacheType;
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub struct PrerequisiteVar {
    pub var: i32,
    pub min: i32,
    pub max: i32,
    pub description: String,
}

pub struct QuestType {
    pub id: TypeId,
    pub name: String,
    pub sortname: String,
    pub progressvarp: Vec<(TypeId, i32, i32)>,
    pub progressvarpbit: Vec<(TypeId, i32, i32)>,
    pub parent: TypeId,
    pub quest_type: u8,
    pub difficulty: u8,
    pub members: bool,
    pub quest_points: u8,
    pub mainstartcoord: Vec<i32>,
    pub viacoord: i32,
    pub questrequirement: Vec<TypeId>,
    pub prerequisite_stats: Vec<(u8, u8)>,
    pub pointsrequirement: u16,
    pub graphic: Option<TypeId>,
    pub varprequirement: Vec<PrerequisiteVar>,
    pub varpbitrequirement: Vec<PrerequisiteVar>,
    pub params: Vec<(TypeId, ParamValue)>,
}

fn read_var_requirements(buf: &mut Packet) -> Vec<PrerequisiteVar> {
    (0..buf.g1())
        .map(|_| PrerequisiteVar {
            var: buf.g4s(),
            min: buf.g4s(),
            max: buf.g4s(),
            description: buf.gjstr(STRING_TERMINATOR),
        })
        .collect()
}

fn read_progress(buf: &mut Packet) -> Vec<(TypeId, i32, i32)> {
    (0..buf.g1())
        .map(|_| (read_id(buf), buf.g4s(), buf.g4s()))
        .collect()
}

impl CacheType for QuestType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        Self {
            id,
            name: String::new(),
            sortname: String::new(),
            progressvarp: Vec::new(),
            progressvarpbit: Vec::new(),
            parent: TypeId::default(),
            quest_type: 0,
            difficulty: 0,
            members: false,
            quest_points: 0,
            mainstartcoord: Vec::new(),
            viacoord: 0,
            questrequirement: Vec::new(),
            prerequisite_stats: Vec::new(),
            pointsrequirement: 0,
            graphic: None,
            varprequirement: Vec::new(),
            varpbitrequirement: Vec::new(),
            params: Vec::new(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.name = read_versioned_str(buf),
            2 => self.sortname = read_versioned_str(buf),
            3 => self.progressvarp = read_progress(buf),
            4 => self.progressvarpbit = read_progress(buf),
            5 => self.parent = read_id(buf),
            6 => self.quest_type = buf.g1(),
            7 => self.difficulty = buf.g1(),
            8 => self.members = true,
            9 => self.quest_points = buf.g1(),
            10 => self.mainstartcoord = (0..buf.g1()).map(|_| buf.g4s()).collect(),
            12 => self.viacoord = buf.g4s(),
            13 => self.questrequirement = (0..buf.g1()).map(|_| read_id(buf)).collect(),
            14 => self.prerequisite_stats = (0..buf.g1()).map(|_| (buf.g1(), buf.g1())).collect(),
            15 => self.pointsrequirement = buf.g2(),
            17 => self.graphic = Some(read_big_id(buf)),
            18 => self.varprequirement = read_var_requirements(buf),
            19 => self.varpbitrequirement = read_var_requirements(buf),
            249 => {
                for _ in 0..buf.g1() {
                    self.params.push(ParamType::decode_param(buf));
                }
            }
            _ => crate::unknown_opcode!("quest", code),
        }
    }
}
