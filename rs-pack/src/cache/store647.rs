//! Rev-647 engine cache bridge.
//!
//! The engine consumes the same [`CacheStore`] shape at every revision; at
//! 647 the source data is the dat2 JS5 cache plus the `cache647` config
//! decoders instead of the jag-era pack pipeline. This module converts the
//! rev-647 `PackedConfig` (client cache + server-overlay TypeProviders) into
//! the engine-facing providers, and loads mapsquares / huffman / fonts
//! straight from the built JS5 store — mirroring the source repo's
//! `CacheStore::from_packed_config`.
//!
//! Field conversions keep the server-overlay semantics 1:1 (the cache647
//! decoders already carry the same server opcodes); purely client-visual
//! fields with no engine reader are dropped.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use rustc_hash::FxHashMap;

use super::CacheStore;
use super::category::CategoryType;
use super::dbrow::{DbRowType, DbRowValue};
use super::dbtable::{DbTableIndex, DbTableType, DbTableValue};
use super::r#enum::EnumType;
use super::flo::FloType;
use super::font::{FontType, FontTypeProvider};
use super::hunt::{HuntCheckInv, HuntCheckInvParam, HuntExtraCheckVar, HuntType};
use super::idk::IdkType;
use super::r#if::IfTypeProvider;
use super::inv::InvType;
use super::loc::LocType;
use super::mesanim::MesAnimType;
use super::midi::MidiProvider;
use super::npc::{NpcPatrol, NpcType};
use super::obj::ObjType;
use super::param::ParamType;
use super::provider::TypeProvider;
use super::seq::SeqType;
use super::spotanim::SpotAnimType;
use super::r#struct::StructType;
use super::varbit::VarbitType;
use super::varn::VarnType;
use super::varp::VarPlayerType;
use super::vars::VarsType;
use super::wordenc::WordEncProvider;
use crate::cache647 as c647;
use crate::cache647::CacheType as CacheType647;
use crate::startup::PackedConfig;
use crate::types::{
    BlockWalk, BodyType, DummyItem, Font, HuntCheckAfk, HuntCheckNotBusy, HuntCheckNotTooStrong,
    HuntCheckVis, HuntFindKeepHunting, HuntModeType, HuntNobodyNear, MoveRestrict, NpcMode,
    ParamValue, ScriptVarType, WearPos,
};

impl CacheStore {
    /// Build the engine-facing store from the rev-647 pack pipeline output.
    ///
    /// Accepts the pre-opened JS5 store from `PackResult` to avoid redundant
    /// cache opens. XTEA map keys must already be installed globally
    /// (`XteaKeyStore::install_global`) so landscape groups decrypt.
    pub fn from_packed_config(
        config: PackedConfig,
        cache: &mut rs_dat2::CacheStore,
        server_dir: &std::path::Path,
    ) -> Result<Box<CacheStore>> {
        let (mut mapsquares, mapcrcs) = load_mapsquares(cache)?;
        let huffman = load_huffman(cache)?;
        let fonts = load_fonts(cache);
        merge_server_spawns(server_dir, &mut mapsquares);
        merge_server_obj_spawns(server_dir, &mut mapsquares);

        let dbtables: TypeProvider<DbTableType> = convert(config.dbtables);
        let mut dbrows: TypeProvider<DbRowType> = convert(config.dbrows);
        widen_dbrows(&mut dbrows, &dbtables);
        let db_index = DbTableIndex::build(&dbtables, &dbrows);

        Ok(Box::new(CacheStore {
            mapsquares,
            mapcrcs,
            objs: convert(config.objs),
            invs: convert(config.invs),
            varps: convert(config.varps),
            varbits: convert(config.varbits),
            dbrows,
            dbtables,
            db_index,
            enums: convert(config.enums),
            flos: convert(config.flos),
            hunts: convert(config.hunts),
            idks: convert(config.idks),
            locs: convert(config.locs),
            mesanims: convert(config.mesanims),
            npcs: convert(config.npcs),
            params: convert(config.params),
            seqs: convert(config.seqs),
            spotanims: convert(config.spotanims),
            structs: convert(config.structs),
            varns: convert(config.varns),
            // No shared-var configs in the 647 content tree.
            varss: TypeProvider::<VarsType>::empty(),
            categories: load_categories(),
            // Rev-647 component metadata lives in the InterfaceLayout; the
            // jag-era .if provider has no 647 source. Reads are since_647-
            // forked at their call sites.
            interfaces: IfTypeProvider {
                names: HashMap::new(),
                types: Vec::new(),
            },
            fonts,
            // The rev-647 chat path is huffman-coded and unfiltered (matching
            // the source repo); wordenc stays empty.
            wordenc: WordEncProvider::empty(),
            songs: MidiProvider {
                names: HashMap::new(),
                midis: Box::new([]),
            },
            jingles: MidiProvider {
                names: HashMap::new(),
                midis: Box::new([]),
            },
            midi_ids: load_midi_ids(),
            midi_tick_lengths: load_midi_tick_lengths(cache),
            multimap: Default::default(),
            freemap: Default::default(),
            huffman,
        }))
    }
}

