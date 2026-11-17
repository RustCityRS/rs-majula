#[cfg(all(since_530, before_647))]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(all(since_530, before_647))]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(rev = "225")]
#[client_prot(Fixed(6), UserEvent)]
pub struct InvButtonD {
    pub com: u16,
    pub slot: u16,
    pub slot2: u16,
}

#[cfg(rev = "225")]
impl ClientProtMessage for InvButtonD {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        InvButtonD {
            com: buf.g2(),
            slot: buf.g2(),
            slot2: buf.g2(),
        }
    }
}

#[cfg(since_244)]
#[cfg_attr(before_530, client_prot(Fixed(7), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(9), UserEvent))]
#[derive(Debug)]
pub struct InvButtonD {
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
    pub slot: u16,
    pub slot2: u16,
    pub mode: u8,
}

#[cfg(all(since_244, before_530))]
impl ClientProtMessage for InvButtonD {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        InvButtonD {
            com: buf.g2(),
            slot: buf.g2(),
            slot2: buf.g2(),
            mode: buf.g1(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for InvButtonD {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let slot = buf.g2();
        let com = buf.g4_alt1();
        let slot2 = buf.g2_alt2();
        let mode = buf.g1_alt3();
        InvButtonD {
            com,
            slot,
            slot2,
            mode,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for InvButtonD {
    const PROT: ClientProt = ClientProt::InvButtonD;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.slot);
        buf.p4_alt1(self.com);
        buf.p2_alt2(self.slot2);
        buf.p1_alt3(self.mode);
    }
}
