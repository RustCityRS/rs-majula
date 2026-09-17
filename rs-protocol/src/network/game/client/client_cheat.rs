use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_io::cp1252::decode_string;
use rs_protocol_macros::client_prot;

// Minimum covers the string terminator.
#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct ClientCheat {
    pub cheat: String,
}

impl ClientProtMessage for ClientCheat {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        // Not `gjstr`: it scans unchecked until it finds a terminator, so a
        // payload without one walks off the end of the buffer.
        let start = buf.pos;
        let end = (start + len).min(buf.data.len());
        let text = &buf.data[start..end];
        let term = text.iter().position(|&b| b == 10);
        buf.pos = term.map_or(end, |i| start + i + 1);
        ClientCheat {
            cheat: decode_string(&text[..term.unwrap_or(text.len())]),
        }
    }
}