/// Convert a cache647 TypeProvider into the engine-facing one.
/// Gives each dbrow a slot for every column of its table.
///
/// A 647 dbrow carries only the columns it sets (`pack647/dbrow.rs`), so its
/// converted row stops at the last one set. 289's carries every column of its
/// table, an unset one with no fields, and `db_getfieldcount` answers 0 for
/// it; on a 647 row a column past the last set one read as out of bounds (the
/// locked doors' optional `trap_successchance`, the table's last column).
fn widen_dbrows(dbrows: &mut TypeProvider<DbRowType>, dbtables: &TypeProvider<DbTableType>) {
    for row in dbrows.types.iter_mut() {
        let Some(width) = dbtables
            .types
            .get(row.table as usize)
            .and_then(|t| t.types.as_ref())
            .map(|t| t.len())
        else {
            continue;
        };
        let have = row.columns.as_ref().map_or(0, |c| c.len());
        if have >= width {
            continue;
        }
        let mut types = row.types.take().map(Vec::from).unwrap_or_default();
        let mut columns = row.columns.take().map(Vec::from).unwrap_or_default();
        types.resize(width, Box::new([]));
        columns.resize(width, None);
        row.types = Some(types.into_boxed_slice());
        row.columns = Some(columns.into_boxed_slice());
    }
}

fn convert<A, B: From<A>>(p: c647::TypeProvider<A>) -> TypeProvider<B> {
    let mut debugnames = FxHashMap::default();
    for (name, id) in p.names {
        debugnames.insert(name, id);
    }
    TypeProvider {
        debugnames,
        types: p
            .types
            .into_vec()
            .into_iter()
            .map(B::from)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }
}

impl<T> TypeProvider<T> {
    fn empty() -> Self {
        TypeProvider {
            debugnames: FxHashMap::default(),
            types: Box::new([]),
        }
    }
}

fn opt_u16(v: i32) -> Option<u16> {
    (v >= 0).then_some(v as u16)
}

fn ops(options: [Option<Box<str>>; 5]) -> Option<Box<[Option<Box<str>>]>> {
    Some(options.into_iter().collect())
}

// The `Box<HashMap>` parameter mirrors the cache647 struct field type.
#[allow(clippy::box_collection)]
fn params_map(
    params: Option<Box<HashMap<i32, c647::ParamValue>>>,
) -> Option<Box<FxHashMap<i32, ParamValue>>> {
    params.map(|p| {
        Box::new(
            p.into_iter()
                .map(|(k, v)| (k, convert_param_value(v)))
                .collect::<FxHashMap<_, _>>(),
        )
    })
}

fn convert_param_value(v: c647::ParamValue) -> ParamValue {
    match v {
        c647::ParamValue::Int(i) => ParamValue::Int(i),
        c647::ParamValue::String(s) => ParamValue::String(s),
    }
}

fn script_var_type(c: u8) -> ScriptVarType {
    ScriptVarType::try_from(c).unwrap_or(ScriptVarType::Int)
}

