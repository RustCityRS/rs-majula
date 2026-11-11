use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::pack::packed_data::PackedData;
use crate::types::VarPlayerScope;
use anyhow::Result;
use rs_io::Packet;

pub struct VarClientTypeProvider {
    ints: TypeProvider<VarClientType>,
    strings: TypeProvider<VarClientType>,
}

impl VarClientTypeProvider {
    pub fn from_indexed_bytes(ints: &PackedData, strings: &PackedData) -> Result<Self> {
        Ok(Self {
            ints: TypeProvider::from_indexed_bytes::<VarClientType>(&ints.dat, &ints.idx, false)?,
            strings: TypeProvider::from_indexed_bytes::<VarClientType>(
                &strings.dat,
                &strings.idx,
                true,
            )?,
        })
    }

    pub fn types(&self) -> impl Iterator<Item = &VarClientType> {
        self.ints.types.iter().chain(self.strings.types.iter())
    }

    pub fn get_by_id(&self, id: u16, string: bool) -> Option<&VarClientType> {
        if string {
            self.strings.get_by_id(id)
        } else {
            self.ints.get_by_id(id)
        }
    }

    pub fn index(&self, id: u16, string: bool) -> usize {
        let (offset, types) = if string {
            (self.ints.count(), &self.strings)
        } else {
            (0, &self.ints)
        };
        assert!((id as usize) < types.count());
        offset + id as usize
    }
}

pub struct VarClientType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub scope: VarPlayerScope,
}

impl CacheType for VarClientType {
    type Context = bool;

    fn new(id: u16) -> Self {
        Self {
            id,
            var_type: ScriptVarType::Int,
            scope: VarPlayerScope::Temp,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            match code {
                1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.scope = VarPlayerScope::Perm,
                _ => crate::unknown_opcode!("varc", code),
            }
        }
    }

    fn post_decode(types: &mut Vec<Self>, strings: &bool) {
        if *strings {
            for varc in types {
                varc.var_type = ScriptVarType::String;
            }
        }
    }
}
