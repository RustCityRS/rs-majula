#[cfg(since_254)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_254)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_254)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(before_647)]
use crate::network::game::server_prot_message::jstr_len;
#[cfg(since_254)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_254)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_254)]
use rs_protocol_macros::server_prot;

#[cfg(all(since_254, before_647))]
#[server_prot(SetPlayerOp, Buffered, VarByte)]
pub struct SetPlayerOp<'a> {
    pub op: u8,
    pub value: &'a str,
    pub primary: u8,
}

#[cfg(all(since_254, before_647))]
impl ServerProtMessage for SetPlayerOp<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.op);
        buf.p1(self.primary);
        buf.pjstr(self.value, 10);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.op) + size_of_val(&self.primary) + jstr_len(self.value)
    }
}

// `SetPlayerOp` — assign a right-click option (1-8) on the local player.
// Rev-647 opcode 84 (SET_PLAYER_OP), VarByte framing.
// Layout: `[p2_alt1 cursor][p1_alt2 primary][pstr text][p1_alt2 index]`.
//
// `cursor` is unused by 2004scape; we always send `0xFFFF` (= -1 in client).
// `primary` matches 2004scape semantics: client treats `0` as primary, `1` as
// secondary (`aBoolArray7[index-1] = primary == 0`).

#[cfg(since_647)]
#[server_prot(SetPlayerOp, Buffered, VarByte)]
pub struct SetPlayerOp {
    pub index: u8,
    pub text: String,
    pub primary: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for SetPlayerOp {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(0xFFFF);
        buf.p1_alt2(self.primary);
        buf.pjstr(&self.text, 0);
        buf.p1_alt2(self.index);
    }

    fn sizeof(&self) -> usize {
        2 + 1 + self.text.len() + 1 + 1
    }
}