impl From<c647::NpcType> for NpcType {
    fn from(n: c647::NpcType) -> Self {
        let debugname = n.debugname().map(Box::from);
        NpcType {
            id: n.id,
            name: n.name,
            desc: n.desc,
            size: n.size,
            category: opt_u16(n.category),
            op: ops(n.options),
            attack: n.attack,
            defence: n.defence,
            strength: n.strength,
            hitpoints: n.hitpoints,
            ranged: n.ranged,
            magic: n.magic,
            vislevel: opt_u16(n.combat),
            wanderrange: n.wander_range,
            maxrange: n.max_range,
            huntrange: n.hunt_range,
            timer: (n.timer > 0).then_some(n.timer),
            respawnrate: n.respawn_rate,
            moverestrict: MoveRestrict::try_from(n.move_restrict).unwrap_or(MoveRestrict::Normal),
            attackrange: u8::try_from(n.attack_range).unwrap_or(u8::MAX),
            blockwalk: BlockWalk::try_from(n.block_walk).unwrap_or(BlockWalk::Npc),
            huntmode: (n.hunt_mode > 0).then_some(n.hunt_mode as u16),
            defaultmode: NpcMode::try_from(n.default_mode).unwrap_or(NpcMode::None),
            members: n.members,
            patrol: n.patrol.map(|p| {
                p.iter()
                    .map(|&(coord, delay)| NpcPatrol { coord, delay })
                    .collect()
            }),
            givechase: n.give_chase,
            regenrate: n.regen_rate,
            #[cfg(since_289)]
            turnspeed: n.turnspeed.clamp(0, u16::MAX as i32) as u16,
            params: params_map(n.params),
            debugname,
        }
    }
}

impl From<c647::ObjType> for ObjType {
    fn from(o: c647::ObjType) -> Self {
        let debugname = o.debugname.clone();
        // Fold the partitioned server params (op 249) back into one map.
        let mut params: FxHashMap<i32, ParamValue> = o
            .params
            .map(|p| {
                p.into_iter()
                    .map(|(k, v)| (k, convert_param_value(v)))
                    .collect()
            })
            .unwrap_or_default();
        for (k, v) in o.iparams {
            params.insert(k as i32, ParamValue::Int(v));
        }
        for (k, v) in o.sparams {
            params.insert(k as i32, ParamValue::String(v.into_boxed_str()));
        }
        let wearpos = |v: i8| -> Option<WearPos> {
            if v >= 0 {
                WearPos::try_from(v as u8).ok()
            } else {
                None
            }
        };
        ObjType {
            id: o.id,
            name: o.name,
            desc: o.desc,
            stackable: o.stackable,
            cost: o.cost,
            wearpos: wearpos(o.wearpos1),
            wearpos2: wearpos(o.wearpos2),
            members: o.members,
            wearpos3: wearpos(o.wearpos3),
            op: ops(o.options),
            iop: ops(o.interface_options),
            weight: (o.weight * 1000.0) as i16,
            category: (o.category != 0).then_some(o.category),
            dummyitem: DummyItem::try_from(o.dummy_obj).unwrap_or(DummyItem::None),
            certlink: (o.certlink != 0).then_some(o.certlink),
            certtemplate: (o.certtemplate != 0).then_some(o.certtemplate),
            tradeable: o.tradeable,
            respawnrate: 100,
            params: (!params.is_empty()).then(|| Box::new(params)),
            debugname,
        }
    }
}

impl From<c647::LocType> for LocType {
    fn from(l: c647::LocType) -> Self {
        let debugname = l.debugname().map(Box::from);
        LocType {
            id: l.id,
            name: l.name,
            desc: l.desc,
            width: l.width,
            length: l.length,
            blockwalk: l.blockwalk,
            blockrange: l.blockrange,
            active: match l.active {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            },
            // A memberop (cache 150-154) takes its slot on a members world,
            // as the client's LocType does; 647's server runs one (its objs
            // load with `members: true`). 985 of 2011's loc ops are memberops
            // (farming patches' Inspect/Harvest/Clear, agility's Grapple).
            op: ops(std::array::from_fn(|i| {
                l.members_op[i].clone().or_else(|| l.op[i].clone())
            })),
            category: opt_u16(l.category),
            forceapproach: l.forceapproach,
            // 2011's multiloc: the client offers the ops of the loc its var
            // picks, and the server checks them there (handlers/oploc.rs).
            multivar: u16::try_from(l.transform_varbit).ok(),
            multivarp: u16::try_from(l.transform_varp).ok(),
            multiloc: l.transforms,
            params: l.params.map(|p| {
                Box::new(
                    p.into_iter()
                        .map(|(k, v)| (k, convert_param_value(v)))
                        .collect::<FxHashMap<_, _>>(),
                )
            }),
            debugname,
        }
    }
}

impl From<c647::SeqType> for SeqType {
    fn from(s: c647::SeqType) -> Self {
        let debugname = s.debugname().map(Box::from);
        SeqType {
            id: s.id,
            duration: s.total_duration,
            priority: s.priority,
            debugname,
        }
    }
}

impl From<c647::SpotAnimType> for SpotAnimType {
    fn from(s: c647::SpotAnimType) -> Self {
        SpotAnimType { id: s.id }
    }
}

