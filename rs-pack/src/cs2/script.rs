//! Binary format reader for CS2 scripts.
//!
//! Layout (from rev-647 client `FileSystem.method651`):
//!   - Last 2 bytes: `switch_section_size` (g2)
//!   - Metadata at `len - switch_section_size - 16 - 2`:
//!     instruction_count (g4), local_int_count (g2), local_string_count (g2),
//!     local_long_count (g2), int_param_count (g2), string_param_count (g2),
//!     long_param_count (g2)
//!   - Switch tables: count (g1), per table: case_count (g2),
//!     per case: key (g4) + target (g4)
//!   - Position 0: name string (null-terminated, may be empty)
//!   - Instructions until metadata: opcode (g2) + operand per instruction
//!
//! Operand format per opcode:
//!   - opcode 3 (PUSH_CONSTANT_STRING): null-terminated string
//!   - opcode 54 (PUSH_CONSTANT_LONG): 8-byte signed long
//!   - opcode >= 150 or opcode in {21, 38, 39}: g1 (unsigned byte)
//!   - all others: g4 (signed 4-byte int)

use anyhow::Result;
use rs_io::Packet;

/// A decoded CS2 script ready for interpretation.
pub struct Script {
    pub name: String,
    pub local_int_count: u16,
    pub local_string_count: u16,
    pub local_long_count: u16,
    pub int_param_count: u16,
    pub string_param_count: u16,
    pub long_param_count: u16,
    /// Raw opcodes, one per instruction.
    pub opcodes: Vec<u16>,
    /// Integer operand per instruction (default 0 for string/long ops).
    pub int_operands: Vec<i32>,
    /// String operand per instruction (Some only for PUSH_CONSTANT_STRING).
    pub string_operands: Vec<Option<String>>,
    /// Long operand per instruction (Some only for PUSH_CONSTANT_LONG).
    pub long_operands: Vec<Option<i64>>,
    /// Switch tables: `switches[table_index]` = vec of (key, offset).
    pub switches: Vec<Vec<(i32, i32)>>,
}

impl Script {
    /// Decode just the parameter counts from the metadata trailer.
    pub fn decode_params(data: &[u8]) -> Result<(u16, u16, u16)> {
        if data.len() < 18 {
            anyhow::bail!("script too short: {} bytes", data.len());
        }
        let len = data.len();
        let switch_section_size = u16::from_be_bytes([data[len - 2], data[len - 1]]) as usize;
        let meta_start = len - switch_section_size - 16 - 2;
        let int_param_count = u16::from_be_bytes([data[meta_start + 10], data[meta_start + 11]]);
        let string_param_count = u16::from_be_bytes([data[meta_start + 12], data[meta_start + 13]]);
        let long_param_count = u16::from_be_bytes([data[meta_start + 14], data[meta_start + 15]]);
        Ok((int_param_count, string_param_count, long_param_count))
    }

    /// Decode a CS2 script from its raw binary representation.
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() < 18 {
            anyhow::bail!("script too short: {} bytes", data.len());
        }

        let len = data.len();

        // Last 2 bytes: switch section size
        let switch_section_size = u16::from_be_bytes([data[len - 2], data[len - 1]]) as usize;

        // Metadata trailer (16 bytes)
        let meta_start = len - switch_section_size - 16 - 2;
        let instruction_count = i32::from_be_bytes([
            data[meta_start],
            data[meta_start + 1],
            data[meta_start + 2],
            data[meta_start + 3],
        ]) as usize;
        let local_int_count = u16::from_be_bytes([data[meta_start + 4], data[meta_start + 5]]);
        let local_string_count = u16::from_be_bytes([data[meta_start + 6], data[meta_start + 7]]);
        let local_long_count = u16::from_be_bytes([data[meta_start + 8], data[meta_start + 9]]);
        let int_param_count = u16::from_be_bytes([data[meta_start + 10], data[meta_start + 11]]);
        let string_param_count = u16::from_be_bytes([data[meta_start + 12], data[meta_start + 13]]);
        let long_param_count = u16::from_be_bytes([data[meta_start + 14], data[meta_start + 15]]);

        // Switch tables
        let mut spos = meta_start + 16;
        let switch_count = data[spos] as usize;
        spos += 1;
        let mut switches = Vec::with_capacity(switch_count);
        for _ in 0..switch_count {
            let case_count = u16::from_be_bytes([data[spos], data[spos + 1]]) as usize;
            spos += 2;
            let mut cases = Vec::with_capacity(case_count);
            for _ in 0..case_count {
                let key = i32::from_be_bytes([
                    data[spos],
                    data[spos + 1],
                    data[spos + 2],
                    data[spos + 3],
                ]);
                spos += 4;
                let target = i32::from_be_bytes([
                    data[spos],
                    data[spos + 1],
                    data[spos + 2],
                    data[spos + 3],
                ]);
                spos += 4;
                cases.push((key, target));
            }
            switches.push(cases);
        }

        // Name string at position 0
        let mut pos: usize = 0;
        let name = read_nullstr(data, &mut pos);

        // Instructions
        let mut opcodes = Vec::with_capacity(instruction_count);
        let mut int_operands = Vec::with_capacity(instruction_count);
        let mut string_operands: Vec<Option<String>> = Vec::with_capacity(instruction_count);
        let mut long_operands: Vec<Option<i64>> = Vec::with_capacity(instruction_count);

        while pos < meta_start {
            let opcode = u16::from_be_bytes([data[pos], data[pos + 1]]);
            pos += 2;

            if opcode == 3 {
                // PUSH_CONSTANT_STRING
                let s = read_nullstr(data, &mut pos);
                opcodes.push(opcode);
                int_operands.push(0);
                string_operands.push(Some(s));
                long_operands.push(None);
            } else if opcode == 54 {
                // PUSH_CONSTANT_LONG
                let v = i64::from_be_bytes([
                    data[pos],
                    data[pos + 1],
                    data[pos + 2],
                    data[pos + 3],
                    data[pos + 4],
                    data[pos + 5],
                    data[pos + 6],
                    data[pos + 7],
                ]);
                pos += 8;
                opcodes.push(opcode);
                int_operands.push(0);
                string_operands.push(None);
                long_operands.push(Some(v));
            } else if opcode >= 150 || opcode == 21 || opcode == 38 || opcode == 39 {
                // Byte operand
                let v = data[pos] as i32;
                pos += 1;
                opcodes.push(opcode);
                int_operands.push(v);
                string_operands.push(None);
                long_operands.push(None);
            } else {
                // Int operand (default)
                let v =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                pos += 4;
                opcodes.push(opcode);
                int_operands.push(v);
                string_operands.push(None);
                long_operands.push(None);
            }
        }

        Ok(Script {
            name,
            local_int_count,
            local_string_count,
            local_long_count,
            int_param_count,
            string_param_count,
            long_param_count,
            opcodes,
            int_operands,
            string_operands,
            long_operands,
            switches,
        })
    }
}

fn read_nullstr(data: &[u8], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    let mut bytes = data[start..*pos].to_vec();
    bytes.push(0);
    let s = Packet::from(bytes).gjstr(0);
    if *pos < data.len() {
        *pos += 1; // skip null terminator
    }
    s
}
