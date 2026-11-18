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
use rs_io::stringtools::cp1252_to_utf8;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg_attr(before_647, client_prot(VarByte { min: 1 }, UserEvent))]
#[cfg_attr(since_647, client_prot(VarByte { min: 3 }, UserEvent))]
#[derive(Debug)]
pub struct ClientCheat {
    pub cheat: String,
    #[cfg(since_647)]
    pub flags: [u8; 2],
}

impl ClientProtMessage for ClientCheat {
    #[cfg(since_647)]
    fn validate(data: &[u8]) -> bool {
        string_end(data, 2) == Some(data.len())
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        #[cfg(since_647)]
        let flags = [buf.g1(), buf.g1()];
        let start = buf.pos;
        let end = len.min(buf.data.len());
        let text = &buf.data[start..end];
        let term = text.iter().position(|&b| b == STRING_TERMINATOR);
        buf.pos = term.map_or(end, |i| start + i + 1);
        ClientCheat {
            cheat: cp1252_to_utf8(&text[..term.unwrap_or(text.len())]),
            #[cfg(since_647)]
            flags,
        }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for ClientCheat {
    const PROT: ClientProt = ClientProt::ClientCheat;

    fn encode(&self, buf: &mut Packet) {
        #[cfg(since_647)]
        {
            buf.p1(self.flags[0]);
            buf.p1(self.flags[1]);
        }
        buf.pjstr(&self.cheat, STRING_TERMINATOR);
    }
}
