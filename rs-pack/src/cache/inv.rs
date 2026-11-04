use super::provider::{CacheType, TypeProvider};
pub use crate::types::InvScope;
use rs_io::Packet;

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
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for InvType {
    type Context = ();

    fn new(id: u16) -> Self {
        InvType {
            id,
            scope: InvScope::Temp,
            size: 1,
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
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.scope = InvScope::try_from(buf.g1()).unwrap(),
                2 => self.size = buf.g2(),
                3 => self.stackall = true,
                4 => {
                    let len = buf.g1() as usize;
                    let mut stockobj = vec![0; len];
                    let mut stockcount = vec![0; len];
                    let mut stockrate = vec![0; len];
                    for index in 0..len {
                        stockobj[index] = buf.g2();
                        stockcount[index] = buf.g2();
                        stockrate[index] = buf.g4s();
                    }
                    self.stockobj = Some(stockobj.into_boxed_slice());
                    self.stockcount = Some(stockcount.into_boxed_slice());
                    self.stockrate = Some(stockrate.into_boxed_slice());
                }
                5 => self.restock = true,
                6 => self.allstock = true,
                7 => self.protect = false,
                8 => self.runweight = true,
                9 => self.dummyinv = true,
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("inv", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
