use super::provider::{CacheType, TypeProvider};
pub use crate::types::InvScope;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type InvTypeProvider = TypeProvider<InvType>;

pub struct InvType {
    pub id: u16,
    pub scope: InvScope,
    pub size: u16,
    pub stackall: bool,
    pub stockobj: Option<Box<[u16]>>,
    pub stockcount: Option<Box<[u16]>>,
    pub stockrate: Option<Box<[i32]>>,
    pub restock: bool,
    pub allstock: bool,
    pub protect: bool,
    pub runweight: bool,
    pub dummyinv: bool,
    debugname: Option<Box<str>>,
}

pub struct InvTypeRaw {
    pub id: u16,
    pub scope: InvScope,
    #[cfg(since_647)]
    pub has_scope: bool,
    pub size: u16,
    pub stackall: bool,
    pub stockobj: Option<Box<[u16]>>,
    pub stockcount: Option<Box<[u16]>>,
    pub stockrate: Option<Box<[i32]>>,
    pub restock: bool,
    pub allstock: bool,
    pub protect: bool,
    pub runweight: bool,
    pub dummyinv: bool,
    debugname: Option<Box<str>>,
}

impl CacheType for InvTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        InvTypeRaw {
            id,
            scope: InvScope::Temp,
            #[cfg(since_647)]
            has_scope: false,
            size: if cfg!(since_647) { 0 } else { 1 },
            stackall: false,
            stockobj: None,
            stockcount: None,
            stockrate: None,
            restock: false,
            allstock: false,
            protect: true,
            runweight: false,
            dummyinv: false,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            match code {
                1 => {
                    self.scope = InvScope::try_from(buf.g1()).unwrap();
                    #[cfg(since_647)]
                    {
                        self.has_scope = true;
                    }
                }
                2 => self.size = buf.g2(),
                3 => self.stackall = true,
                4 => self.decode_stock(buf),
                5 => self.restock = true,
                6 => self.allstock = true,
                7 => self.protect = false,
                8 => self.runweight = true,
                9 => self.dummyinv = true,
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("inv", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl InvTypeRaw {
    fn decode_stock(&mut self, buf: &mut Packet) {
        let count = buf.g1() as usize;
        let mut objs = Vec::with_capacity(count);
        let mut counts = Vec::with_capacity(count);
        let mut rates = Vec::with_capacity(count);
        for _ in 0..count {
            objs.push(buf.g2());
            counts.push(buf.g2());
            rates.push(buf.g4s());
        }
        self.stockobj = Some(objs.into_boxed_slice());
        self.stockcount = Some(counts.into_boxed_slice());
        self.stockrate = Some(rates.into_boxed_slice());
    }
}

impl From<InvTypeRaw> for InvType {
    fn from(raw: InvTypeRaw) -> Self {
        Self {
            id: raw.id,
            scope: raw.scope,
            size: raw.size,
            stackall: raw.stackall,
            stockobj: raw.stockobj,
            stockcount: raw.stockcount,
            stockrate: raw.stockrate,
            restock: raw.restock,
            allstock: raw.allstock,
            protect: raw.protect,
            runweight: raw.runweight,
            dummyinv: raw.dummyinv,
            debugname: raw.debugname,
        }
    }
}

impl InvType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
