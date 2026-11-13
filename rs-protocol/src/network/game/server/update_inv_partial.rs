use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(UpdateInvPartial, Immediate, VarShort)]
pub struct UpdateInvPartial<'a> {
    #[cfg(before_647)]
    pub com: u16,
    #[cfg(since_647)]
    pub inv_id: u16,
    #[cfg(since_647)]
    pub other: bool,
    pub objs: &'a [(u16, Option<(u16, i32)>)],
}

impl ServerProtMessage for UpdateInvPartial<'_> {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_647)]
        buf.p2(self.com);
        #[cfg(since_647)]
        {
            buf.p2(self.inv_id);
            buf.p1(self.other as u8);
        }
        for (slot, obj) in self.objs {
            #[cfg(before_647)]
            buf.p1(*slot as u8);
            #[cfg(since_647)]
            buf.psmart1or2(*slot as i32);

            match obj {
                None => {
                    buf.p2(0);
                    #[cfg(before_647)]
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
        #[cfg(before_647)]
        let header = size_of_val(&self.com);
        #[cfg(since_647)]
        let header = size_of_val(&self.inv_id) + size_of_val(&self.other);
        header
            + self
                .objs
                .iter()
                .map(|(_slot, obj)| {
                    #[cfg(before_647)]
                    let slot_size = 1;
                    #[cfg(since_647)]
                    let slot_size = if *_slot < 128 { 1 } else { 2 };
                    slot_size
                        + match obj {
                            #[cfg(before_647)]
                            None => 3,
                            #[cfg(since_647)]
                            None => 2,
                            Some(obj) => 2 + if obj.1 >= u8::MAX as i32 { 5 } else { 1 },
                        }
                })
                .sum::<usize>()
    }
}
