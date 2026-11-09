use crate::cache::category::CategoryTypeProvider;
use crate::cache::dbrow::DbRowTypeProvider;
use crate::cache::dbtable::DbTableTypeProvider;
use crate::cache::r#enum::EnumTypeProvider;
use crate::cache::flo::FloTypeProvider;
use crate::cache::hunt::HuntTypeProvider;
use crate::cache::idk::IdkTypeProvider;
use crate::cache::r#if::IfTypeProvider;
use crate::cache::inv::InvTypeProvider;
use crate::cache::loc::LocTypeProvider;
use crate::cache::mesanim::MesAnimTypeProvider;
use crate::cache::npc::NpcTypeProvider;
use crate::cache::obj::ObjTypeProvider;
use crate::cache::param::ParamTypeProvider;
use crate::cache::seq::SeqTypeProvider;
use crate::cache::spotanim::SpotAnimTypeProvider;
use crate::cache::r#struct::StructTypeProvider;
#[cfg(since_254)]
use crate::cache::varbit::VarbitTypeProvider;
use crate::cache::varn::VarnTypeProvider;
use crate::cache::varp::VarPlayerTypeProvider;
use crate::cache::vars::VarsTypeProvider;
#[cfg(all(since_244, before_647))]
use crate::types::OndemandBlobs;
use crate::types::{MapSquareCrcs, MapSquareCsv, MapSquares};
#[cfg(since_647)]
use area::{AreaMap, AreaTypeProvider};
#[cfg(since_647)]
use controller::ControllerTypeProvider;
use dbtable::DbTableIndex;
use font::FontTypeProvider;
use midi::MidiProvider;
#[cfg(since_647)]
use rs_crypto::huffman::Huffman;
#[cfg(rev = "225")]
use seq_frame::SeqFrameProvider;
use std::collections::HashMap;
#[cfg(before_647)]
use std::sync::Arc;
#[cfg(since_647)]
use varcon::VarConTypeProvider;
#[cfg(since_647)]
use varobj::VarObjTypeProvider;
use wordenc::WordEncProvider;

#[cfg(since_647)]
pub use midi::MIDI_JINGLE_BASE;

#[cfg(since_647)]
pub mod area;
#[cfg(since_647)]
pub mod bas;
#[cfg(since_647)]
pub mod billboard;
pub mod category;
#[cfg(since_647)]
pub mod controller;
#[cfg(since_647)]
pub mod cursor;
pub mod dbrow;
pub mod dbtable;
mod decode;
pub mod r#enum;
pub mod flo;
#[cfg(since_647)]
pub mod flu;
pub mod font;
#[cfg(since_647)]
pub mod hitmark;
pub mod hunt;
pub mod idk;
pub mod r#if;
pub mod inv;
#[cfg(since_647)]
pub mod js5;
pub mod lenient;
#[cfg(since_647)]
pub mod light;
#[cfg(since_647)]
pub mod loader;
pub mod loc;
pub mod map;
#[cfg(since_647)]
pub mod mel;
pub mod mesanim;
pub mod midi;
#[cfg(since_647)]
pub mod msi;
pub mod npc;
pub mod obj;
pub mod param;
#[cfg(since_647)]
pub mod particleeffector;
#[cfg(since_647)]
pub mod particleemitter;
pub mod provider;
#[cfg(since_647)]
pub mod quest;
#[cfg(since_647)]
pub mod quickchatcat;
#[cfg(since_647)]
pub mod quickchatphrase;
pub mod script;
pub mod seq;
pub mod seq_frame;
pub mod spotanim;
pub mod r#struct;
#[cfg(since_647)]
pub mod texture;
#[cfg(since_254)]
pub mod varbit;
#[cfg(since_647)]
pub mod varcon;
pub mod varn;
#[cfg(since_647)]
pub mod varobj;
pub mod varp;
pub mod vars;
pub mod wordenc;

