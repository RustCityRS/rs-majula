//! A port of the rev-647 client's `Packet` writer, line for line from the
//! deob-647 client's `Packet.java`.
//!
//! Shared by the inbound wire suite and the headless handler tests
//! (`rs-test`), so a test lays down a packet's bytes in the order the
//! client's send site writes them. Included with `#[path]`; it is not a test
//! target of its own.

/// The writer port is deliberately complete (it mirrors the client's `Packet`),
/// so some writers have no caller until the matching packet gains a test.
#[allow(dead_code)]
#[derive(Default)]
pub struct ClientPacket {
    buf: Vec<u8>,
}

#[allow(dead_code)]
impl ClientPacket {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    /// `buffer[pos++] = (byte) v`
    pub fn p1(&mut self, v: i32) -> &mut Self {
        self.buf.push(v as u8);
        self
    }

    /// `buffer[pos++] = (byte) -v`
    pub fn p1_alt2(&mut self, v: i32) -> &mut Self {
        self.buf.push((-v) as u8);
        self
    }

    /// `buffer[pos++] = (byte) (128 - v)`
    pub fn p1_alt3(&mut self, v: i32) -> &mut Self {
        self.buf.push((128 - v) as u8);
        self
    }

    /// `buffer[pos++] = (byte) (v + 128)`
    pub fn p1_alt1(&mut self, v: i32) -> &mut Self {
        self.buf.push((v + 128) as u8);
        self
    }

    /// Big-endian 16.
    pub fn p2(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
        self
    }

    /// Little-endian 16.
    pub fn p2_alt1(&mut self, v: i32) -> &mut Self {
        self.buf.push(v as u8);
        self.buf.push((v >> 8) as u8);
        self
    }

    /// Big-endian 16, low byte biased by 128.
    pub fn p2_alt2(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 8) as u8);
        self.buf.push((v + 128) as u8);
        self
    }

    /// Little-endian 16, low byte biased by 128.
    pub fn p2_alt3(&mut self, v: i32) -> &mut Self {
        self.buf.push((v + 128) as u8);
        self.buf.push((v >> 8) as u8);
        self
    }

    pub fn p3(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
        self
    }

    /// Big-endian 32.
    pub fn p4(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 24) as u8);
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
        self
    }

    /// Little-endian 32.
    pub fn p4_alt1(&mut self, v: i32) -> &mut Self {
        self.buf.push(v as u8);
        self.buf.push((v >> 8) as u8);
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 24) as u8);
        self
    }

    /// Middle-endian 32: `>>16, >>24, v, >>8`.
    pub fn p4_alt3(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 16) as u8);
        self.buf.push((v >> 24) as u8);
        self.buf.push(v as u8);
        self.buf.push((v >> 8) as u8);
        self
    }

    /// Middle-endian 32: `>>8, v, >>24, >>16`.
    pub fn p4_alt2(&mut self, v: i32) -> &mut Self {
        self.buf.push((v >> 8) as u8);
        self.buf.push(v as u8);
        self.buf.push((v >> 24) as u8);
        self.buf.push((v >> 16) as u8);
        self
    }

    pub fn p8(&mut self, v: i64) -> &mut Self {
        for shift in (0..64).step_by(8).rev() {
            self.buf.push((v >> shift) as u8);
        }
        self
    }

    /// `pjstr`: CP-1252 bytes then a NUL terminator.
    pub fn pjstr(&mut self, s: &str) -> &mut Self {
        self.buf.extend(cp1252(s));
        self.buf.push(0);
        self
    }

    /// `pjstr2`: a leading NUL, then CP-1252 bytes, then a NUL terminator.
    pub fn pjstr2(&mut self, s: &str) -> &mut Self {
        self.buf.push(0);
        self.buf.extend(cp1252(s));
        self.buf.push(0);
        self
    }

    pub fn finish(&self) -> Vec<u8> {
        self.buf.clone()
    }
}

/// The subset of CP-1252 the tests use. The client packs strings through
/// `AClass23_Sub4.pack`; every character in these fixtures is ASCII, where
/// CP-1252 and ASCII agree.
fn cp1252(s: &str) -> Vec<u8> {
    assert!(
        s.is_ascii(),
        "test fixtures stay ASCII so the CP-1252 packing is not itself under test"
    );
    s.as_bytes().to_vec()
}
