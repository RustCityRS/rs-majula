#[cfg(since_530)]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(since_530)]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::client_prot_message::string_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

#[cfg(all(since_530, before_647))]
#[client_prot(VarByte { min: 10 }, UserEvent)]
#[derive(Debug)]
pub struct MessageQuickchatPrivate {
    pub user37: i64,
    pub id: TypeId,
    pub bytes: Vec<u8>,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for MessageQuickchatPrivate {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let user37 = buf.g8s();
        let id = TypeId::new(buf.g2());
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessageQuickchatPrivate { user37, id, bytes }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for MessageQuickchatPrivate {
    const PROT: ClientProt = ClientProt::MessageQuickchatPrivate;

    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p2(self.id.g2());
        buf.pdata(&self.bytes, 0, self.bytes.len());
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 3 }, UserEvent)]
#[derive(Debug)]
pub struct MessageQuickchatPrivate {
    pub name: String,
    pub id: TypeId,
    pub bytes: Vec<u8>,
}

#[cfg(since_647)]
impl ClientProtMessage for MessageQuickchatPrivate {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0).is_some_and(|end| end + 2 <= data.len())
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let name = buf.gjstr(STRING_TERMINATOR);
        let id = TypeId::new(buf.g2());
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessageQuickchatPrivate { name, id, bytes }
    }
}

#[cfg(since_647)]
impl ClientProtEncode for MessageQuickchatPrivate {
    const PROT: ClientProt = ClientProt::MessageQuickchatPrivate;

    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.name, STRING_TERMINATOR);
        buf.p2(self.id.g2());
        buf.pdata(&self.bytes, 0, self.bytes.len());
    }
}
