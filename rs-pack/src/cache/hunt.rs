use super::provider::{CacheType, TypeProvider};
use crate::types::*;
use rs_io::Packet;

pub type HuntTypeProvider = TypeProvider<HuntType>;

pub struct HuntType {
    pub id: u16,
    pub hunt_type: HuntModeType,
    pub check_vis: HuntCheckVis,
    pub check_nottoostrong: HuntCheckNotTooStrong,
    pub check_notbusy: HuntCheckNotBusy,
    pub find_keephunting: HuntFindKeepHunting,
    pub find_newmode: NpcMode,
    pub nobodynear: HuntNobodyNear,
    pub check_notcombat: Option<u16>,
    pub check_notcombat_self: Option<u16>,
    pub check_afk: HuntCheckAfk,
    pub rate: u16,
    pub check_category: Option<u16>,
    pub check_npc: Option<u16>,
    pub check_obj: Option<u16>,
    pub check_loc: Option<u16>,
    pub check_inv: Option<HuntCheckInv>,
    pub check_invparam: Option<HuntCheckInvParam>,
    pub extracheck_vars: Vec<HuntExtraCheckVar>,
    debugname: Option<Box<str>>,
}

pub struct HuntCheckInv {
    pub inv: u16,
    pub obj: u16,
    pub condition: String,
    pub value: i32,
}

pub struct HuntCheckInvParam {
    pub inv: u16,
    pub param: u16,
    pub condition: String,
    pub value: i32,
}

pub struct HuntExtraCheckVar {
    pub varp: u16,
    pub condition: String,
    pub value: i32,
}

impl CacheType for HuntType {
    type Context = ();

    fn new(id: u16) -> Self {
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
            extracheck_vars: Vec::new(),
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.hunt_type = HuntModeType::try_from(buf.g1()).unwrap(),
                2 => self.check_vis = HuntCheckVis::try_from(buf.g1()).unwrap(),
                3 => self.check_nottoostrong = HuntCheckNotTooStrong::try_from(buf.g1()).unwrap(),
                4 => self.check_notbusy = HuntCheckNotBusy::On,
                5 => self.find_keephunting = HuntFindKeepHunting::On,
                6 => self.find_newmode = NpcMode::try_from(buf.g1()).unwrap(),
                7 => self.nobodynear = HuntNobodyNear::try_from(buf.g1()).unwrap(),
                8 => self.check_notcombat = Some(buf.g2()),
                9 => self.check_notcombat_self = Some(buf.g2()),
                10 => self.check_afk = HuntCheckAfk::Off,
                11 => self.rate = buf.g2(),
                12 => self.check_category = Some(buf.g2()),
                13 => self.check_npc = Some(buf.g2()),
                14 => self.check_obj = Some(buf.g2()),
                15 => self.check_loc = Some(buf.g2()),
                16 => {
                    let inv = buf.g2();
                    let obj = buf.g2();
                    let condition = buf.gjstr(10);
                    let value = buf.g4s();
                    self.check_inv = Some(HuntCheckInv {
                        inv,
                        obj,
                        condition,
                        value,
                    });
                }
                17 => {
                    let inv = buf.g2();
                    let param = buf.g2();
                    let condition = buf.gjstr(10);
                    let value = buf.g4s();
                    self.check_invparam = Some(HuntCheckInvParam {
                        inv,
                        param,
                        condition,
                        value,
                    });
                }
                18..=20 => {
                    let varp = buf.g2();
                    let condition = buf.gjstr(10);
                    let value = buf.g4s();
                    self.extracheck_vars.push(HuntExtraCheckVar {
                        varp,
                        condition,
                        value,
                    });
                }
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized hunt config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
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