impl From<c647::VarpType> for VarPlayerType {
    fn from(v: c647::VarpType) -> Self {
        let debugname = v.debugname().map(Box::from);
        VarPlayerType {
            id: v.id,
            scope: match v.scope {
                c647::varp_type::VarPlayerScope::Temp => crate::types::VarPlayerScope::Temp,
                c647::varp_type::VarPlayerScope::Perm => crate::types::VarPlayerScope::Perm,
            },
            var_type: v
                .var_type
                .map(script_var_type)
                .unwrap_or(ScriptVarType::Int),
            protect: v.protect,
            clientcode: v.clientcode,
            transmit: v.transmit,
            debugname,
        }
    }
}

impl From<c647::VarBitType> for VarbitType {
    fn from(v: c647::VarBitType) -> Self {
        VarbitType {
            id: v.id,
            basevar: v.varp_id,
            start_bit: v.lsb,
            end_bit: v.msb,
            debugname: None,
        }
    }
}

impl From<c647::InvType> for InvType {
    fn from(i: c647::InvType) -> Self {
        let debugname = i.debugname().map(Box::from);
        InvType {
            id: i.id,
            scope: match i.scope {
                crate::types647::InvScope::Temp => crate::types::InvScope::Temp,
                crate::types647::InvScope::Perm => crate::types::InvScope::Perm,
                crate::types647::InvScope::Shared => crate::types::InvScope::Shared,
            },
            size: i.size,
            stackall: i.stackall,
            stockobj: (!i.stock_obj.is_empty()).then(|| i.stock_obj.into_boxed_slice()),
            stockcount: (!i.stock_count.is_empty()).then(|| i.stock_count.into_boxed_slice()),
            stockrate: (!i.stock_rate.is_empty()).then(|| i.stock_rate.into_boxed_slice()),
            restock: i.restock,
            allstock: i.allstock,
            protect: i.protect,
            runweight: i.runweight,
            dummyinv: i.dummyinv,
            debugname,
        }
    }
}

impl From<c647::EnumType> for EnumType {
    fn from(e: c647::EnumType) -> Self {
        EnumType {
            id: e.id,
            inputtype: script_var_type(e.key_type),
            outputtype: script_var_type(e.value_type),
            default_int: e.default_int,
            default_str: (!e.default_string.is_empty()).then_some(e.default_string),
            values: e
                .entries
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        match v {
                            c647::EnumValue::Int(i) => ParamValue::Int(i),
                            c647::EnumValue::String(s) => ParamValue::String(s),
                        },
                    )
                })
                .collect(),
            debugname: None,
        }
    }
}

impl From<c647::StructType> for StructType {
    fn from(s: c647::StructType) -> Self {
        let params: FxHashMap<i32, ParamValue> = s
            .params
            .into_iter()
            .map(|(k, v)| (k, convert_param_value(v)))
            .collect();
        StructType {
            id: s.id,
            params: (!params.is_empty()).then(|| Box::new(params)),
            debugname: None,
        }
    }
}

impl From<c647::ParamType> for ParamType {
    fn from(p: c647::ParamType) -> Self {
        ParamType {
            id: p.id,
            var_type: p
                .type_char
                .map(script_var_type)
                .unwrap_or(ScriptVarType::Int),
            default_int: p.default_int.unwrap_or(0),
            default_str: p.default_string,
            autodisable: p.auto_disable,
            debugname: None,
        }
    }
}

impl From<c647::IdkType> for IdkType {
    fn from(i: c647::IdkType) -> Self {
        IdkType {
            id: i.id,
            body_type: i
                .body_type
                .and_then(|b| BodyType::try_from(b).ok())
                .unwrap_or(BodyType::ManHair),
            disable: i.disable,
        }
    }
}

impl From<c647::MesAnimType> for MesAnimType {
    fn from(m: c647::MesAnimType) -> Self {
        let debugname = m.debugname().map(Box::from);
        MesAnimType {
            id: m.id,
            len: m.len,
            debugname,
        }
    }
}

