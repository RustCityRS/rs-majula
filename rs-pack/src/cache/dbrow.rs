use super::ScriptVarType;
use super::decode::read_array;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

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
    debugname: Box<str>,
}

impl DbRowType {
    pub fn decode_values(buf: &mut Packet, types: &[u8]) -> Box<[DbRowValue]> {
        let len = buf.g1() as usize;
        let mut values = Vec::with_capacity(len * types.len());
        for _ in 0..len {
            for &t in types {
                values.push(if t == ScriptVarType::String as u8 {
                    DbRowValue::String(buf.gjstr(STRING_TERMINATOR).into_boxed_str())
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
            debugname: Box::default(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            match code {
                3 => {
                    let len = buf.g1() as usize;
                    let mut types = vec![vec![].into_boxed_slice(); len].into_boxed_slice();
                    let mut columns = vec![None; len].into_boxed_slice();
                    loop {
                        let column = buf.g1() as usize;
                        if column == 0xFF {
                            break;
                        }
                        types[column] = read_array(buf, Packet::g1);
                        columns[column] = Some(DbRowType::decode_values(buf, &types[column]));
                    }
                    self.types = Some(types);
                    self.columns = Some(columns);
                }
                4 => self.table = buf.g2(),
                250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                _ => crate::unknown_opcode!("dbrow", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::dbtable::{DbIndexKey, DbTableIndex, DbTableType, DbTableValue};

    #[test]
    fn sparse_columns_keep_defaults_values_and_names() {
        let mut table = Packet::new(128);
        table.p1(1);
        table.p1(3);
        table.p1(0x82);
        table.p1(2);
        table.p1(b'i');
        table.p1(b's');
        table.p1(1);
        table.p4(42);
        table.pjstr("default", STRING_TERMINATOR);
        table.p1(0xFF);
        table.p1(252);
        table.p1(3);
        table.p1(0);
        table.p1(0);
        table.p1(1);
        table.p1(250);
        table.pjstr("table", STRING_TERMINATOR);
        table.p1(0);
        let mut definition = DbTableType::new(0);
        definition.decode(&mut Packet::from(table.data[..table.pos].to_vec()));
        let tables = TypeProvider::<DbTableType>::from_types(vec![definition], ());
        let defaults = tables
            .get_by_debugname("table")
            .unwrap()
            .get_default(2)
            .unwrap();
        assert!(matches!(&defaults[0], DbTableValue::Int(42)));
        assert!(matches!(&defaults[1], DbTableValue::String(s) if s.as_ref() == "default"));

        let mut row = Packet::new(128);
        row.p1(3);
        row.p1(3);
        row.p1(2);
        row.p1(2);
        row.p1(b'i');
        row.p1(b's');
        row.p1(1);
        row.p4(7);
        row.pjstr("value", STRING_TERMINATOR);
        row.p1(0xFF);
        row.p1(250);
        row.pjstr("row", STRING_TERMINATOR);
        row.p1(0);
        let mut definition = DbRowType::new(0);
        definition.decode(&mut Packet::from(row.data[..row.pos].to_vec()));
        let rows = TypeProvider::<DbRowType>::from_types(vec![definition], ());
        let row = rows.get_by_debugname("row").unwrap();
        let columns = row.columns.as_ref().unwrap();
        assert!(columns[0].is_none());
        assert!(columns[1].is_none());
        let values = columns[2].as_ref().unwrap();
        assert!(matches!(&values[0], DbRowValue::Int(7)));
        assert!(matches!(&values[1], DbRowValue::String(s) if s.as_ref() == "value"));
        let index = DbTableIndex::build(&tables, &rows);
        assert_eq!(index.find(&DbIndexKey::Int(7), 2 << 4), &[0]);
    }
}
