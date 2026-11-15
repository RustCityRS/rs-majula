use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(all(since_530, before_647))]
const OTHER: i32 = -70001;

#[cfg(before_530)]
#[server_prot(UpdateInvFull, Immediate, VarShort)]
pub struct UpdateInvFull<'a> {
    pub com: u16,
    pub objs: &'a [Option<(TypeId, i32)>],
}

#[cfg(before_289)]
impl ServerProtMessage for UpdateInvFull<'_> {
    fn encode(&self, buf: &mut Packet) {
        // only send up to the last slot in use (anything beyond is cleared by the client)
        let max = self.find_max_pos();
        buf.p2(self.com);
        buf.p1(max as u8);
        for obj in &self.objs[..max] {
            match obj {
                None => {
                    buf.p2(0);
                    buf.p1(0);
                }
                Some(obj) => {
                    buf.p2(obj.0.g2().saturating_add(1));
                    let count = obj.1;
                    if count >= u8::MAX as i32 {
                        buf.p1(u8::MAX);
                        buf.p4(count);
                    } else {
                        buf.p1(count as u8);
                    }
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        let max = self.find_max_pos();
        size_of_val(&self.com)
            + 1
            + self.objs[..max]
                .iter()
                .map(|obj| match obj {
                    None => 3,
                    Some(obj) => 2 + if obj.1 >= u8::MAX as i32 { 5 } else { 1 },
                })
                .sum::<usize>()
    }
}

#[cfg(all(since_289, before_530))]
impl ServerProtMessage for UpdateInvFull<'_> {
    fn encode(&self, buf: &mut Packet) {
        // only send up to the last slot in use (anything beyond is cleared by the client)
        let max = self.find_max_pos();
        buf.p2(self.com);
        buf.p2(max as u16);
        for obj in &self.objs[..max] {
            match obj {
                None => {
                    buf.p2(0);
                    buf.p1(0);
                }
                Some(obj) => {
                    buf.p2(obj.0.g2().saturating_add(1));
                    let count = obj.1;
                    if count >= u8::MAX as i32 {
                        buf.p1(u8::MAX);
                        buf.p4(count);
                    } else {
                        buf.p1(count as u8);
                    }
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        let max = self.find_max_pos();
        size_of_val(&self.com)
            + 2
            + self.objs[..max]
                .iter()
                .map(|obj| match obj {
                    None => 3,
                    Some(obj) => 2 + if obj.1 >= u8::MAX as i32 { 5 } else { 1 },
                })
                .sum::<usize>()
    }
}

#[cfg(before_530)]
impl UpdateInvFull<'_> {
    fn find_max_pos(&self) -> usize {
        self.objs
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i + 1)
    }
}

/// One populated inventory slot. `None` in `UpdateInvFull::slots`
/// represents an empty slot.
#[cfg(since_530)]
#[derive(Debug, Clone, Copy)]
pub struct InvItem {
    pub id: TypeId,
    pub count: u32,
}

#[cfg(since_530)]
#[server_prot(UpdateInvFull, Immediate, VarShort)]
pub struct UpdateInvFull {
    #[cfg(all(since_530, before_647))]
    pub com: i32,
    pub inv_id: TypeId,
    /// The other player's copy of the inv (flags bit 0): the client keeps
    /// it apart from the player's own, as trade's other offer.
    pub other: bool,
    pub capacity: u16,
    pub slots: Vec<Option<InvItem>>,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for UpdateInvFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(if self.other { OTHER } else { self.com });
        buf.p2(self.inv_id.g2());
        buf.p2(self.capacity);
        for slot in &self.slots {
            match slot {
                Some(item) => {
                    if item.count >= 255 {
                        buf.p1_alt3(255);
                        buf.p4(item.count as i32);
                    } else {
                        buf.p1_alt3(item.count as u8);
                    }
                    buf.p2(item.id.g2() + 1);
                }
                None => {
                    buf.p1_alt3(0);
                    buf.p2(0);
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of::<u16>()
            + size_of_val(&self.capacity)
            + self
                .slots
                .iter()
                .map(|slot| match slot {
                    Some(item) if item.count >= 255 => 7,
                    _ => 3,
                })
                .sum::<usize>()
    }
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateInvFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.inv_id.g2());
        buf.p1(self.other as u8);
        buf.p2(self.capacity);
        for slot in &self.slots {
            match slot {
                Some(item) => {
                    if item.count >= 255 {
                        buf.p1(255);
                        #[cfg(before_742)]
                        buf.p4_alt1(item.count as i32);
                        #[cfg(since_742)]
                        buf.p4(item.count as i32);
                    } else {
                        buf.p1(item.count as u8);
                    }
                    let id_plus_one = item.id.g2() + 1;
                    #[cfg(before_742)]
                    {
                        buf.p1((id_plus_one >> 8) as u8);
                        buf.p1_alt1(id_plus_one as u8);
                    }
                    #[cfg(since_742)]
                    buf.p2_alt3(id_plus_one);
                }
                None => {
                    buf.p1(0);
                    #[cfg(before_742)]
                    {
                        buf.p1(0);
                        buf.p1_alt1(0);
                    }
                    #[cfg(since_742)]
                    buf.p2_alt3(0);
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        size_of::<u16>()
            + size_of_val(&self.other)
            + size_of_val(&self.capacity)
            + self
                .slots
                .iter()
                .map(|slot| match slot {
                    Some(item) if item.count >= 255 => 7,
                    _ => 3,
                })
                .sum::<usize>()
    }
}