impl From<c647::HuntType> for HuntType {
    fn from(h: c647::HuntType) -> Self {
        let debugname = h.debugname().map(Box::from);
        let (check_inv, check_invparam) = match h.check_inv_opcode {
            16 => (
                Some(HuntCheckInv {
                    inv: h.check_inv.max(0) as u16,
                    obj: h.check_inv_obj.max(0) as u16,
                    condition: h.check_inv_condition.clone(),
                    value: h.check_inv_val,
                }),
                None,
            ),
            17 => (
                None,
                Some(HuntCheckInvParam {
                    inv: h.check_inv.max(0) as u16,
                    param: h.check_obj_param.max(0) as u16,
                    condition: h.check_inv_condition.clone(),
                    value: h.check_inv_val,
                }),
            ),
            _ => (None, None),
        };
        HuntType {
            id: h.id,
            // 2011's hunt configs carry no inventory-category check.
            check_invcat: None,
            hunt_type: HuntModeType::try_from(h.hunt_type).unwrap_or(HuntModeType::Off),
            check_vis: HuntCheckVis::try_from(h.check_vis).unwrap_or(HuntCheckVis::Off),
            check_nottoostrong: HuntCheckNotTooStrong::try_from(h.check_not_too_strong)
                .unwrap_or(HuntCheckNotTooStrong::Off),
            check_notbusy: if h.check_not_busy {
                HuntCheckNotBusy::On
            } else {
                HuntCheckNotBusy::Off
            },
            find_keephunting: if h.find_keep_hunting {
                HuntFindKeepHunting::On
            } else {
                HuntFindKeepHunting::Off
            },
            find_newmode: NpcMode::try_from(h.find_new_mode).unwrap_or(NpcMode::None),
            nobodynear: HuntNobodyNear::try_from(h.nobody_near)
                .unwrap_or(HuntNobodyNear::PauseHunt),
            check_notcombat: opt_u16(h.check_not_combat),
            check_notcombat_self: opt_u16(h.check_not_combat_self),
            check_afk: if h.check_afk {
                HuntCheckAfk::On
            } else {
                HuntCheckAfk::Off
            },
            rate: h.rate,
            check_category: opt_u16(h.check_category),
            check_npc: opt_u16(h.check_npc),
            check_obj: opt_u16(h.check_obj),
            check_loc: opt_u16(h.check_loc),
            check_inv,
            check_invparam,
            extracheck_vars: h
                .check_vars
                .into_iter()
                .map(|(_op, var, condition, value)| HuntExtraCheckVar {
                    varp: var,
                    condition,
                    value,
                })
                .collect(),
            debugname,
        }
    }
}

impl From<c647::FloType> for FloType {
    fn from(f: c647::FloType) -> Self {
        let debugname = f.debugname().map(Box::from);
        FloType {
            id: f.id,
            colour: f.color.unwrap_or(0) as i32,
            texture: (f.texture >= 0 && f.texture <= u8::MAX as i32).then_some(f.texture as u8),
            overlay: false,
            occlude: f.occlude,
            debugname,
        }
    }
}

impl From<c647::VarnType> for VarnType {
    fn from(v: c647::VarnType) -> Self {
        let debugname = v.debugname().map(Box::from);
        VarnType {
            id: v.id,
            var_type: script_var_type(v.var_type as u8),
            debugname,
        }
    }
}

impl From<c647::DbRowType> for DbRowType {
    fn from(r: c647::DbRowType) -> Self {
        let column_count = r.columns.iter().map(|c| c.col_idx + 1).max().unwrap_or(0);
        let mut types: Vec<Box<[u8]>> = vec![Box::new([]); column_count];
        let mut columns: Vec<Option<Box<[DbRowValue]>>> = vec![None; column_count];
        for col in r.columns {
            types[col.col_idx] = col.types.clone().into_boxed_slice();
            columns[col.col_idx] = Some(
                col.values
                    .into_iter()
                    .map(|v| match v {
                        c647::DbValue::Int(i) => DbRowValue::Int(i),
                        c647::DbValue::String(s) => DbRowValue::String(s),
                    })
                    .collect(),
            );
        }
        DbRowType {
            id: r.id,
            types: (column_count > 0).then(|| types.into_boxed_slice()),
            columns: (column_count > 0).then(|| columns.into_boxed_slice()),
            table: r.table_id.max(0) as u16,
            debugname: r.debugname.map(|d| d.into_boxed_str()),
        }
    }
}

