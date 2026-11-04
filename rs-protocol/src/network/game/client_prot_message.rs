use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;

pub trait ClientProtMessageInfo {
    const FRAME: ClientProtFrame;
    const CATEGORY: ClientProtCategory;
}

pub trait ClientProtMessage: ClientProtMessageInfo {
    fn validate(data: &[u8]) -> bool {
        match Self::FRAME {
            ClientProtFrame::Fixed(len) => data.len() == len as usize,
            ClientProtFrame::VarByte { min } | ClientProtFrame::VarShort { min } => {
                data.len() >= min as usize
            }
        }
    }

    /// Decodes a payload that has passed `validate`, with `len` bytes remaining.
    fn decode(buf: &mut Packet, len: usize) -> Self;
}

#[cfg(since_647)]
pub(crate) fn string_end(data: &[u8], start: usize) -> Option<usize> {
    data.get(start..)?
        .iter()
        .position(|&b| b == 0)
        .map(|len| start + len + 1)
}

#[cfg(since_647)]
pub(crate) fn smart_end(data: &[u8], start: usize) -> Option<usize> {
    let size = if *data.get(start)? < 128 { 1 } else { 2 };
    (start + size <= data.len()).then_some(start + size)
}
