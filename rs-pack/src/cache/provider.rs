use rs_io::Packet;
use rustc_hash::FxHashMap;

pub trait CacheType: Sized {
    type Context;

    fn new(id: u16) -> Self;
    fn decode(&mut self, buf: &mut Packet);
    fn post_decode(_types: &mut Vec<Self>, _ctx: &Self::Context) {}
    fn debugname(&self) -> Option<&str>;
}

pub struct TypeProvider<T> {
    pub debugnames: FxHashMap<Box<str>, u16>,
    pub types: Box<[T]>,
}

impl<T> TypeProvider<T> {
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

        Raw::post_decode(&mut raws, &ctx);

        let mut debugnames = FxHashMap::with_capacity_and_hasher(count, Default::default());
        let mut types = Vec::with_capacity(count);

        for (index, raw) in raws.into_iter().enumerate() {
            if let Some(debugname) = raw.debugname() {
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

    pub fn get_by_debugname(&self, name: &str) -> Option<&T> {
        self.debugnames.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}