impl From<c647::DbTableType> for DbTableType {
    fn from(t: c647::DbTableType) -> Self {
        let count = t.columns.len();
        let mut types: Vec<Box<[u8]>> = Vec::with_capacity(count);
        let mut defaults: Vec<Option<Box<[DbTableValue]>>> = Vec::with_capacity(count);
        let mut names: Vec<Box<str>> = Vec::with_capacity(count);
        let mut props: Vec<u8> = Vec::with_capacity(count);
        for col in t.columns {
            types.push(col.types.clone().into_boxed_slice());
            defaults.push((!col.defaults.is_empty()).then(|| {
                col.defaults
                    .into_iter()
                    .map(|v| match v {
                        c647::DbValue::Int(i) => DbTableValue::Int(i),
                        c647::DbValue::String(s) => DbTableValue::String(s),
                    })
                    .collect()
            }));
            names.push(col.name.unwrap_or_default().into_boxed_str());
            props.push(col.props);
        }
        DbTableType {
            id: t.id,
            types: (count > 0).then(|| types.into_boxed_slice()),
            defaults: (count > 0).then(|| defaults.into_boxed_slice()),
            columns: (count > 0).then(|| names.into_boxed_slice()),
            props: (count > 0).then(|| props.into_boxed_slice()),
            debugname: t.debugname.map(|d| d.into_boxed_str()),
        }
    }
}

// ── JS5 asset loaders (ported from the source repo's from_packed_config) ────

fn load_mapsquares(
    cache: &mut rs_dat2::CacheStore,
) -> Result<(crate::types::MapSquares, crate::types::MapSquareCrcs)> {
    let ref_table = match cache.reference_table(rs_dat2::archive::MAPS) {
        Some(rt) => rt.clone(),
        None => {
            tracing::warn!("No reference table for archive 5 (MAPS)");
            return Ok((Default::default(), Default::default()));
        }
    };

    let xtea = rs_dat2::xtea::XteaKeyStore::global();

    let name_lookup = build_map_name_lookup();
    let mut mapsquares = crate::types::MapSquares::default();
    let mut mapcrcs = crate::types::MapSquareCrcs::default();

    for group in &ref_table.groups {
        let Some(&(kind, mx, mz)) = name_lookup.get(&group.name_hash) else {
            continue;
        };

        let data = if kind == 'l' {
            // Landscape groups may be XTEA-encrypted.
            let keys = xtea
                .as_ref()
                .map(|x| x.get(mx as u32, mz as u32))
                .unwrap_or([0; 4]);
            match cache.read_group_xtea(rs_dat2::archive::MAPS, group.id, &keys) {
                Ok(Some(d)) => d.to_vec(),
                _ => continue,
            }
        } else {
            match cache.read_group_raw_decompressed(rs_dat2::archive::MAPS, group.id) {
                Ok(Some(d)) => d.to_vec(),
                _ => continue,
            }
        };

        // Re-compress for storage (the engine decompresses on demand).
        let compressed =
            rs_dat2::compression::compress(&data, rs_dat2::compression::CompressionType::Gzip)
                .unwrap_or_default();

        mapsquares.insert((kind, mx, mz), Arc::from(compressed.as_slice()));
        mapcrcs.insert((kind, mx, mz), group.crc as i32);
    }

    tracing::info!(
        "Loaded {} mapsquare groups from JS5 cache",
        mapsquares.len()
    );
    Ok((mapsquares, mapcrcs))
}

/// Merge server NPC spawn regions (`map.dat`/`map.idx`) into the `n` layer.
fn merge_server_spawns(server_dir: &std::path::Path, mapsquares: &mut crate::types::MapSquares) {
    merge_server_layer(server_dir, "map", 'n', mapsquares);
}

/// Merge global ground-item spawns (`objspawn.dat`/`objspawn.idx`) into the
/// `o` layer; `GameMap::load_objs` reads these.
fn merge_server_obj_spawns(
    server_dir: &std::path::Path,
    mapsquares: &mut crate::types::MapSquares,
) {
    merge_server_layer(server_dir, "objspawn", 'o', mapsquares);
}

