//! HuntType — CacheType implementation for NPC hunt AI definitions.
//!
//! Server-side virtual type (archive 2, group 2). Not packed into the
//! rev-647 client cache by Jagex. Ground-truth decoder is 2004scape's
//! `engine/src/cache/config/HuntType.ts`.

use rs_io::Packet;

use super::CacheType;

/// NPC hunt behaviour definition.
///
/// | op  | shape                                                       |
/// |-----|-------------------------------------------------------------|
/// | 1   | g1 (hunt mode type)                                         |
/// | 2   | g1 (check vis)                                              |
/// | 3   | g1 (check not too strong)                                   |
/// | 4   | flag (check not busy = true)                                |
/// | 5   | flag (find keep hunting = true)                             |
/// | 6   | g1 (find new mode)                                         |
/// | 7   | g1 (nobody near)                                           |
/// | 8   | g2 (check not combat — varp id)                             |
/// | 9   | g2 (check not combat self — varn id)                        |
/// | 10  | flag (check afk = false)                                   |
/// | 11  | g2 (rate)                                                  |
/// | 12  | g2 (check category)                                        |
/// | 13  | g2 (check npc)                                             |
/// | 14  | g2 (check obj)                                             |
/// | 15  | g2 (check loc)                                             |
/// | 16  | g2+g2+gjstr+g4 (check inv obj)                             |
/// | 17  | g2+g2+gjstr+g4 (check inv param)                           |
/// | 18..=20 | g2+gjstr+g4 (check var)                                |
/// | 250 | gjstr (debugname)                                          |
pub struct HuntType {
    pub id: u16,
    pub hunt_type: u8,
    pub check_vis: u8,
    pub check_not_too_strong: u8,
    pub check_not_busy: bool,
    pub find_keep_hunting: bool,
    pub find_new_mode: u8,
    pub nobody_near: u8,
    pub check_not_combat: i32,
    pub check_not_combat_self: i32,
    pub check_afk: bool,
    pub rate: u16,
    pub check_category: i32,
    pub check_npc: i32,
    pub check_obj: i32,
    pub check_loc: i32,
    pub check_inv: i32,
    pub check_inv_obj: i32,
    pub check_obj_param: i32,
    pub check_inv_condition: String,
    pub check_inv_val: i32,
    /// op 16/17 discriminator: 16 = inv_obj, 17 = inv_param
    pub check_inv_opcode: u8,
    /// op 18..=20: (var_id, condition, value) triples
    pub check_vars: Vec<(u8, u16, String, i32)>,
    debugname: Option<Box<str>>,
}

impl CacheType for HuntType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            hunt_type: 0,
            check_vis: 0,
            check_not_too_strong: 0,
            check_not_busy: false,
            find_keep_hunting: false,
            find_new_mode: 0,
            nobody_near: 0,
            check_not_combat: -1,
            check_not_combat_self: -1,
            check_afk: true,
            rate: 1,
            check_category: -1,
            check_npc: -1,
            check_obj: -1,
            check_loc: -1,
            check_inv: -1,
            check_inv_obj: -1,
            check_obj_param: -1,
            check_inv_condition: String::new(),
            check_inv_val: -1,
            check_inv_opcode: 0,
            check_vars: Vec::new(),
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            match opcode {
                1 => self.hunt_type = buf.g1(),
                2 => self.check_vis = buf.g1(),
                3 => self.check_not_too_strong = buf.g1(),
                4 => self.check_not_busy = true,
                5 => self.find_keep_hunting = true,
                6 => self.find_new_mode = buf.g1(),
                7 => self.nobody_near = buf.g1(),
                8 => self.check_not_combat = buf.g2() as i32,
                9 => self.check_not_combat_self = buf.g2() as i32,
                10 => self.check_afk = false,
                11 => self.rate = buf.g2(),
                12 => self.check_category = buf.g2() as i32,
                13 => self.check_npc = buf.g2() as i32,
                14 => self.check_obj = buf.g2() as i32,
                15 => self.check_loc = buf.g2() as i32,
                16 => {
                    self.check_inv = buf.g2() as i32;
                    self.check_inv_obj = buf.g2() as i32;
                    self.check_inv_condition = buf.gjstr(0).to_string();
                    self.check_inv_val = buf.g4s();
                    self.check_inv_opcode = 16;
                }
                17 => {
                    self.check_inv = buf.g2() as i32;
                    self.check_obj_param = buf.g2() as i32;
                    self.check_inv_condition = buf.gjstr(0).to_string();
                    self.check_inv_val = buf.g4s();
                    self.check_inv_opcode = 17;
                }
                18..=20 => {
                    let var_id = buf.g2();
                    let condition = buf.gjstr(0).to_string();
                    let value = buf.g4s();
                    self.check_vars.push((opcode, var_id, condition, value));
                }
                250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
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
        _ => false,
    }
}