pub struct CacheStore {
    #[cfg(before_647)]
    pub crctable_bytes: Arc<[u8]>,
    #[cfg(before_647)]
    pub crc_buffer32: i32,
    #[cfg(all(since_244, before_274))]
    pub ondemand_zip: Arc<[u8]>,
    #[cfg(all(since_244, before_274))]
    pub build: Arc<[u8]>,
    #[cfg(all(since_244, before_647))]
    pub ondemand: OndemandBlobs,
    #[cfg(before_647)]
    pub crcs: HashMap<&'static str, i32>,
    #[cfg(before_647)]
    pub jags: HashMap<&'static str, Arc<[u8]>>,
    pub mapsquares: MapSquares,
    pub mapcrcs: MapSquareCrcs,
    pub objs: ObjTypeProvider,
    pub invs: InvTypeProvider,
    pub varps: VarPlayerTypeProvider,
    #[cfg(since_254)]
    pub varbits: VarbitTypeProvider,
    pub dbrows: DbRowTypeProvider,
    pub dbtables: DbTableTypeProvider,
    pub db_index: DbTableIndex,
    pub enums: EnumTypeProvider,
    pub flos: FloTypeProvider,
    pub hunts: HuntTypeProvider,
    pub idks: IdkTypeProvider,
    pub locs: LocTypeProvider,
    pub mesanims: MesAnimTypeProvider,
    pub npcs: NpcTypeProvider,
    pub params: ParamTypeProvider,
    #[cfg(rev = "225")]
    pub seq_frames: SeqFrameProvider,
    pub seqs: SeqTypeProvider,
    pub spotanims: SpotAnimTypeProvider,
    pub structs: StructTypeProvider,
    pub varns: VarnTypeProvider,
    pub varss: VarsTypeProvider,
    pub categories: CategoryTypeProvider,
    pub interfaces: IfTypeProvider,
    pub fonts: FontTypeProvider,
    pub wordenc: WordEncProvider,
    pub songs: MidiProvider,
    pub jingles: MidiProvider,
    #[cfg(since_244)]
    pub midi_ids: HashMap<Box<str>, u16>,
    #[cfg(since_254)]
    pub midi_tick_lengths: Box<[Option<u16>]>,
    #[cfg(before_647)]
    pub static_assets: HashMap<Box<str>, Arc<[u8]>>,
    #[cfg(before_647)]
    pub multimap: MapSquareCsv,
    pub freemap: MapSquareCsv,
    #[cfg(since_647)]
    pub huffman: Huffman,
    #[cfg(since_647)]
    pub areas: AreaTypeProvider,
    #[cfg(since_647)]
    pub area_map: AreaMap,
    #[cfg(since_647)]
    pub multiway_area: Option<u16>,
    #[cfg(since_647)]
    pub varobjs: VarObjTypeProvider,
    #[cfg(since_647)]
    pub varcons: VarConTypeProvider,
    #[cfg(since_647)]
    pub controllers: ControllerTypeProvider,
}

pub use crate::types::ScriptVarType;

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
    Midi(i32),
    #[cfg(since_647)]
    Cursor(i32),
    #[cfg(since_647)]
    MapArea(i32),
    #[cfg(since_647)]
    Graphic(i32),
    #[cfg(since_647)]
    Jingle(i32),
    #[cfg(since_647)]
    ChatCat(i32),
    #[cfg(since_647)]
    Model(i32),
    #[cfg(since_647)]
    Texture(i32),
    #[cfg(since_647)]
    Bas(i32),
    #[cfg(since_647)]
    Vorbis(i32),
    #[cfg(since_647)]
    MapElement(i32),
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
            ScriptVarType::Midi => VarValue::Midi(value),
            #[cfg(since_647)]
            ScriptVarType::Cursor => VarValue::Cursor(value),
            #[cfg(since_647)]
            ScriptVarType::MapArea => VarValue::MapArea(value),
            #[cfg(since_647)]
            ScriptVarType::Graphic => VarValue::Graphic(value),
            #[cfg(since_647)]
            ScriptVarType::Jingle => VarValue::Jingle(value),
            #[cfg(since_647)]
            ScriptVarType::ChatCat => VarValue::ChatCat(value),
            #[cfg(since_647)]
            ScriptVarType::Model => VarValue::Model(value),
            #[cfg(since_647)]
            ScriptVarType::Texture => VarValue::Texture(value),
            #[cfg(since_647)]
            ScriptVarType::Bas => VarValue::Bas(value),
            #[cfg(since_647)]
            ScriptVarType::Vorbis => VarValue::Vorbis(value),
            #[cfg(since_647)]
            ScriptVarType::MapElement => VarValue::MapElement(value),
        }
    }

    pub fn default_for(var_type: ScriptVarType) -> Self {
        match var_type {
            ScriptVarType::Int | ScriptVarType::AutoInt => VarValue::Int(0),
            _ => Self::from_int(var_type, -1),
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
            | VarValue::DbRow(v)
            | VarValue::Midi(v) => *v,
            #[cfg(since_647)]
            VarValue::Cursor(v)
            | VarValue::MapArea(v)
            | VarValue::Graphic(v)
            | VarValue::Jingle(v)
            | VarValue::ChatCat(v)
            | VarValue::Model(v)
            | VarValue::Texture(v)
            | VarValue::Bas(v)
            | VarValue::Vorbis(v)
            | VarValue::MapElement(v) => *v,
        }
    }
}
