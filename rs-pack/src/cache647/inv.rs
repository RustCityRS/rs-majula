//! InvType — CacheType implementation for inventory definitions.
//!
//! Decoded from archive 2, group 5 (CONFIG_INV).
//! Cross-referenced against rev-647 client `InvType.decode(Packet, int)`.

use rs_io::Packet;

use super::CacheType;
// Re-export InvScope so downstream can `use rs_pack::cache::inv::InvScope`.
pub use crate::types647::InvScope;

/// Inventory definition.
///
/// | op  | shape                                                       |
/// |-----|-------------------------------------------------------------|
/// | 1   | g1 (scope: 0=temp, 1=perm, 2=shared)                        |
/// | 2   | g2 (capacity)                                               |
/// | 3   | flag (stackall = true)                                      |
/// | 4   | u8 count, count x (g2 obj, g2 count, g4s rate) — stock list |
/// | 5   | flag (restock = true)                                       |
/// | 6   | flag (allstock = true)                                      |
/// | 7   | flag (protect = false)                                      |
/// | 8   | flag (runweight = true)                                     |
/// | 9   | flag (dummyinv = true)                                      |
/// | 250 | gjstr (debugname)                                           |
pub struct InvType {
    pub id: u16,
    pub scope: InvScope,
    pub has_scope: bool,
    pub size: u16,
    pub stackall: bool,
    pub stock_obj: Vec<u16>,
    pub stock_count: Vec<u16>,
    pub stock_rate: Vec<i32>,
    pub restock: bool,
    pub allstock: bool,
    pub protect: bool,
    pub runweight: bool,
    pub dummyinv: bool,
    debugname: Option<Box<str>>,
}

impl CacheType for InvType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            scope: InvScope::Temp,
            has_scope: false,
            size: 0,
            stackall: false,
            stock_obj: Vec::new(),
            stock_count: Vec::new(),
            stock_rate: Vec::new(),
            restock: false,
            allstock: false,
            protect: true,
            runweight: false,
            dummyinv: false,
            debugname: None,
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
                    self.scope = InvScope::try_from(buf.g1()).unwrap_or(InvScope::Temp);
                    self.has_scope = true;
                }
                2 => self.size = buf.g2(),
                3 => self.stackall = true,
                // Server opcode 4 = restock (flag). Client cache opcode 4 is
                // batched stock (g1 count + entries), but client inv data only
                // emits opcode 2 (size), so 4 in a merged .dat is always the
                // server version.
                4 => self.restock = true,
                5 => self.allstock = true,
                // Server opcode 6 = individual stock entry (g2 obj + g2 count + g4 rate).
                6 => {
                    self.stock_obj.push(buf.g2());
                    self.stock_count.push(buf.g2());
                    self.stock_rate.push(buf.g4s());
                }
                7 => self.protect = false,
                // Server opcode 8 = protect=false (matches 2004scape numbering)
                8 => self.protect = false,
                9 => self.runweight = true,
                10 => self.dummyinv = true,
                250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
