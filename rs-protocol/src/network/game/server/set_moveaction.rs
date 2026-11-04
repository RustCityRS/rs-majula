//! `SetMoveaction` — override the "Walk here" right-click option.
//! Rev-647 opcode 83 (SET_MOVEACTION), VarByte framing.
//!
//! NOTE: the client decodes this conditionally on `FloTypeList.anInt2651`
//! (HD detail level): `> 2` reads the option label string, `> 0` reads a
//! cursor id. The server doesn't know the client's HD tier up front, so
//! we always emit both fields (the "full" form). For the
//! low-detail-client case the client will misinterpret the payload —
//! callers should only emit this when they know the client is HD.
//!
//! Client read order (`MapAreaConfig.java`):
//! `if HD > 2: gjstr label; if HD > 0: g2 cursor_id`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(SetMoveaction, Buffered, VarByte)]
pub struct SetMoveaction {
    /// Option label (e.g. "Climb up").
    pub label: String,
    /// Cursor id (65535 / -1 for default).
    pub cursor_id: u16,
}

impl ServerProtMessage for SetMoveaction {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(0);
        buf.pjstr(&self.label, 0);
        buf.p2(self.cursor_id);
    }

    fn sizeof(&self) -> usize {
        // pjstr2 writes [0x00][bytes][0x00] (len + 2), plus p2 (+ 2).
        self.label.len() + 2 + 2
    }
}
