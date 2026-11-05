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
#[cfg(since_647)]
use crate::startup::PackedConfig;
#[cfg(all(since_244, before_647))]
use crate::types::OndemandBlobs;
use crate::types::{MapSquareCrcs, MapSquareCsv, MapSquares};
#[cfg(since_647)]
use crate::{CONTENT_DIR, zone_csv::load_csv_zones};
#[cfg(since_647)]
use anyhow::Result;
use dbtable::DbTableIndex;
use font::FontTypeProvider;
use midi::MidiProvider;
#[cfg(since_647)]
use provider::TypeProvider;
#[cfg(since_647)]
use rs_crypto::huffman::Huffman;
#[cfg(since_647)]
use rs_dat2::CacheStore as Js5CacheStore;
#[cfg(rev = "225")]
use seq_frame::SeqFrameProvider;
use std::collections::HashMap;
#[cfg(since_647)]
use std::path::Path;
#[cfg(before_647)]
use std::sync::Arc;
use wordenc::WordEncProvider;

#[cfg(since_647)]
pub use midi::MIDI_JINGLE_BASE;

pub mod category;
pub mod dbrow;
pub mod dbtable;
mod decode;
pub mod r#enum;
pub mod flo;
#[cfg(since_647)]
pub mod flu;
pub mod font;
pub mod hunt;
pub mod idk;
pub mod r#if;
pub mod inv;
pub mod lenient;
#[cfg(since_647)]
pub mod loader;
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
#[cfg(since_254)]
pub mod varbit;
pub mod varn;
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
    pub multimap: MapSquareCsv,
    pub freemap: MapSquareCsv,
    #[cfg(since_647)]
    pub huffman: Huffman,
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
            ScriptVarType::Midi => VarValue::Midi(-1),
            #[cfg(since_647)]
            ScriptVarType::Cursor => VarValue::Cursor(-1),
            #[cfg(since_647)]
            ScriptVarType::MapArea => VarValue::MapArea(-1),
            #[cfg(since_647)]
            ScriptVarType::Graphic => VarValue::Graphic(-1),
            #[cfg(since_647)]
            ScriptVarType::Jingle => VarValue::Jingle(-1),
            #[cfg(since_647)]
            ScriptVarType::ChatCat => VarValue::ChatCat(-1),
            #[cfg(since_647)]
            ScriptVarType::Model => VarValue::Model(-1),
            #[cfg(since_647)]
            ScriptVarType::Texture => VarValue::Texture(-1),
            #[cfg(since_647)]
            ScriptVarType::Bas => VarValue::Bas(-1),
            #[cfg(since_647)]
            ScriptVarType::Vorbis => VarValue::Vorbis(-1),
            #[cfg(since_647)]
            ScriptVarType::MapElement => VarValue::MapElement(-1),
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

#[cfg(since_647)]
impl CacheStore {
    pub fn from_packed_config(
        config: PackedConfig,
        cache: &mut Js5CacheStore,
        server_dir: &Path,
    ) -> Result<Box<CacheStore>> {
        let (mut mapsquares, mapcrcs) = loader::load_mapsquares(cache)?;
        let huffman = loader::load_huffman(cache)?;
        let fonts = FontTypeProvider::from_cache(cache);
        loader::merge_server_layer(server_dir, "map", 'n', &mut mapsquares);
        loader::merge_server_layer(server_dir, "objspawn", 'o', &mut mapsquares);

        let dbtables = config.dbtables;
        let dbrows = config.dbrows;
        let db_index = DbTableIndex::build(&dbtables, &dbrows);

        Ok(Box::new(CacheStore {
            mapsquares,
            mapcrcs,
            objs: config.objs,
            invs: config.invs,
            varps: config.varps,
            varbits: config.varbits,
            dbrows,
            dbtables,
            db_index,
            enums: config.enums,
            flos: config.flos,
            hunts: config.hunts,
            idks: config.idks,
            locs: config.locs,
            mesanims: config.mesanims,
            npcs: config.npcs,
            params: config.params,
            seqs: config.seqs,
            spotanims: config.spotanims,
            structs: config.structs,
            varns: config.varns,
            varss: TypeProvider::default(),
            categories: config.categories,
            interfaces: IfTypeProvider {
                names: HashMap::new(),
                types: Vec::new(),
            },
            fonts,
            wordenc: WordEncProvider::empty(),
            songs: MidiProvider {
                names: HashMap::new(),
                midis: Box::new([]),
            },
            jingles: MidiProvider {
                names: HashMap::new(),
                midis: Box::new([]),
            },
            midi_ids: midi::load_midi_ids(),
            midi_tick_lengths: midi::load_midi_tick_lengths(cache),
            multimap: load_csv_zones(&Path::new(CONTENT_DIR).join("maps/multiway.csv")),
            freemap: load_csv_zones(&Path::new(CONTENT_DIR).join("maps/free2play.csv")),
            huffman,
        }))
    }
}
