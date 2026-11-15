#![cfg(since_742)]

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(1), ClientEvent)]
pub struct Unknown1;

impl ClientProtMessage for Unknown1 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown1
    }
}

#[client_prot(Fixed(1), ClientEvent)]
pub struct Unknown2;

impl ClientProtMessage for Unknown2 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown2
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown10;

impl ClientProtMessage for Unknown10 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown10
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown14;

impl ClientProtMessage for Unknown14 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown14
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct Unknown15;

impl ClientProtMessage for Unknown15 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown15
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown29;

impl ClientProtMessage for Unknown29 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown29
    }
}

#[client_prot(Fixed(4), ClientEvent)]
pub struct Unknown60;

impl ClientProtMessage for Unknown60 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown60
    }
}

#[client_prot(Fixed(7), ClientEvent)]
pub struct Unknown61;

impl ClientProtMessage for Unknown61 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown61
    }
}

#[client_prot(Fixed(2), ClientEvent)]
pub struct Unknown63;

impl ClientProtMessage for Unknown63 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown63
    }
}

#[client_prot(Fixed(15), ClientEvent)]
pub struct Unknown89;

impl ClientProtMessage for Unknown89 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown89
    }
}

#[client_prot(Fixed(9), ClientEvent)]
pub struct Unknown91;

impl ClientProtMessage for Unknown91 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown91
    }
}

#[client_prot(Fixed(1), ClientEvent)]
pub struct Unknown92;

impl ClientProtMessage for Unknown92 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown92
    }
}

#[client_prot(Fixed(4), ClientEvent)]
pub struct Unknown93;

impl ClientProtMessage for Unknown93 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown93
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown97;

impl ClientProtMessage for Unknown97 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown97
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct Unknown98;

impl ClientProtMessage for Unknown98 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown98
    }
}

#[client_prot(Fixed(0), ClientEvent)]
pub struct Unknown100;

impl ClientProtMessage for Unknown100 {
    fn decode(_: &mut Packet, _: usize) -> Self {
        Unknown100
    }
}

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct Unknown101;

impl ClientProtMessage for Unknown101 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown101
    }
}

#[client_prot(VarShort { min: 0 }, ClientEvent)]
pub struct Unknown102;

impl ClientProtMessage for Unknown102 {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        buf.pos = len;
        Unknown102
    }
}
