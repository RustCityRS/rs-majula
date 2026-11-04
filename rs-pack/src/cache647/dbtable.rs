//! DbTable cache type — decodes the server `dbtable.dat` binary written by
//! `crate::pack::dbtable::pack_dbtables`. Server-only (matches 2004scape
//! `DbTableType.ts`).
//!
//! Binary shape (exact inverse of the packer):
//!
//! | op  | shape                                                                       |
//! |-----|-----------------------------------------------------------------------------|
//! | 1   | g1 count, per column: g1 flags (col_idx=flags&0x7F, has_default=flags&0x80), |
//! |     | g1 type_count, type_count × g1 type_char,                                   |
//! |     | if has_default: g1 field_count + field_count × type_count values            |
//! |     | (pjstr if type 's' else p4); terminated by 255                              |
//! | 251 | g1 count + count × pjstr column names                                       |
//! | 252 | g1 count + count × g1 column prop flag bytes                                 |
//! | 250 | pjstr debugname                                                             |

use rs_io::Packet;

use super::CacheType;

/// Column property flags (mirrors the packer's `PROP_*` constants).
pub const PROP_INDEXED: u8 = 0x1;
pub const PROP_REQUIRED: u8 = 0x2;
pub const PROP_LIST: u8 = 0x4;
pub const PROP_CLIENTSIDE: u8 = 0x8;

/// A single column definition of a dbtable.
#[derive(Debug, Default, Clone)]
pub struct DbColumn {
    /// The tuple type chars for this column (e.g. `[b'c', b'c']` for a
    /// `(coord, coord)` tuple). `len()` is the per-tuple field count.
    pub types: Vec<u8>,
    /// Property flags (`PROP_INDEXED | PROP_REQUIRED | …`).
    pub props: u8,
    /// Column name (from opcode 251), if present.
    pub name: Option<String>,
    /// Default values, if present (flat list of `field_count × type_count`).
    pub defaults: Vec<DbValue>,
}

/// A decoded dbtable/dbrow field value — either an int or a string.
#[derive(Debug, Clone)]
pub enum DbValue {
    Int(i32),
    String(Box<str>),
}

/// Server `dbtable.dat` entry.
#[derive(Debug, Default)]
pub struct DbTableType {
    pub id: u16,
    /// Columns indexed by column index.
    pub columns: Vec<DbColumn>,
    pub debugname: Option<String>,
}

impl CacheType for DbTableType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            columns: Vec::new(),
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
                    let count = buf.g1() as usize;
                    for _ in 0..count {
                        let flags = buf.g1();
                        let col_idx = (flags & 0x7F) as usize;
                        let has_default = (flags & 0x80) != 0;
                        let type_count = buf.g1() as usize;
                        let mut types = Vec::with_capacity(type_count);
                        for _ in 0..type_count {
                            types.push(buf.g1());
                        }
                        let mut defaults = Vec::new();
                        if has_default {
                            let field_count = buf.g1() as usize;
                            for _ in 0..field_count {
                                for &tc in &types {
                                    defaults.push(read_value(buf, tc));
                                }
                            }
                        }
                        ensure_len(&mut self.columns, col_idx);
                        self.columns[col_idx] = DbColumn {
                            types,
                            props: 0,
                            name: None,
                            defaults,
                        };
                    }
                    // end-of-columns marker
                    let _ = buf.g1(); // 255
                }
                251 => {
                    let count = buf.g1() as usize;
                    ensure_len(&mut self.columns, count.saturating_sub(1));
                    for i in 0..count {
                        let name = buf.gjstr(0);
                        self.columns[i].name = Some(name);
                    }
                }
                252 => {
                    let count = buf.g1() as usize;
                    ensure_len(&mut self.columns, count.saturating_sub(1));
                    for i in 0..count {
                        self.columns[i].props = buf.g1();
                    }
                }
                250 => {
                    self.debugname = Some(buf.gjstr(0));
                }
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

impl DbTableType {
    /// Number of tuple-field types in a column (the per-tuple stride).
    pub fn column_type_count(&self, col_idx: usize) -> usize {
        self.columns
            .get(col_idx)
            .map(|c| c.types.len())
            .unwrap_or(0)
    }
}

/// Read one field value of the given type char.
fn read_value(buf: &mut Packet, type_char: u8) -> DbValue {
    if type_char == b's' {
        DbValue::String(buf.gjstr(0).into_boxed_str())
    } else {
        DbValue::Int(buf.g4s())
    }
}

/// Grow `cols` so index `idx` is valid, filling with defaults.
fn ensure_len(cols: &mut Vec<DbColumn>, idx: usize) {
    while cols.len() <= idx {
        cols.push(DbColumn::default());
    }
}
