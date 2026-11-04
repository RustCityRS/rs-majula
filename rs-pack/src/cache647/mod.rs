pub mod loader;
pub mod provider;
pub mod script;

pub mod anim_frame;
pub mod dbrow;
pub mod dbtable;
pub mod enum_type;
pub mod flo;
pub mod flu;
pub mod font;
pub mod hunt;
pub mod idk_type;
pub mod inv;
pub mod loc;
pub mod mesanim;
pub mod npc;
pub mod obj;
pub mod param_type;
pub mod seq;
pub mod spotanim;
pub mod struct_type;
pub mod varbit;
pub mod varn_type;
pub mod varp_type;

pub use provider::{CacheType, ParamValue, TypeProvider};

pub use dbrow::{DbRowColumn, DbRowType};
pub use dbtable::{DbColumn, DbTableType, DbValue};
pub use enum_type::{EnumType, EnumValue};
pub use flo::FloType;
pub use flu::FluType;
pub use font::FontType;
pub use hunt::HuntType;
pub use idk_type::IdkType;
pub use inv::InvType;
pub use loc::LocType;
pub use mesanim::MesAnimType;
pub use npc::NpcType;
pub use obj::{ObjContext, ObjType};
pub use param_type::ParamType;
pub use script::{Script, ScriptProvider};
pub use seq::SeqType;
pub use spotanim::SpotAnimType;
pub use struct_type::{StructType, decode_params};
pub use varbit::VarBitType;
pub use varn_type::VarnType;
pub use varp_type::VarpType;

pub use crate::types647::ScriptVarType;

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
