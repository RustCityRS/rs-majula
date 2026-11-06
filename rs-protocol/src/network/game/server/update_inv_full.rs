use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(UpdateInvFull, Immediate, VarShort)]
pub struct UpdateInvFull<'a> {
    pub com: u16,
    pub objs: &'a [Option<(u16, i32)>],
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
                    buf.p2(obj.0.saturating_add(1));
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

#[cfg(all(since_289, before_647))]
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
                    buf.p2(obj.0.saturating_add(1));
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

#[cfg(before_647)]
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
#[cfg(since_647)]
#[derive(Debug, Clone, Copy)]
pub struct InvItem {
    pub id: u16,
    pub count: u32,
}

#[cfg(since_647)]
#[server_prot(UpdateInvFull, Buffered, VarShort)]
pub struct UpdateInvFull {
    pub inv_id: u16,
    /// The other player's copy of the inv (flags bit 0): the client keeps
    /// it apart from the player's own, as trade's other offer.
    pub other: bool,
    pub capacity: u16,
    pub slots: Vec<Option<InvItem>>,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateInvFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.inv_id);
        buf.p1(self.other as u8);
        buf.p2(self.capacity);
        for slot in &self.slots {
            match slot {
                Some(item) => {
                    if item.count >= 255 {
                        buf.p1(255);
                        buf.p4_alt1(item.count as i32);
                    } else {
                        buf.p1(item.count as u8);
                    }
                    let id_plus_one = item.id + 1;
                    buf.p1((id_plus_one >> 8) as u8);
                    buf.p1_alt1(id_plus_one as u8);
                }
                None => {
                    buf.p1(0);
                    buf.p1(0);
                    buf.p1_alt1(0);
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        // 5-byte header + 3–7 bytes per slot; oversize slightly so the
        // buffer is never under-allocated for sparse / dense inventories.
        (3 + self.capacity as usize * 7).max(64)
    }
}
