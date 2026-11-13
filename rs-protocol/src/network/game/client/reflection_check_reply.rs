use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_io::crc::getcrc;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 8 }, ClientEvent)]
pub struct ReflectionCheckReply {
    pub id: i32,
    pub bytes: Vec<u8>,
    pub crc: i32,
}

/// What the client read for one check of a `REFLECTION_CHECKER` request, as
/// `AClass31_Sub1.method2357` writes it: the int (a field's value or a
/// member's modifiers), or the code of what went wrong, from -1 (no such
/// class) to -21.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReflectionResult {
    Value(i32),
    Error(i8),
}

impl ReflectionCheckReply {
    /// Whether the CRC32 the client appends matches the id and results.
    pub fn crc_matches(&self) -> bool {
        let mut covered = Packet::new(4 + self.bytes.len());
        covered.p4(self.id);
        covered.pdata(&self.bytes, 0, self.bytes.len());
        getcrc(&covered.data, 0, covered.pos) == self.crc
    }

    /// The results of a request of `checks` checks, every one of which
    /// answers with an int, or `None` when the bytes hold anything else.
    pub fn results(&self, checks: usize) -> Option<Vec<ReflectionResult>> {
        let mut buf = Packet::from(self.bytes.clone());
        let mut results = Vec::with_capacity(checks);
        for _ in 0..checks {
            if buf.remaining() < 1 {
                return None;
            }
            results.push(match buf.g1s() {
                0 if buf.remaining() >= 4 => ReflectionResult::Value(buf.g4s()),
                code @ (-6..=-1 | -21..=-10) => ReflectionResult::Error(code),
                _ => return None,
            });
        }
        (buf.remaining() == 0).then_some(results)
    }
}

impl ClientProtMessage for ReflectionCheckReply {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let id = buf.g4s();
        let bytes = buf.data[buf.pos..len - 4].to_vec();
        buf.pos = len - 4;
        let crc = buf.g4s();
        ReflectionCheckReply { id, bytes, crc }
    }
}
