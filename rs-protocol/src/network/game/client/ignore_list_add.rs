//! `IgnoreListAdd` — rev-647 IGNORELIST_ADD (opcode 85, var_byte).
//!
//! Client write order (`Statics1.method3487`): `pjstr(name)`, then
//! `p1(flag ? 1 : 0)` -- the one social edit with a byte after the name. The
//! flag is the method's boolean: CS2 op 3607 sends `false` and op 3630
//! `true`. What 3630's ignore differs in is not established here, so the
//! flag is carried as read and not yet acted on.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct IgnoreListAdd {
    pub name: String,
    /// Set when the add came from CS2 op 3630 rather than op 3607.
    pub op3630: bool,
}

impl ClientProtMessage for IgnoreListAdd {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let name = buf.gjstr(0);
        let op3630 = buf.g1() == 1;
        IgnoreListAdd { name, op3630 }
    }
}
