use super::decode::read_id;
use super::provider::{CacheType, TypeProvider};
use crate::types::*;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub type HuntTypeProvider = TypeProvider<HuntType>;

pub struct HuntType {
    pub id: TypeId,
    pub hunt_type: HuntModeType,
    pub check_vis: HuntCheckVis,
    pub check_nottoostrong: HuntCheckNotTooStrong,
    pub check_notbusy: HuntCheckNotBusy,
    pub find_keephunting: HuntFindKeepHunting,
    pub find_newmode: NpcMode,
    pub nobodynear: HuntNobodyNear,
    pub check_notcombat: Option<TypeId>,
    pub check_notcombat_self: Option<TypeId>,
    pub check_afk: HuntCheckAfk,
    pub rate: u16,
    pub check_category: Option<TypeId>,
    pub check_npc: Option<TypeId>,
    pub check_obj: Option<TypeId>,
    pub check_loc: Option<TypeId>,
    pub check_inv: Option<HuntCheckInv>,
    pub check_invparam: Option<HuntCheckInvParam>,
    pub check_invcat: Option<HuntCheckInvCat>,
    pub extracheck_vars: Vec<HuntExtraCheckVar>,
    debugname: Box<str>,
}

pub struct HuntCheckInv {
    pub inv: TypeId,
    pub obj: TypeId,
    pub condition: String,
    pub value: i32,
}

pub struct HuntCheckInvParam {
    pub inv: TypeId,
    pub param: TypeId,
    pub condition: String,
    pub value: i32,
}

pub struct HuntCheckInvCat {
    pub inv: TypeId,
    pub category: TypeId,
    pub condition: String,
    pub value: i32,
}

pub struct HuntExtraCheckVar {
    pub varp: TypeId,
    pub condition: String,
    pub value: i32,
}

impl CacheType for HuntType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        HuntType {
            id,
            hunt_type: HuntModeType::Off,
            check_vis: HuntCheckVis::Off,
            check_nottoostrong: HuntCheckNotTooStrong::Off,
            check_notbusy: HuntCheckNotBusy::Off,
            find_keephunting: HuntFindKeepHunting::Off,
            find_newmode: NpcMode::None,
            nobodynear: HuntNobodyNear::PauseHunt,
            check_notcombat: None,
            check_notcombat_self: None,
            check_afk: HuntCheckAfk::On,
            rate: 1,
            check_category: None,
            check_npc: None,
            check_obj: None,
            check_loc: None,
            check_inv: None,
            check_invparam: None,
            check_invcat: None,
            extracheck_vars: Vec::new(),
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.hunt_type = HuntModeType::try_from(buf.g1()).unwrap(),
            2 => self.check_vis = HuntCheckVis::try_from(buf.g1()).unwrap(),
            3 => self.check_nottoostrong = HuntCheckNotTooStrong::try_from(buf.g1()).unwrap(),
            4 => self.check_notbusy = HuntCheckNotBusy::On,
            5 => self.find_keephunting = HuntFindKeepHunting::On,
            6 => self.find_newmode = NpcMode::try_from(buf.g1()).unwrap(),
            7 => self.nobodynear = HuntNobodyNear::try_from(buf.g1()).unwrap(),
            8 => self.check_notcombat = Some(read_id(buf)),
            9 => self.check_notcombat_self = Some(read_id(buf)),
            10 => self.check_afk = HuntCheckAfk::Off,
            11 => self.rate = buf.g2(),
            12 => self.check_category = Some(read_id(buf)),
            13 => self.check_npc = Some(read_id(buf)),
            14 => self.check_obj = Some(read_id(buf)),
            #[cfg(before_742)]
            15 => self.check_loc = Some(read_id(buf)),
            #[cfg(since_742)]
            15 => self.check_loc = Some(TypeId::from_int(buf.g4s())),
            16 => {
                let inv = read_id(buf);
                let obj = read_id(buf);
                let condition = buf.gjstr(STRING_TERMINATOR);
                let value = buf.g4s();
                self.check_inv = Some(HuntCheckInv {
                    inv,
                    obj,
                    condition,
                    value,
                });
            }
            17 => {
                let inv = read_id(buf);
                let param = read_id(buf);
                let condition = buf.gjstr(STRING_TERMINATOR);
                let value = buf.g4s();
                self.check_invparam = Some(HuntCheckInvParam {
                    inv,
                    param,
                    condition,
                    value,
                });
            }
            18 => {
                let inv = read_id(buf);
                let category = read_id(buf);
                let condition = buf.gjstr(STRING_TERMINATOR);
                let value = buf.g4s();
                self.check_invcat = Some(HuntCheckInvCat {
                    inv,
                    category,
                    condition,
                    value,
                });
            }
            19..=21 => self.decode_check_var(buf),
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("hunt", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

impl HuntType {
    fn decode_check_var(&mut self, buf: &mut Packet) {
        let varp = read_id(buf);
        let condition = buf.gjstr(STRING_TERMINATOR);
        let value = buf.g4s();
        self.extracheck_vars.push(HuntExtraCheckVar {
            varp,
            condition,
            value,
        });
    }
}

pub fn check_hunt_condition(value: i32, condition: &str, target: i32) -> bool {
    match condition {
        ">" => value > target,
        "<" => value < target,
        "=" => value == target,
        "!" => value != target,
        "&" => (value & target) == 0,
        "|" => (value | target) != 0,
        _ => false,
    }
}
