use super::ScriptVarType;
use super::dbrow::{DbRowType, DbRowValue};
use super::decode::read_array;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use std::collections::HashMap;

pub type DbTableTypeProvider = TypeProvider<DbTableType>;

pub type DbTableTypes = Option<Box<[Box<[u8]>]>>;
pub type DbTableDefaults = Option<Box<[Option<Box<[DbTableValue]>>]>>;
pub type DbTableColumns = Option<Box<[Box<str>]>>;

#[derive(Debug, Clone)]
pub enum DbTableValue {
    Int(i32),
    String(Box<str>),
}

pub struct DbTableType {
    pub id: u16,
    pub types: DbTableTypes,
    pub defaults: DbTableDefaults,
    debugname: Box<str>,
    pub columns: DbTableColumns,
    pub props: Option<Box<[u8]>>,
}

impl DbTableType {
    pub fn get_default(&self, column: usize) -> Option<Vec<DbTableValue>> {
        if let Some(defaults) = &self.defaults
            && let Some(Some(vals)) = defaults.get(column)
        {
            return Some(vals.to_vec());
        }
        let types = self.types.as_ref()?.get(column)?;
        Some(
            types
                .iter()
                .map(|&t| {
                    if t == ScriptVarType::String as u8 {
                        DbTableValue::String(Box::from(""))
                    } else if t == ScriptVarType::Boolean as u8 {
                        DbTableValue::Int(0)
                    } else {
                        DbTableValue::Int(-1)
                    }
                })
                .collect(),
        )
    }

    pub fn decode_values(buf: &mut Packet, types: &[u8]) -> Box<[DbTableValue]> {
        let len = buf.g1() as usize;
        let mut values = Vec::with_capacity(len * types.len());
        for _ in 0..len {
            for &t in types {
                values.push(if t == ScriptVarType::String as u8 {
                    DbTableValue::String(buf.gjstr(STRING_TERMINATOR).into_boxed_str())
                } else {
                    DbTableValue::Int(buf.g4s())
                });
            }
        }
        values.into_boxed_slice()
    }
}

impl CacheType for DbTableType {
    type Context = ();

    fn new(id: u16) -> Self {
        DbTableType {
            id,
            types: None,
            defaults: None,
            debugname: Box::default(),
            columns: None,
            props: None,
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
                    let len = buf.g1() as usize;
                    let mut types = vec![vec![].into_boxed_slice(); len].into_boxed_slice();
                    let mut defaults = vec![None; len].into_boxed_slice();
                    loop {
                        let info = buf.g1();
                        if info == 0xFF {
                            break;
                        }
                        let column = (info & 0x7F) as usize;
                        types[column] = read_array(buf, Packet::g1);
                        if (info & 0x80) != 0 {
                            defaults[column] = Some(DbTableType::decode_values(buf, &types[column]))
                        }
                    }
                    self.types = Some(types);
                    self.defaults = Some(defaults);
                }
                250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                251 => {
                    self.columns = Some(read_array(buf, |buf| {
                        buf.gjstr(STRING_TERMINATOR).into_boxed_str()
                    }));
                }
                252 => self.props = Some(read_array(buf, Packet::g1)),
                _ => crate::unknown_opcode!("dbtable", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

const INDEXED: u8 = 0x1;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DbIndexKey {
    Int(i32),
    String(Box<str>),
}

pub struct DbTableIndex {
    rows: HashMap<u32, HashMap<DbIndexKey, Vec<u16>>>,
}

impl DbTableIndex {
    pub fn build(tables: &TypeProvider<DbTableType>, rows: &TypeProvider<DbRowType>) -> Self {
        let mut index: HashMap<u32, HashMap<DbIndexKey, Vec<u16>>> = HashMap::new();

        for table in tables.types.iter() {
            let Some(props) = &table.props else { continue };

            let has_indexed = props.iter().any(|&p| (p & INDEXED) != 0);
            if !has_indexed {
                continue;
            }

            for row in rows.types.iter() {
                if row.table != table.id {
                    continue;
                }

                let Some(columns) = &row.columns else {
                    continue;
                };
                let Some(row_types) = &row.types else {
                    continue;
                };

                for (column, values) in columns.iter().enumerate() {
                    if column >= props.len() || (props[column] & INDEXED) == 0 {
                        continue;
                    }

                    let Some(values) = values else { continue };
                    let col_types = match row_types.get(column) {
                        Some(t) if !t.is_empty() => t,
                        _ => continue,
                    };

                    if col_types.len() > 1 {
                        let field_count = values.len() / col_types.len();
                        for field_id in 0..field_count {
                            for id in 0..col_types.len() {
                                let packed = ((table.id as u32 & 0xFFFF) << 12)
                                    | ((column as u32 & 0x7F) << 4)
                                    | (id as u32 & 0xF);
                                let idx = id + field_id * col_types.len();
                                if let Some(value) = values.get(idx) {
                                    let key = match value {
                                        DbRowValue::Int(v) => DbIndexKey::Int(*v),
                                        DbRowValue::String(v) => DbIndexKey::String(v.clone()),
                                    };
                                    index
                                        .entry(packed)
                                        .or_default()
                                        .entry(key)
                                        .or_default()
                                        .push(row.id);
                                }
                            }
                        }
                    } else {
                        let packed =
                            ((table.id as u32 & 0xFFFF) << 12) | ((column as u32 & 0x7F) << 4);
                        for value in values.iter() {
                            let key = match value {
                                DbRowValue::Int(v) => DbIndexKey::Int(*v),
                                DbRowValue::String(v) => DbIndexKey::String(v.clone()),
                            };
                            index
                                .entry(packed)
                                .or_default()
                                .entry(key)
                                .or_default()
                                .push(row.id);
                        }
                    }
                }
            }
        }

        DbTableIndex { rows: index }
    }

    pub fn find(&self, query: &DbIndexKey, table_column_packed: u32) -> &[u16] {
        let tuple = table_column_packed & 0xF;
        let lookup_key = if tuple == 0 {
            table_column_packed
        } else {
            table_column_packed - 1
        };
        self.rows
            .get(&lookup_key)
            .and_then(|lookup| lookup.get(query))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}
