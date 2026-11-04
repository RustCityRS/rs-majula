use std::collections::HashMap;

use rs_io::Packet;

pub trait CacheType: Sized {
    type Context;

    fn new(id: u16) -> Self;
    fn decode(&mut self, buf: &mut Packet);
    fn post_decode(_types: &mut Vec<Self>, _ctx: &Self::Context) {}
    fn debugname(&self) -> Option<&str> {
        None
    }
}

pub struct TypeProvider<T> {
    pub names: HashMap<Box<str>, u16>,
    pub types: Box<[T]>,
}

impl<T: CacheType> TypeProvider<T> {
    pub fn from_bytes(dat: &[u8], ctx: T::Context) -> Self {
        let mut buf = Packet::from(dat.to_vec());
        let count = buf.g2() as usize;

        let mut types = Vec::with_capacity(count);
        let mut names = HashMap::new();

        for i in 0..count {
            let id = i as u16;
            let mut entry = T::new(id);
            entry.decode(&mut buf);
            if let Some(name) = entry.debugname() {
                names.insert(name.into(), id);
            }
            types.push(entry);
        }

        T::post_decode(&mut types, &ctx);

        Self {
            names,
            types: types.into_boxed_slice(),
        }
    }

    pub fn get_by_id(&self, id: u16) -> Option<&T> {
        self.types.get(id as usize)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&T> {
        let &id = self.names.get(name)?;
        self.get_by_id(id)
    }

    pub fn get_by_id_mut(&mut self, id: u16) -> Option<&mut T> {
        self.types.get_mut(id as usize)
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}

impl<T> Default for TypeProvider<T> {
    fn default() -> Self {
        Self {
            names: HashMap::new(),
            types: Vec::new().into_boxed_slice(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum ParamValue {
    Int(i32),
    String(Box<str>),
}
