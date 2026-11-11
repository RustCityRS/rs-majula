use crate::pack::packed_data::PackedData;
use anyhow::Result;
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub trait CacheType: Sized {
    type Context;

    fn new(id: u16) -> Self;
    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            self.decode_opcode(buf, code);
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8);
    fn post_decode(_types: &mut Vec<Self>, _ctx: &Self::Context) {}

    /// The name configs and scripts know the record by. Client records carry
    /// none.
    fn debugname(&self) -> &str {
        ""
    }
}

pub struct TypeProvider<T> {
    pub debugnames: FxHashMap<Box<str>, u16>,
    pub types: Box<[T]>,
}

impl<T> TypeProvider<T> {
    pub fn from_indexed_bytes<Raw>(dat: &[u8], idx: &[u8], ctx: Raw::Context) -> Result<Self>
    where
        Raw: CacheType,
        T: From<Raw>,
    {
        let raws = PackedData::entries(dat, idx)?
            .map(|(id, data)| {
                let mut raw = Raw::new(id);
                raw.decode(&mut Packet::from(data.to_vec()));
                raw
            })
            .collect();
        Ok(Self::from_types(raws, ctx))
    }

    pub fn from_bytes<Raw>(dat: &[u8], ctx: Raw::Context) -> TypeProvider<T>
    where
        Raw: CacheType,
        T: From<Raw>,
    {
        let mut dat = Packet::from(dat.to_vec());

        let count = dat.g2() as usize;

        let mut raws: Vec<Raw> = Vec::with_capacity(count);
        for index in 0..count {
            let mut entry = Raw::new(index as u16);
            entry.decode(&mut dat);
            raws.push(entry);
        }

        Self::from_types::<Raw>(raws, ctx)
    }

    pub fn from_types<Raw>(mut raws: Vec<Raw>, ctx: Raw::Context) -> TypeProvider<T>
    where
        Raw: CacheType,
        T: From<Raw>,
    {
        Raw::post_decode(&mut raws, &ctx);

        let count = raws.len();
        let mut debugnames = FxHashMap::default();
        let mut types = Vec::with_capacity(count);

        for (index, raw) in raws.into_iter().enumerate() {
            let debugname = raw.debugname();
            if !debugname.is_empty() {
                debugnames.insert(Box::from(debugname), index as u16);
            }

            types.push(T::from(raw));
        }

        TypeProvider {
            debugnames,
            types: types.into_boxed_slice(),
        }
    }

    pub fn get_by_id(&self, id: u16) -> Option<&T> {
        self.types.get(id as usize)
    }

    #[cfg(since_647)]
    pub fn map<U>(self, f: impl FnMut(T) -> U) -> TypeProvider<U> {
        TypeProvider {
            debugnames: self.debugnames,
            types: self.types.into_vec().into_iter().map(f).collect(),
        }
    }

    pub fn get_by_debugname(&self, name: &str) -> Option<&T> {
        self.debugnames.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}

impl<T> Default for TypeProvider<T> {
    fn default() -> Self {
        Self {
            debugnames: FxHashMap::default(),
            types: Box::new([]),
        }
    }
}