fn merge_server_layer(
    server_dir: &std::path::Path,
    stem: &str,
    kind: char,
    mapsquares: &mut crate::types::MapSquares,
) {
    let Ok(dat) = std::fs::read(server_dir.join(format!("{stem}.dat"))) else {
        return;
    };
    let Ok(idx) = std::fs::read(server_dir.join(format!("{stem}.idx"))) else {
        return;
    };
    if dat.len() < 2 || idx.len() < 2 {
        return;
    }

    let count = u16::from_be_bytes([dat[0], dat[1]]) as usize;
    let mut dat_offset = 2;
    let mut loaded = 0usize;

    for i in 0..count {
        let idx_off = 2 + i * 2;
        if idx_off + 1 >= idx.len() {
            break;
        }
        let len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;
        if len >= 2 && dat_offset + len <= dat.len() {
            let mx = dat[dat_offset];
            let mz = dat[dat_offset + 1];
            let spawn_data = &dat[dat_offset + 2..dat_offset + len];

            // Wrap in a no-compression container so the map decompressor can
            // handle it uniformly.
            let data_len = spawn_data.len() as u32;
            let mut container = Vec::with_capacity(5 + spawn_data.len());
            container.push(0); // CompressionType::None
            container.extend_from_slice(&data_len.to_be_bytes());
            container.extend_from_slice(spawn_data);

            mapsquares.insert((kind, mx, mz), Arc::from(container.as_slice()));
            loaded += 1;
        }
        dat_offset += len;
    }

    tracing::info!("Loaded {loaded} {stem} regions from server {stem}.dat");
}

fn load_huffman(cache: &mut rs_dat2::CacheStore) -> Result<rs_crypto::huffman::Huffman> {
    let target_hash = rs_dat2::xtea::java_string_hash("huffman");
    let ref_table = cache
        .reference_table(rs_dat2::archive::BINARY)
        .ok_or_else(|| anyhow::anyhow!("No reference table for archive 10 (BINARY)"))?
        .clone();

    for group in &ref_table.groups {
        if group.name_hash != target_hash {
            continue;
        }
        if let Some(g) = cache.read_group(rs_dat2::archive::BINARY, group.id)? {
            let bits = &g.entries[0].data;
            tracing::info!("Loaded Huffman table ({} symbols) from cache", bits.len());
            return Ok(rs_crypto::huffman::Huffman::new(bits));
        }
    }
    anyhow::bail!("Huffman group not found in archive 10 (BINARY)")
}

/// Load fontmetrics glyph widths for chat message word-wrapping.
///
/// Two lookup paths have to work at once:
///
/// * The engine addresses the four well-known faces by [`Font`] ordinal
///   (`cache().fonts.get(Font::P12)`), so those keep slots 0..3.
/// * RuneScript addresses a font by name, and from 647 the compiler resolves
///   that name through `fontmetrics.pack` to the font's **cache group id** --
///   `p12_full` is 495, not 1. Every group in archive 13 is therefore loaded and
///   registered under its own id, which is what `~mesbox`'s `split_init(..,
///   p12_full)` actually asks for. Without that mapping the VM answered
///   `FontNotFound(495)` and every mesbox in the game died mid-script.
fn load_fonts(cache: &mut rs_dat2::CacheStore) -> FontTypeProvider {
    let slot_names: [(&str, Font); 4] = [
        ("p11_full", Font::P11),
        ("p12_full", Font::P12),
        ("b12_full", Font::B12),
        ("q8_full", Font::Q8),
    ];
    let mut fonts: Vec<FontType> = Font::ALL
        .iter()
        .map(|&f| FontType {
            id: f as u8,
            draw_width: [0; 256],
        })
        .collect();
    let mut id_index: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();

    let Some(ref_table) = cache
        .reference_table(rs_dat2::archive::FONTMETRICS)
        .cloned()
    else {
        tracing::warn!("No reference table for archive 13 (FONTMETRICS)");
        return FontTypeProvider::from_slots(fonts);
    };

    // Which group backs each ordinal slot, so a script naming one of the four
    // well-known faces resolves to the same FontType the engine uses.
    let mut slot_of_group: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for (name, slot) in slot_names {
        let hash = rs_dat2::xtea::java_string_hash(name);
        if let Some(group) = ref_table.groups.iter().find(|g| g.name_hash == hash) {
            slot_of_group.insert(group.id, slot as usize);
        }
    }

    for group in &ref_table.groups {
        let Ok(Some(g)) = cache.read_group(rs_dat2::archive::FONTMETRICS, group.id) else {
            continue;
        };
        let Some(entry) = g.entries.first() else {
            continue;
        };
        // 2-byte header then one advance per code point.
        if entry.data.len() < 258 {
            continue;
        }
        let mut draw_width = [0u8; 256];
        draw_width.copy_from_slice(&entry.data[2..258]);

        let index = match slot_of_group.get(&group.id) {
            Some(&slot) => {
                fonts[slot].draw_width = draw_width;
                slot
            }
            None => {
                fonts.push(FontType {
                    // FontType::id is a u8 slot tag; cache ids do not fit and are
                    // carried by `id_index` instead.
                    id: 0,
                    draw_width,
                });
                fonts.len() - 1
            }
        };
        id_index.insert(group.id as u16, index);
    }

    FontTypeProvider { fonts, id_index }
}

