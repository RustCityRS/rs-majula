//! Outbound packet encoding — `Buffered` vs `Immediate` split, matching
//! Jordan's `Player::write` flow.
//!
//! ## Buffered (default)
//!
//! [`encode_buffered`] returns a [`BufferedPacket`] holding the encoded
//! payload + size header + **plaintext** opcode byte(s) at the front.
//! The engine pushes these into `Player::buffered`. At end-of-tick, a
//! flush walks the queue, ISAAC-encrypts each `BufferedPacket`'s
//! opcode bytes in place, and pushes the result onto the wire.
//!
//! ## Immediate
//!
//! [`encode_immediate`] ISAAC-encrypts and writes the bytes
//! synchronously into the player's outbound buffer — used for bulk
//! transfers and chat (`MessageGame`, `RebuildNormal`, `RebuildRegion`).
//!
//! Both paths share the size-framing convention: size header bytes
//! (1 for VarByte, 2 for VarShort) are reserved between the opcode and
//! payload, then back-filled with the actual encoded length.

use rs_crypto::isaac::Isaac;
use rs_io::{Packet, PacketFrame};

use crate::network::game::server_prot_message::ServerProtMessage;

/// Hard cap on a single server-to-client frame (opcode + size header + payload).
/// Matches the rev-647 client's inbound `input_packet = BitPacket(15000)` buffer
/// (xlite2-client `BASTypeList.java:6`). Anything larger overruns the client's
/// read buffer and desyncs ISAAC, so we drop it server-side.
pub const MAX_SERVER_FRAME_SIZE: usize = 15000;

/// One outbound packet queued for end-of-tick flush.
///
/// Holds the fully-encoded bytes, with the leading **plaintext**
/// opcode byte(s) (1 for opcode < 128, 2 for opcode >= 128). The
/// flush step ISAAC-encrypts those leading bytes and ships the
/// payload.
pub struct BufferedPacket {
    /// Encoded bytes: `[opcode (1 or 2)][size header (0/1/2)][payload]`.
    pub bytes: Vec<u8>,
    /// Number of opcode bytes at the start (1 or 2). The flush
    /// ISAAC-encrypts exactly this many bytes.
    pub opcode_bytes: u8,
}

/// Number of wire bytes the opcode itself takes (1 if < 128, else 2).
#[inline]
const fn opcode_bytes(opcode: u8) -> u8 {
    if opcode < 128 { 1 } else { 2 }
}

/// Number of size-header bytes for a frame kind.
#[inline]
const fn frame_bytes(frame: PacketFrame) -> u8 {
    match frame {
        PacketFrame::Fixed => 0,
        PacketFrame::VarByte => 1,
        PacketFrame::VarShort => 2,
    }
}

/// Build a `BufferedPacket` for `message` — opcode is left **plaintext**;
/// the caller (engine flush) ISAAC-encrypts the leading byte(s) before
/// pushing to the network.
pub fn encode_buffered<M: ServerProtMessage>(message: &M) -> Option<BufferedPacket> {
    let opcode = M::PROT.id();
    let opc_bytes = opcode_bytes(opcode) as usize;
    let frm_bytes = frame_bytes(M::FRAME) as usize;
    let payload_len = message.sizeof();
    let total = opc_bytes + frm_bytes + payload_len;

    if total > MAX_SERVER_FRAME_SIZE {
        tracing::error!(
            opcode,
            total,
            max = MAX_SERVER_FRAME_SIZE,
            "dropping oversized buffered frame"
        );
        return None;
    }

    let mut bytes = Vec::with_capacity(total);

    // Opcode (plaintext — flush will ISAAC-encrypt these byte(s) in place).
    if opc_bytes == 1 {
        bytes.push(opcode);
    } else {
        bytes.push(((opcode as u16) >> 8) as u8 + 128);
        bytes.push(opcode);
    }

    // Reserve size-header slot — back-filled after encoding the payload.
    let header_start = bytes.len();
    bytes.resize(header_start + frm_bytes, 0);
    let payload_start = bytes.len();

    // Encode payload into a fresh Packet sized to `sizeof()`, then splice.
    let mut buf = Packet::new(payload_len);
    message.encode(&mut buf);
    let actual_payload = &buf.data[..buf.pos];

    // Hard guard: encode wrote more than the sizeof() hint promised.
    debug_assert!(
        actual_payload.len() <= payload_len,
        "encode wrote {} bytes but sizeof() said {}",
        actual_payload.len(),
        payload_len
    );

    bytes.extend_from_slice(actual_payload);

    // Back-fill the size header.
    let written = bytes.len() - payload_start;
    match M::FRAME {
        PacketFrame::Fixed => {}
        PacketFrame::VarByte => {
            bytes[header_start] = written as u8;
        }
        PacketFrame::VarShort => {
            bytes[header_start] = (written >> 8) as u8;
            bytes[header_start + 1] = written as u8;
        }
    }

    Some(BufferedPacket {
        bytes,
        opcode_bytes: opc_bytes as u8,
    })
}

/// ISAAC-encrypt the leading opcode byte(s) of a `BufferedPacket` in
/// place. Called once per packet at end-of-tick flush; the caller then
/// drains `bp.bytes` into the network outbox.
pub fn isaac_encrypt_opcode(bp: &mut BufferedPacket, isaac: &mut Isaac) {
    for i in 0..bp.opcode_bytes as usize {
        bp.bytes[i] = bp.bytes[i].wrapping_add(isaac.next_int() as u8);
    }
}

/// Encode `message` ISAAC-encrypted and append the resulting bytes to
/// `out`. Used for `Immediate`-priority packets — does not buffer.
pub fn encode_immediate<M: ServerProtMessage>(isaac: &mut Isaac, out: &mut Vec<u8>, message: &M) {
    let opcode = M::PROT.id();
    let payload_len = message.sizeof();

    // Encode payload first so we know its real length for var packets.
    let mut buf = Packet::new(payload_len);
    message.encode(&mut buf);
    let payload = &buf.data[..buf.pos];

    let opc_bytes = opcode_bytes(opcode) as usize;
    let frm_bytes = frame_bytes(M::FRAME) as usize;
    let total = opc_bytes + frm_bytes + payload.len();

    if total > MAX_SERVER_FRAME_SIZE {
        tracing::error!(
            opcode,
            total,
            max = MAX_SERVER_FRAME_SIZE,
            "dropping oversized immediate frame"
        );
        return;
    }

    encode_server_packet(out, opcode as u16, payload, M::FRAME, isaac);
}

/// Low-level: ISAAC-encrypt the opcode (1 or 2 bytes per RS protocol),
/// write the size header per `PacketFrame`, then append the payload.
pub fn encode_server_packet(
    out: &mut Vec<u8>,
    opcode: u16,
    data: &[u8],
    frame: PacketFrame,
    isaac: &mut Isaac,
) {
    out.reserve(2 + 2 + data.len());
    if opcode < 128 {
        let encoded = (opcode as u8).wrapping_add(isaac.next_int() as u8);
        out.push(encoded);
    } else {
        let hi = ((opcode >> 8) as u8 + 128).wrapping_add(isaac.next_int() as u8);
        let lo = (opcode as u8).wrapping_add(isaac.next_int() as u8);
        out.push(hi);
        out.push(lo);
    }
    match frame {
        PacketFrame::Fixed => {}
        PacketFrame::VarByte => out.push(data.len() as u8),
        PacketFrame::VarShort => {
            let len = data.len() as u16;
            out.push((len >> 8) as u8);
            out.push(len as u8);
        }
    }
    out.extend_from_slice(data);
}
