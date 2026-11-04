//! VarBitType — CacheType implementation for variable bit definitions.
//!
//! Decoded from archive 22 (CONFIG_VARBIT), rev 647.
//! Mirrors the client's `VarbitType.decode`.

use rs_io::Packet;

use super::provider::CacheType;

/// VarBit definition.
#[derive(Debug, Clone)]
pub struct VarBitType {
    pub id: u16,
    pub varp_id: u16,
    pub lsb: u8,
    pub msb: u8,
}

impl CacheType for VarBitType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            varp_id: 0,
            lsb: 0,
            msb: 0,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            match opcode {
                1 => {
                    self.varp_id = buf.g2();
                    self.lsb = buf.g1();
                    self.msb = buf.g1();
                }
                _ => panic!("VarBitType: unknown opcode {opcode} at id {}", self.id),
            }
        }
    }
}

impl VarBitType {
    /// Get the bit mask.
    pub fn mask(&self) -> u32 {
        ((1u32 << (self.msb - self.lsb + 1)) - 1) << self.lsb
    }

    /// Extract the value from a varp.
    pub fn get_value(&self, varp_value: i32) -> i32 {
        (varp_value >> self.lsb) & ((1 << (self.msb - self.lsb + 1)) - 1)
    }

    /// Set the value within a varp.
    pub fn set_value(&self, varp_value: i32, value: i32) -> i32 {
        let mask = self.mask() as i32;
        (varp_value & !mask) | ((value << self.lsb) & mask)
    }
}
