use std::collections::HashMap;
use std::sync::Arc;

use crate::cache::r#enum::EnumType;
use crate::cache::r#if::IfTypeProvider;
use crate::cache::r#struct::StructType;
#[cfg(since_244)]
use crate::types::OndemandBlobs;
use crate::types::{MapSquareCrcs, MapSquareCsv, MapSquares};
use category::CategoryType;
use dbrow::DbRowType;
use dbtable::{DbTableIndex, DbTableType};
use flo::FloType;
use font::FontTypeProvider;
use hunt::HuntType;
use idk::IdkType;
use inv::InvType;
use loc::LocType;
use mesanim::MesAnimType;
use midi::MidiProvider;
use npc::NpcType;
use obj::ObjType;
use param::ParamType;
use provider::TypeProvider;
use seq::SeqType;
#[cfg(rev = "225")]
use seq_frame::SeqFrameProvider;
use spotanim::SpotAnimType;
use varn::VarnType;
use varp::VarPlayerType;
use vars::VarsType;
use wordenc::WordEncProvider;

pub mod category;
pub mod dbrow;
pub mod dbtable;
pub mod r#enum;
pub mod flo;
pub mod font;
pub mod hunt;
pub mod idk;
pub mod r#if;
pub mod inv;
pub mod loc;
pub mod mesanim;
pub mod midi;
pub mod npc;
pub mod obj;
pub mod param;
pub mod provider;
pub mod script;
pub mod seq;
pub mod seq_frame;
pub mod spotanim;
pub mod r#struct;
pub mod varn;
pub mod varp;
pub mod vars;
pub mod wordenc;

pub struct CacheStore {
    pub crctable: [i32; 9],
    pub crctable_bytes: Arc<[u8]>,
    #[cfg(since_244)]
    pub ondemand_zip: Arc<[u8]>,
    #[cfg(since_244)]
    pub build: Arc<[u8]>,
    #[cfg(since_244)]
    pub ondemand: OndemandBlobs,
    pub crcs: HashMap<&'static str, i32>,
    pub jags: HashMap<&'static str, Arc<[u8]>>,
    pub mapsquares: MapSquares,
    pub mapcrcs: MapSquareCrcs,
    pub objs: TypeProvider<ObjType>,
    pub invs: TypeProvider<InvType>,
    pub varps: TypeProvider<VarPlayerType>,
    pub dbrows: TypeProvider<DbRowType>,
    pub dbtables: TypeProvider<DbTableType>,
    pub db_index: DbTableIndex,
    pub enums: TypeProvider<EnumType>,
    pub flos: TypeProvider<FloType>,
    pub hunts: TypeProvider<HuntType>,
    pub idks: TypeProvider<IdkType>,
    pub locs: TypeProvider<LocType>,
    pub mesanims: TypeProvider<MesAnimType>,
    pub npcs: TypeProvider<NpcType>,
    pub params: TypeProvider<ParamType>,
    #[cfg(rev = "225")]
    pub seq_frames: SeqFrameProvider,
    pub seqs: TypeProvider<SeqType>,
    pub spotanims: TypeProvider<SpotAnimType>,
    pub structs: TypeProvider<StructType>,
    pub varns: TypeProvider<VarnType>,
    pub varss: TypeProvider<VarsType>,
    pub categories: TypeProvider<CategoryType>,
    pub interfaces: IfTypeProvider,
    pub fonts: FontTypeProvider,
    pub wordenc: WordEncProvider,
    pub songs: MidiProvider,
    pub jingles: MidiProvider,
    #[cfg(since_244)]
    pub midi_ids: HashMap<Box<str>, u16>,
    pub static_assets: HashMap<Box<str>, Arc<[u8]>>,
    pub multimap: MapSquareCsv,
    pub freemap: MapSquareCsv,
}

pub use crate::types::ScriptVarType;

impl CacheStore {
    pub fn is_multi(&self, x: u16, z: u16, y: u8) -> bool {
        let zone_key = ((x >> 3) & 0x7FF) as u32
            | ((((z >> 3) & 0x7FF) as u32) << 11)
            | (((y & 0x3) as u32) << 22);
        self.multimap.contains(&zone_key)
    }

    pub fn is_free(&self, x: u16, z: u16) -> bool {
        let zone_key = ((x >> 3) & 0x7FF) as u32 | ((((z >> 3) & 0x7FF) as u32) << 11);
        self.freemap.contains(&zone_key)
    }

    /// Returns `true` if any of the four orthogonally-adjacent tiles is
    /// flagged free-to-play.
    ///
    /// Used during map loading so that collision and zone allocation extend
    /// one tile into the members area bordering free-to-play land, keeping
    /// pathing and line-of-sight correct at the boundary.
    pub fn borders_free(&self, x: u16, z: u16) -> bool {
        self.is_free(x + 1, z)
            || self.is_free(x.saturating_sub(1), z)
            || self.is_free(x, z + 1)
            || self.is_free(x, z.saturating_sub(1))
    }
}

