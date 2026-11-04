use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

pub type DbRowTypeProvider = TypeProvider<DbRowType>;

pub type DbRowTypes = Option<Box<[Box<[u8]>]>>;
pub type DbRowColumns = Option<Box<[Option<Box<[DbRowValue]>>]>>;

#[derive(Debug, Clone)]
pub enum DbRowValue {
    Int(i32),
    String(Box<str>),
}

pub struct DbRowType {
    pub id: u16,
    pub types: DbRowTypes,
    pub columns: DbRowColumns,
    pub table: u16,
    pub(crate) debugname: Option<Box<str>>,
}

impl DbRowType {
    pub fn decode_values(buf: &mut Packet, types: &[u8]) -> Box<[DbRowValue]> {
        let len = buf.g1() as usize;
        let mut values = Vec::with_capacity(len * types.len());
        for _ in 0..len {
            for &t in types {
                values.push(if t == ScriptVarType::String as u8 {
                    DbRowValue::String(buf.gjstr(10).into_boxed_str())
                } else {
                    DbRowValue::Int(buf.g4s())
                });
            }
        }
        values.into_boxed_slice()
    }
}

impl CacheType for DbRowType {
    type Context = ();

    fn new(id: u16) -> Self {
        DbRowType {
            id,
            types: None,
            columns: None,
            table: 0,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                3 => {
                    let len = buf.g1() as usize;
                    let mut types = vec![vec![].into_boxed_slice(); len].into_boxed_slice();
                    let mut columns = vec![None; len].into_boxed_slice();
                    loop {
                        let column = buf.g1() as usize;
                        if column == 0xFF {
                            break;
                        }
                        let type_count = buf.g1() as usize;
                        types[column] = (0..type_count)
                            .map(|_| buf.g1())
                            .collect::<Vec<_>>()
                            .into_boxed_slice();
                        columns[column] = Some(DbRowType::decode_values(buf, &types[column]));
                    }
                    self.types = Some(types);
                    self.columns = Some(columns);
                }
                4 => self.table = buf.g2(),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("dbrow", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