/// Category ids come from the content pack registry (`category.pack`); the
/// engine only resolves them by debugname (cheat args, hunt checks).
fn load_categories() -> TypeProvider<CategoryType> {
    let path = std::path::Path::new(crate::PACK_DIR).join("category.pack");
    let mut debugnames = FxHashMap::default();
    let mut max_id = 0u16;
    let mut entries: Vec<(u16, Box<str>)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&path) {
        for line in text.lines() {
            let Some((id, name)) = line.split_once('=') else {
                continue;
            };
            let Ok(id) = id.trim().parse::<u16>() else {
                continue;
            };
            let name: Box<str> = name.trim().into();
            max_id = max_id.max(id);
            entries.push((id, name));
        }
    }
    if entries.is_empty() {
        return TypeProvider::empty();
    }
    let mut types: Vec<CategoryType> = (0..=max_id)
        .map(|id| CategoryType {
            id,
            debugname: None,
        })
        .collect();
    for (id, name) in entries {
        debugnames.insert(name.clone(), id);
        types[id as usize].debugname = Some(name);
    }
    TypeProvider {
        debugnames,
        types: types.into_boxed_slice(),
    }
}

/// Where a jingle sits among the `midi` ids (content/647/pack/midi.pack): a
/// song at its own 2011 id, a jingle at this plus its id, as 2011 keeps songs
/// and jingles apart and 289's `midi` names both.
pub const MIDI_JINGLE_BASE: usize = 10000;

/// Each 2011 song's and jingle's length in game ticks, by `midi` id, as
/// 289's `midi_length` gives it (`ceil(ms / 600) + 1`, `MidiType`). 2011's
/// music is Jagex MIDI (archives 6 and 11), decoded as the client does.
fn load_midi_tick_lengths(cache: &mut rs_dat2::CacheStore) -> Box<[Option<u16>]> {
    let mut lengths: Vec<Option<u16>> = Vec::new();
    for (archive, base) in [
        (rs_dat2::archive::SONGS, 0),
        (rs_dat2::archive::JINGLES, MIDI_JINGLE_BASE),
    ] {
        let Some(table) = cache.reference_table(archive).cloned() else {
            continue;
        };
        for group in &table.groups {
            let Ok(Some(g)) = cache.read_group(archive, group.id) else {
                continue;
            };
            let Some(ms) = g
                .entries
                .first()
                .and_then(|e| crate::repack::midi::decode_jagex_midi(&e.data).ok())
                .and_then(|smf| super::midi::parse_midi_length(&smf))
            else {
                continue;
            };
            let i = base + group.id as usize;
            if i >= lengths.len() {
                lengths.resize(i + 1, None);
            }
            lengths[i] = Some(((ms as f64 / 600.0).ceil() as u32 + 1) as u16);
        }
    }
    tracing::info!(
        "Loaded {} song and jingle lengths",
        lengths.iter().flatten().count()
    );
    lengths.into_boxed_slice()
}

/// Song/jingle name → id map from the content pack registries, for the
/// cheat-arg parser and any script-side name resolution.
fn load_midi_ids() -> HashMap<Box<str>, u16> {
    let mut ids = HashMap::new();
    for stem in ["songs", "jingles"] {
        let path = std::path::Path::new(crate::PACK_DIR).join(format!("{stem}.pack"));
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            let Some((id, name)) = line.split_once('=') else {
                continue;
            };
            if let Ok(id) = id.trim().parse::<u16>() {
                ids.insert(name.trim().into(), id);
            }
        }
    }
    ids
}

/// name_hash → (kind, mx, mz) lookup for all possible map group names.
fn build_map_name_lookup() -> HashMap<i32, (char, u8, u8)> {
    let prefixes: &[(&str, char)] = &[
        ("m", 'm'),
        ("l", 'l'),
        ("n", 'n'),
        ("um", 'u'),
        ("ul", 'U'),
        ("e", 'e'),
    ];
    let mut map = HashMap::new();
    for mx in 0..=255u8 {
        for mz in 0..=255u8 {
            for &(prefix, kind) in prefixes {
                let name = format!("{prefix}{mx}_{mz}");
                let hash = rs_dat2::xtea::java_string_hash(&name);
                map.insert(hash, (kind, mx, mz));
            }
        }
    }
    map
}
