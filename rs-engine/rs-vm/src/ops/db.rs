use crate::engine::{ScriptEngine, engine};
use crate::register::OpsRegistry;
use crate::state::ScriptState;
use crate::util::pop_dbrow;
use crate::{ScriptError, handlers, none};
use rs_pack::cache::dbrow::DbRowValue;
use rs_pack::cache::dbtable::{DbIndexKey, DbTableValue};
use rs_pack::cache::script::*;
use rustc_hash::FxHashSet;

/// Registers database query opcodes for searching, iterating, and reading
/// fields from cache-defined database tables and rows.
///
/// # Opcodes Registered
///
/// - `DB_FIND` / `DB_FIND_WITH_COUNT` -- query a table index by int or string
///   key, storing the matching row ids for iteration; the `_WITH_COUNT` form
///   also pushes the number of matches.
/// - `DB_FIND_REFINE` / `DB_FIND_REFINE_WITH_COUNT` -- intersect a fresh query
///   with the rows from the previous query.
/// - `DB_LISTALL` / `DB_LISTALL_WITH_COUNT` -- select every row in a table.
/// - `DB_FINDNEXT` -- advance the row cursor and push the next matching row id
///   (or -1 when exhausted).
/// - `DB_FINDBYINDEX` -- push the row id at a given position in the result set.
/// - `DB_GETROWTABLE` -- push the id of the table that owns a given row.
/// - `DB_GETFIELD` -- read one or more typed values from a specific column
///   and tuple index of a database row.
/// - `DB_GETFIELDCOUNT` -- push the number of multi-value entries in a given
///   column of a database row.
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::db::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 7500
        none!(m, DB_FIND_WITH_COUNT => |s| {
            db_find(engine::<E>(), s, true)?;
        });

        // 7501
        none!(m, DB_FINDNEXT => |s| {
            if s.db_table.is_none() {
                return Err(ScriptError::Runtime("No table selected!".to_string()));
            }
            let next = s.db_row.map_or(0, |r| r + 1);
            match s.db_row_query.get(next as usize) {
                Some(&id) => {
                    s.db_row = Some(next);
                    s.push_int(id as i32);
                }
                None => s.push_int(-1),
            }
        });

        // 7502
        none!(m, DB_GETFIELD => |s| {
            let index = s.pop_int();
            let packed = s.pop_int();
            let dbrow = pop_dbrow::<E>(s)?;
            let table = ((packed >> 12) & 0xFFFF) as u16;
            let column = ((packed >> 4) & 0x7F) as usize;
            let tuple = (packed & 0xF) - 1;
            let dbtable = engine::<E>()
                .dbtables()
                .get_by_id(table)
                .ok_or(ScriptError::DbTableNotFound(table as i32))?;
            let Some(types) = &dbtable.types else {
                return Err(ScriptError::Runtime(format!("Dbtable {table} does not have any types!")));
            };
            let value_types = types.get(column)
                .ok_or(ScriptError::Runtime(format!("Dbtable type {column} out of bounds!")))?;
            let (off, len) = if tuple >= 0 {
                if tuple as usize >= value_types.len() {
                    return Err(ScriptError::Runtime(format!("Tuple index {tuple} out of bounds for {}!", value_types.len())));
                }
                (tuple as usize, tuple as usize + 1)
            } else {
                (0, value_types.len())
            };

            let row_values: Option<&[DbRowValue]> = if dbrow.table != table {
                None
            } else {
                let type_len = value_types.len();
                let start = index as usize * type_len;
                dbrow.columns
                    .as_ref()
                    .and_then(|cols| cols.get(column))
                    .and_then(|opt| opt.as_ref())
                    .and_then(|vals| vals.get(start..start + type_len))
            };

            if let Some(values) = row_values {
                for val in &values[off..len] {
                    match val {
                        DbRowValue::Int(v) => s.push_int(*v),
                        DbRowValue::String(v) => s.push_string(v),
                    }
                }
            } else {
                let defaults = dbtable.get_default(column)
                    .ok_or(ScriptError::Runtime(format!("Dbtable {table} column {column} has no defaults!")))?;
                for val in &defaults[off..len] {
                    match val {
                        DbTableValue::Int(v) => s.push_int(*v),
                        DbTableValue::String(v) => s.push_string(v),
                    }
                }
            }
        });

        // 7503
        none!(m, DB_GETFIELDCOUNT => |s| {
            let packed = s.pop_int();
            let dbrow = pop_dbrow::<E>(s)?;
            let table = ((packed >> 12) & 0xFFFF) as u16;
            let column = ((packed >> 4) & 0x7F) as u8;
            let dbtable = engine::<E>()
                .dbtables()
                .get_by_id(table)
                .ok_or(ScriptError::DbTableNotFound(table as i32))?;
            if dbrow.table != table {
                s.push_int(0); return Ok(());
            }
            let Some(columns) = &dbrow.columns else {
                return Err(ScriptError::Runtime(format!("Dbtable columns {column} is not defined!")));
            };
            let values = columns.get(column as usize)
                .ok_or(ScriptError::Runtime(format!("Dbrow column {column} out of bounds!")))?;
            let Some(types) = &dbtable.types else {
                return Err(ScriptError::Runtime(format!("Dbtable types for {table} is not defined!")));
            };
            let types = types.get(column as usize)
                .ok_or(ScriptError::Runtime(format!("Dbtable type at column {column} out of bounds!")))?;
            s.push_int(values.as_ref().map_or(0, |v| (v.len() / types.len()) as i32));
        });

        // 7504
        none!(m, DB_LISTALL_WITH_COUNT => |s| {
            db_listall(engine::<E>(), s, true)?;
        });

        // 7505
        none!(m, DB_GETROWTABLE => |s| {
            let dbrow = pop_dbrow::<E>(s)?;
            s.push_int(dbrow.table as i32);
        });

        // 7506
        none!(m, DB_FINDBYINDEX => |s| {
            if s.db_table.is_none() {
                return Err(ScriptError::Runtime("No table selected!".to_string()));
            }
            let index = s.pop_int();
            if index < 0 || index as usize >= s.db_row_query.len() {
                s.push_int(-1); // null
                return Ok(());
            }
            let id = s.db_row_query[index as usize];
            let dbrow = engine::<E>()
                .dbrows()
                .get_by_id(id)
                .ok_or(ScriptError::DbRowNotFound(id as i32))?;
            s.push_int(dbrow.id as i32);
        });

        // 7507
        none!(m, DB_FIND_REFINE_WITH_COUNT => |s| {
            db_find_refine(engine::<E>(), s, true)?;
        });

        // 7508
        none!(m, DB_FIND => |s| {
            db_find(engine::<E>(), s, false)?;
        });

        // 7509
        none!(m, DB_FIND_REFINE => |s| {
            db_find_refine(engine::<E>(), s, false)?;
        });

        // 7510
        none!(m, DB_LISTALL => |s| {
            db_listall(engine::<E>(), s, false)?;
        });
    }
}

