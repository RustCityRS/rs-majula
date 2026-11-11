use super::decode::read_versioned_str;
use super::param::ParamType;
use super::provider::CacheType;
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub struct QuestVarRequirement {
    pub var: i32,
    pub min: i32,
    pub max: i32,
    pub description: String,
}

pub struct QuestType {
    pub id: u16,
    pub name: String,
    pub sortname: String,
    pub progressvarp: Vec<(u16, i32, i32)>,
    pub progressvarbit: Vec<(u16, i32, i32)>,
    pub op5: u16,
    pub quest_type: u8,
    pub difficulty: u8,
    pub members: bool,
    pub points: u8,
    pub op10: Vec<i32>,
    pub op12: i32,
    pub questrequirement: Vec<u16>,
    pub statrequirement: Vec<(u8, u8)>,
    pub pointsrequirement: u16,
    pub graphic: Option<u16>,
    pub varprequirement: Vec<QuestVarRequirement>,
    pub varbitrequirement: Vec<QuestVarRequirement>,
    pub params: Vec<(i32, ParamValue)>,
}

fn read_var_requirements(buf: &mut Packet) -> Vec<QuestVarRequirement> {
    (0..buf.g1())
        .map(|_| QuestVarRequirement {
            var: buf.g4s(),
            min: buf.g4s(),
            max: buf.g4s(),
            description: buf.gjstr(STRING_TERMINATOR),
        })
        .collect()
}

fn read_progress(buf: &mut Packet) -> Vec<(u16, i32, i32)> {
    (0..buf.g1())
        .map(|_| (buf.g2(), buf.g4s(), buf.g4s()))
        .collect()
}

impl CacheType for QuestType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            name: String::new(),
            sortname: String::new(),
            progressvarp: Vec::new(),
            progressvarbit: Vec::new(),
            op5: 0,
            quest_type: 0,
            difficulty: 0,
            members: false,
            points: 0,
            op10: Vec::new(),
            op12: 0,
            questrequirement: Vec::new(),
            statrequirement: Vec::new(),
            pointsrequirement: 0,
            graphic: None,
            varprequirement: Vec::new(),
            varbitrequirement: Vec::new(),
            params: Vec::new(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.name = read_versioned_str(buf),
            2 => self.sortname = read_versioned_str(buf),
            3 => self.progressvarp = read_progress(buf),
            4 => self.progressvarbit = read_progress(buf),
            5 => self.op5 = buf.g2(),
            6 => self.quest_type = buf.g1(),
            7 => self.difficulty = buf.g1(),
            8 => self.members = true,
            9 => self.points = buf.g1(),
            10 => self.op10 = (0..buf.g1()).map(|_| buf.g4s()).collect(),
            12 => self.op12 = buf.g4s(),
            13 => self.questrequirement = (0..buf.g1()).map(|_| buf.g2()).collect(),
            14 => self.statrequirement = (0..buf.g1()).map(|_| (buf.g1(), buf.g1())).collect(),
            15 => self.pointsrequirement = buf.g2(),
            17 => self.graphic = Some(buf.g2()),
            18 => self.varprequirement = read_var_requirements(buf),
            19 => self.varbitrequirement = read_var_requirements(buf),
            249 => {
                for _ in 0..buf.g1() {
                    self.params.push(ParamType::decode_param(buf));
                }
            }
            _ => crate::unknown_opcode!("quest", code),
        }
    }
}
