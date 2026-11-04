//! Every Fixed-frame client packet's decoder reads exactly its frame.
//!
//! The engine dispatches with the structs `client_prot` re-exports -- the same
//! ones `ClientProt::info()` frames by -- so this holds for what the server
//! actually runs, not just for structs a test chose. At rev 647 it did not:
//! the engine decoded four targeted ops, chat modes and friend/ignore edits
//! with their pre-647 structs, reading 8 bytes of a 15-byte frame or 3 of 1.
//!
//! Runs for whichever revision `REV` selects. Before 647 the anticheat and
//! event packets are deliberately decoded as nothing at all -- the engine
//! drains the frame either way -- so a decoder there may also read zero bytes.
//! What no revision may do is read past its frame.

use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::client_prot_frame::ClientProtFrame;

#[test]
fn every_fixed_decoder_reads_exactly_its_frame() {
    let mut wrong = Vec::new();
    for prot in ClientProt::ALL {
        let ClientProtFrame::Fixed(size) = prot.info().frame else {
            continue;
        };
        let size = size as usize;
        // Zero-filled, with slack past the frame so an over-read lands on
        // bytes we own and shows up in the count instead of in memory.
        let data = vec![0u8; size + 256];
        let read = prot.decoded_len(&data, size);
        let discarded = cfg!(before_647) && read == 0;
        if read != size && !discarded {
            wrong.push(format!("{prot:?} (Fixed {size}) read {read}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "{} client decoder(s) disagree with their frame:\n  {}",
        wrong.len(),
        wrong.join("\n  ")
    );
}
