//! `SetChatfiltersettings` — rev-647 SET_CHATFILTERSETTINGS (opcode 10, fixed 3).
//!
//! Sent when the player changes their public / private / trade chat privacy
//! filters. The client builds it from CS2 op 5001 (`Class47.java`):
//! `p1(public) · p1(private) · p1(trade)`, where each value is one of the
//! per-channel filter ids (public 0..3 On/Friends/Off/Hide, private/trade
//! 0..2 On/Friends/Off — see `FilterPrivate`).
//!
//! NOTE (rev divergence): this is a *separate* packet from `CHAT_SETMODE`
//! (opcode 61), which in rev-647 is the single-byte public/clan chat-type
//! toggle — not the privacy filters. Older revisions (e.g. Jordan's rev-225)
//! packed all three filters into `CHAT_SETMODE`; rev-647 does not.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct SetChatfiltersettings {
    pub public: u8,
    pub private: u8,
    pub trade: u8,
}

impl ClientProtMessage for SetChatfiltersettings {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let public = buf.g1();
        let private = buf.g1();
        let trade = buf.g1();
        SetChatfiltersettings {
            public,
            private,
            trade,
        }
    }
}
