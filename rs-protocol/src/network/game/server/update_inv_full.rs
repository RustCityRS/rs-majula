use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

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

#[cfg(since_289)]
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

impl UpdateInvFull<'_> {
    fn find_max_pos(&self) -> usize {
        self.objs
            .iter()
            .rposition(Option::is_some)
            .map_or(0, |i| i + 1)
    }
}