/// Queries a table index by int or string key (`DB_FIND` / `DB_FIND_WITH_COUNT`).
///
/// Stack args (top first): the `is_string` flag (`2` = string), the query value,
/// and the packed `table << 12 | column << 4 | tuple` id. Validates the table,
/// stores the matching row ids in `db_row_query`, and resets the cursor; pushes
/// the match count when `with_count` is set.
fn db_find<E: ScriptEngine + 'static>(
    engine: &'static E,
    s: &mut ScriptState,
    with_count: bool,
) -> crate::Result<()> {
    let (query, packed) = decode_query(s);
    let table = ((packed >> 12) & 0xFFFF) as u16;
    engine
        .dbtables()
        .get_by_id(table)
        .ok_or(ScriptError::DbTableNotFound(table as i32))?;
    s.db_table = Some(table);
    s.db_row = None;
    s.db_row_query = engine.dbindex().find(&query, packed).to_vec();
    finish_query(s, with_count)
}

/// Pops a string/int query key and the packed field id from the stack.
///
/// Pops (top first): the `is_string` flag (`2` = string), the query value, and
/// the packed `table << 12 | column << 4 | tuple` id. Shared by the `DB_FIND`
/// and `DB_FIND_REFINE` families, which decode their arguments identically.
fn decode_query(s: &mut ScriptState) -> (DbIndexKey, u32) {
    let is_string = s.pop_int() == 2;
    let query = if is_string {
        DbIndexKey::String(s.pop_string().into_boxed_str())
    } else {
        DbIndexKey::Int(s.pop_int())
    };
    let packed = s.pop_int() as u32;
    (query, packed)
}

/// Finalizes a query op by pushing the match count when `with_count` is set.
///
/// Shared tail of `db_find`, `db_listall`, and `db_find_refine`.
fn finish_query(s: &mut ScriptState, with_count: bool) -> crate::Result<()> {
    if with_count {
        let count = s.db_row_query.len() as i32;
        s.push_int(count);
    }
    Ok(())
}

/// Selects every row in a table (`DB_LISTALL` / `DB_LISTALL_WITH_COUNT`).
///
/// Pops the table id, validates it, then fills `db_row_query` with the ids of
/// all rows whose `table` matches; pushes the count when `with_count` is set.
fn db_listall<E: ScriptEngine + 'static>(
    engine: &'static E,
    s: &mut ScriptState,
    with_count: bool,
) -> crate::Result<()> {
    let table = s.pop_int();
    let table_id = table as u16;
    engine
        .dbtables()
        .get_by_id(table_id)
        .ok_or(ScriptError::DbTableNotFound(table))?;
    s.db_table = Some(table_id);
    s.db_row = None;
    s.db_row_query = engine
        .dbrows()
        .types
        .iter()
        .filter(|row| row.table == table_id)
        .map(|row| row.id)
        .collect();
    finish_query(s, with_count)
}

/// Refines the current result set (`DB_FIND_REFINE` / `DB_FIND_REFINE_WITH_COUNT`).
///
/// Runs a fresh index query and keeps only the rows already present in
/// `db_row_query` (intersection, preserving the previous order). The selected
/// table is left unchanged; pushes the count when `with_count` is set.
fn db_find_refine<E: ScriptEngine + 'static>(
    engine: &'static E,
    s: &mut ScriptState,
    with_count: bool,
) -> crate::Result<()> {
    let (query, packed) = decode_query(s);
    let found = engine.dbindex().find(&query, packed);
    let found_set: FxHashSet<u16> = found.iter().copied().collect();
    let prev = std::mem::take(&mut s.db_row_query);
    s.db_row = None;
    s.db_row_query = prev
        .into_iter()
        .filter(|id| found_set.contains(id))
        .collect();
    finish_query(s, with_count)
}