#[derive(Debug, Clone)]
pub enum VarValue {
    Int(i32),
    AutoInt(i32),
    String(String),
    Enum(i32),
    Obj(i32),
    Loc(i32),
    Component(i32),
    NamedObj(i32),
    Struct(i32),
    Boolean(i32),
    Coord(i32),
    Category(i32),
    Spotanim(i32),
    Npc(i32),
    Inv(i32),
    Synth(i32),
    Seq(i32),
    Stat(i32),
    Varp(i32),
    PlayerUid(i32),
    NpcUid(i32),
    Interface(i32),
    NpcStat(i32),
    Idkit(i32),
    DbRow(i32),
}

impl VarValue {
    pub fn from_int(var_type: ScriptVarType, value: i32) -> Self {
        match var_type {
            ScriptVarType::Int => VarValue::Int(value),
            ScriptVarType::AutoInt => VarValue::AutoInt(value),
            ScriptVarType::String => VarValue::String(String::new()),
            ScriptVarType::Enum => VarValue::Enum(value),
            ScriptVarType::Obj => VarValue::Obj(value),
            ScriptVarType::Loc => VarValue::Loc(value),
            ScriptVarType::Component => VarValue::Component(value),
            ScriptVarType::NamedObj => VarValue::NamedObj(value),
            ScriptVarType::Struct => VarValue::Struct(value),
            ScriptVarType::Boolean => VarValue::Boolean(value),
            ScriptVarType::Coord => VarValue::Coord(value),
            ScriptVarType::Category => VarValue::Category(value),
            ScriptVarType::Spotanim => VarValue::Spotanim(value),
            ScriptVarType::Npc => VarValue::Npc(value),
            ScriptVarType::Inv => VarValue::Inv(value),
            ScriptVarType::Synth => VarValue::Synth(value),
            ScriptVarType::Seq => VarValue::Seq(value),
            ScriptVarType::Stat => VarValue::Stat(value),
            ScriptVarType::Varp => VarValue::Varp(value),
            ScriptVarType::PlayerUid => VarValue::PlayerUid(value),
            ScriptVarType::NpcUid => VarValue::NpcUid(value),
            ScriptVarType::Interface => VarValue::Interface(value),
            ScriptVarType::NpcStat => VarValue::NpcStat(value),
            ScriptVarType::Idkit => VarValue::Idkit(value),
            ScriptVarType::DbRow => VarValue::DbRow(value),
        }
    }

    pub fn default_for(var_type: ScriptVarType) -> Self {
        match var_type {
            ScriptVarType::Int | ScriptVarType::AutoInt => VarValue::Int(0),
            ScriptVarType::String => VarValue::String(String::new()),
            ScriptVarType::Boolean => VarValue::Boolean(-1),
            ScriptVarType::Obj => VarValue::Obj(-1),
            ScriptVarType::NamedObj => VarValue::NamedObj(-1),
            ScriptVarType::Npc => VarValue::Npc(-1),
            ScriptVarType::Loc => VarValue::Loc(-1),
            ScriptVarType::Component => VarValue::Component(-1),
            ScriptVarType::Interface => VarValue::Interface(-1),
            ScriptVarType::Enum => VarValue::Enum(-1),
            ScriptVarType::Struct => VarValue::Struct(-1),
            ScriptVarType::Coord => VarValue::Coord(-1),
            ScriptVarType::Category => VarValue::Category(-1),
            ScriptVarType::Spotanim => VarValue::Spotanim(-1),
            ScriptVarType::Inv => VarValue::Inv(-1),
            ScriptVarType::Synth => VarValue::Synth(-1),
            ScriptVarType::Seq => VarValue::Seq(-1),
            ScriptVarType::Stat => VarValue::Stat(-1),
            ScriptVarType::NpcStat => VarValue::NpcStat(-1),
            ScriptVarType::Varp => VarValue::Varp(-1),
            ScriptVarType::PlayerUid => VarValue::PlayerUid(-1),
            ScriptVarType::NpcUid => VarValue::NpcUid(-1),
            ScriptVarType::Idkit => VarValue::Idkit(-1),
            ScriptVarType::DbRow => VarValue::DbRow(-1),
        }
    }

    pub fn as_int(&self) -> i32 {
        match self {
            VarValue::String(_) => -1,
            VarValue::Int(v)
            | VarValue::AutoInt(v)
            | VarValue::Enum(v)
            | VarValue::Obj(v)
            | VarValue::Loc(v)
            | VarValue::Component(v)
            | VarValue::NamedObj(v)
            | VarValue::Struct(v)
            | VarValue::Boolean(v)
            | VarValue::Coord(v)
            | VarValue::Category(v)
            | VarValue::Spotanim(v)
            | VarValue::Npc(v)
            | VarValue::Inv(v)
            | VarValue::Synth(v)
            | VarValue::Seq(v)
            | VarValue::Stat(v)
            | VarValue::Varp(v)
            | VarValue::PlayerUid(v)
            | VarValue::NpcUid(v)
            | VarValue::Interface(v)
            | VarValue::NpcStat(v)
            | VarValue::Idkit(v)
            | VarValue::DbRow(v) => *v,
        }
    }
}
