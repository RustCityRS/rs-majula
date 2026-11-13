use rs_io::Packet;
use runec::opcodes::{PUSH_CONSTANT_STRING, is_large_operand};
use rustc_hash::FxHashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub struct ScriptProvider {
    pub names: FxHashMap<Box<str>, i32>,
    pub scripts: Box<[Option<Arc<Script>>]>,
    pub lookups: FxHashMap<i32, i32>,
}

impl ScriptProvider {
    pub fn from_bytes(dat_bytes: &[u8], idx_bytes: &[u8]) -> ScriptProvider {
        let mut dat = Packet::from(dat_bytes.to_vec());
        let mut idx = Packet::from(idx_bytes.to_vec());

        let count = dat.g4s() as usize;
        idx.pos += 4;

        dat.g4s(); // compiled version

        let mut names: FxHashMap<Box<str>, i32> =
            FxHashMap::with_capacity_and_hasher(count, Default::default());
        let mut scripts = vec![None; count];
        let mut lookups: FxHashMap<i32, i32> =
            FxHashMap::with_capacity_and_hasher(count, Default::default());

        for (index, slot) in scripts.iter_mut().enumerate() {
            let length = idx.g4s() as usize;
            if length == 0 {
                continue;
            }

            let id = index as i32;

            let start = dat.pos;
            let end = start + length;

            let script = Script::new(&mut dat, id, length);

            let info = &script.info;
            names.insert(info.name.clone(), id);
            if info.lookup != -1 {
                lookups.insert(info.lookup, id);
            }

            if dat.pos > end {
                panic!("Script {index} has read past end!");
            }

            dat.pos = end;

            *slot = Some(Arc::new(script));
        }

        ScriptProvider {
            names,
            scripts: Box::from(scripts),
            lookups,
        }
    }

    pub fn io(&mut self, dat_path: &Path, idx_path: &Path) -> anyhow::Result<()> {
        let dat = fs::read(dat_path)?;
        let idx = fs::read(idx_path)?;
        *self = Self::from_bytes(&dat, &idx);
        Ok(())
    }

    #[inline]
    pub fn get_by_id(&self, id: i32) -> Option<&Arc<Script>> {
        self.scripts.get(id as usize).and_then(|s| s.as_ref())
    }

