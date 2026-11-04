//! DbRow cache type — decodes the server `dbrow.dat` binary written by
//! `crate::pack::dbrow::pack_dbrows`. Server-only (matches 2004scape
//! `DbRowType.ts`).
//!
//! Binary shape (exact inverse of the packer):
//!
//! | op  | shape                                                                  |
//! |-----|------------------------------------------------------------------------|
//! | 4   | g2 table_id                                                            |
//! | 3   | g1 count, per column: g1 col_idx, g1 type_count, type_count × g1 type, |
//! |     | g1 field_count, field_count × type_count values                        |
//! |     | (pjstr if type 's' else p4); terminated by 255                         |
//! | 250 | pjstr debugname                                                        |

use rs_io::Packet;

use super::CacheType;
use super::dbtable::DbValue;

/// One column's stored values in a dbrow.
#[derive(Debug, Clone)]
pub struct DbRowColumn {
    /// Index into the owning table's column list.
    pub col_idx: usize,
    /// Per-tuple type chars (the stride for `getfieldcount`/`getfield`).
    pub types: Vec<u8>,
    /// Flat list of stored values, length = `field_count * types.len()`.
    pub values: Vec<DbValue>,
}

impl DbRowColumn {
    /// Number of tuples stored = `values.len() / types.len()`.
    pub fn tuple_count(&self) -> usize {
        if self.types.is_empty() {
            0
        } else {
            self.values.len() / self.types.len()
        }
    }
}

/// Server `dbrow.dat` entry.
#[derive(Debug, Default)]
pub struct DbRowType {
    pub id: u16,
    /// Owning table id (from opcode 4); -1 if absent.
    pub table_id: i32,
    /// Columns in stored order; look up by `col_idx`.
    pub columns: Vec<DbRowColumn>,
    pub debugname: Option<String>,
}

impl CacheType for DbRowType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            table_id: -1,
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
                3 => {
                    // Leading count, then col entries until the 255 terminator.
                    let _count = buf.g1();
                    loop {
                        let col_idx = buf.g1();
                        if col_idx == 255 {
                            break;
                        }
                        let type_count = buf.g1() as usize;
                        let mut types = Vec::with_capacity(type_count);
                        for _ in 0..type_count {
                            types.push(buf.g1());
                        }
                        let field_count = buf.g1() as usize;
                        let mut values = Vec::with_capacity(field_count * type_count);
                        for _ in 0..field_count {
                            for &tc in &types {
                                if tc == b's' {
                                    values.push(DbValue::String(buf.gjstr(0).into_boxed_str()));
                                } else {
                                    values.push(DbValue::Int(buf.g4s()));
                                }
                            }
                        }
                        self.columns.push(DbRowColumn {
                            col_idx: col_idx as usize,
                            types,
                            values,
                        });
                    }
                }
                4 => {
                    self.table_id = buf.g2() as i32;
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

impl DbRowType {
    /// Find the stored column matching a table column index.
    pub fn column(&self, col_idx: usize) -> Option<&DbRowColumn> {
        self.columns.iter().find(|c| c.col_idx == col_idx)
    }
}