    #[inline]
    pub fn get_by_lookup(&self, key: i32) -> Option<&Arc<Script>> {
        self.lookups.get(&key).and_then(|&id| self.get_by_id(id))
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Arc<Script>> {
        self.names.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.scripts.iter().filter(|s| s.is_some()).count()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub name: Box<str>,
    pub path: Box<str>,
    pub lookup: i32,
    pub params: Box<[u8]>,
    pub pcs: Box<[i32]>,
    pub lines: Box<[i32]>,
}

impl ScriptInfo {
    pub fn line_number(&self, pc: i32) -> i32 {
        for (i, &pcs) in self.pcs.iter().enumerate() {
            if pcs > pc {
                return self.lines.get(i.wrapping_sub(1)).copied().unwrap_or(0);
            }
        }
        self.lines.last().copied().unwrap_or(0)
    }

    pub fn file_name(&self) -> &str {
        Path::new(&*self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.path)
    }
}

#[derive(Debug)]
pub struct Script {
    pub id: i32,
    pub int_arg_count: u16,
    pub string_arg_count: u16,
    pub int_local_count: u16,
    pub string_local_count: u16,
    pub opcodes: Box<[u16]>,
    pub int_operands: Box<[i32]>,
    pub strings: Box<[Box<str>]>,
    pub switch_tables: Box<[FxHashMap<i32, i32>]>,
    pub info: ScriptInfo,
}

impl Script {
    pub fn new(dat: &mut Packet, id: i32, length: usize) -> Self {
        let start = dat.pos;
        let end = start + length;

        if end < 16 {
            panic!("Invalid script file (minimum length) must be 16 bytes.");
        }

        dat.pos = end - 2;

        let trailer_len = dat.g2() as usize;
        let trailer_pos = end - trailer_len - 12 - 2;

        if trailer_pos >= end {
            panic!("Invalid script file (bad trailer pos).");
        }

        dat.pos = trailer_pos;

        let instructions = dat.g4s() as usize;
        let int_local_count = dat.g2();
        let string_local_count = dat.g2();
        let int_arg_count = dat.g2();
        let string_arg_count = dat.g2();
        let switch_count = dat.g1() as usize;

        let mut switch_tables: Vec<FxHashMap<i32, i32>> = Vec::with_capacity(switch_count);
        for _ in 0..switch_count {
            let count = dat.g2() as usize;
            let mut table: FxHashMap<i32, i32> =
                FxHashMap::with_capacity_and_hasher(count, Default::default());

            for _ in 0..count {
                table.insert(dat.g4s(), dat.g4s());
            }

            switch_tables.push(table);
        }

        dat.pos = start;
        let name = dat.gjstr(0);
        let path = dat.gjstr(0);
        let lookup = dat.g4s();

        let params_count = dat.g1() as usize;
        let mut params = vec![0; params_count];
        for param in params.iter_mut() {
            *param = dat.g1();
        }

        let lines_count = dat.g2() as usize;
        let mut pcs = vec![0; lines_count];
        let mut lines = vec![0; lines_count];
        for index in 0..lines_count {
            pcs[index] = dat.g4s();
            lines[index] = dat.g4s();
        }

        let info = ScriptInfo {
            name: name.into_boxed_str(),
            path: path.into_boxed_str(),
            lookup,
            params: params.into_boxed_slice(),
            pcs: pcs.into_boxed_slice(),
            lines: lines.into_boxed_slice(),
        };

        let mut opcodes: Vec<u16> = vec![0; instructions];
        let mut int_operands: Vec<i32> = vec![0; instructions];
        let mut strings: Vec<Box<str>> = Vec::new();

        let mut pc: usize = 0;
        while trailer_pos > dat.pos {
            let code = dat.g2();

            if code == PUSH_CONSTANT_STRING {
                int_operands[pc] = strings.len() as i32;
                strings.push(dat.gjstr(0).into_boxed_str());
            } else if is_large_operand(code) {
                int_operands[pc] = dat.g4s();
            } else {
                int_operands[pc] = dat.g1() as i32;
            }

            opcodes[pc] = code;
            pc += 1;
        }

        Script {
            id,
            int_arg_count,
            string_arg_count,
            int_local_count,
            string_local_count,
            opcodes: opcodes.into_boxed_slice(),
            int_operands: int_operands.into_boxed_slice(),
            strings: strings.into_boxed_slice(),
            switch_tables: switch_tables.into_boxed_slice(),
            info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use runec::opcodes::PUSH_CONSTANT_INT;

    #[test]
    fn string_pushes_index_the_string_table() {
        let mut bytes = b"test\0test\0".to_vec();
        bytes.extend((-1i32).to_be_bytes()); // lookup
        bytes.push(0); // params
        bytes.extend(0u16.to_be_bytes()); // lines
        bytes.extend(PUSH_CONSTANT_STRING.to_be_bytes());
        bytes.extend(b"hi\0");
        bytes.extend(PUSH_CONSTANT_INT.to_be_bytes());
        bytes.extend(7i32.to_be_bytes());
        bytes.extend(PUSH_CONSTANT_STRING.to_be_bytes());
        bytes.extend(b"yo\0");
        bytes.extend(3i32.to_be_bytes()); // instructions
        bytes.extend([0; 8]); // locals and args
        bytes.push(0); // switches
        bytes.extend(1u16.to_be_bytes()); // trailer length
        let length = bytes.len();

        let script = Script::new(&mut Packet::from(bytes), 0, length);

        assert_eq!(*script.int_operands, [0, 7, 1]);
        assert_eq!(
            script.strings.iter().map(|s| &**s).collect::<Vec<_>>(),
            ["hi", "yo"]
        );
    }
}
