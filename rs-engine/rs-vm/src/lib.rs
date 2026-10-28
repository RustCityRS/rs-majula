pub mod engine;
pub mod iterators;
pub mod macros;
pub mod npc_uid;
pub mod obj_serial;
pub mod ops;
pub mod player_uid;
pub mod pointer;
pub mod register;
pub mod state;
pub mod subject;
pub mod trigger;
pub mod util;
pub mod vm;

pub use npc_uid::NpcUid;
pub use obj_serial::ObjSerial;
pub use player_uid::PlayerUid;

use crate::trigger::ServerTriggerType;
use thiserror::Error;

/// Unified error type for the script virtual machine.
///
/// `ScriptError` represents every failure mode that can occur during script
/// loading, compilation, and runtime execution. Variants fall into three
/// broad categories:
///
/// 1. **I/O and loading** -- errors that occur while reading script files from
///    disk or the cache (`Io`, `Load`).
/// 2. **Lookup failures** -- attempts to reference a game entity (player, NPC,
///    object, location, interface, etc.) or cache definition by an ID or name
///    that does not exist. Each entity kind has both an integer-keyed and a
///    string-keyed variant.
/// 3. **Runtime faults** -- problems detected during VM execution such as
///    stack overflow/underflow, unknown opcodes, instruction-count limits,
///    missing subjects, and general runtime errors (`Runtime`, `StackOverflow`,
///    `StackUnderflow`, `UnknownOpcode`, `InstructionLimit`, `NoSubject`,
///    `Client`).
///
/// All variants implement `Display` via the `thiserror` derive so they can be
/// logged and reported to players in debug builds.
#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Load error: {0}")]
    Load(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Stack overflow")]
    StackOverflow,

    #[error("Stack underflow")]
    StackUnderflow,

    #[error("Unknown opcode: {0}")]
    UnknownOpcode(u16),

    #[error("Script not found: {0}")]
    ScriptNotFound(i32),

    #[error("Script not found: {0}")]
    ScriptNotFoundName(String),

    #[error("Player not found: {0}")]
    PlayerNotFound(i32),

    #[error("Player not found: {0}")]
    PlayerNotFoundName(String),

    #[error("Boolean not found: {0}")]
    BooleanNotFound(i32),

    #[error("Boolean not found: {0}")]
    BooleanNotFoundName(String),

    #[error("Category not found: {0}")]
    CategoryNotFound(i32),

    #[error("Category not found: {0}")]
    CategoryNotFoundName(String),

    #[error("Inv not found: {0}")]
    InvNotFound(i32),

    #[error("Inv not found: {0}")]
    InvNotFoundName(String),

    #[error("Jingle not found: {0}")]
    JingleNotFound(i32),

    #[error("Jingle not found: {0}")]
    JingleNotFoundName(String),

    #[error("Song not found: {0}")]
    SongNotFound(i32),

    #[error("Song not found: {0}")]
    SongNotFoundName(String),

    #[error("Inv not transmitted: {0}")]
    InvNotTransmitted(String),

    #[error("Idk not found: {0}")]
    IdkNotFound(i32),

    #[error("Idk not found: {0}")]
    IdkNotFoundName(String),

    #[error("Seq not found: {0}")]
    SeqNotFound(i32),

    #[error("Seq not found: {0}")]
    SeqNotFoundName(String),

    #[error("Dbrow not found: {0}")]
    DbRowNotFound(i32),

    #[error("Dbtable not found: {0}")]
    DbTableNotFound(i32),

    #[error("Font not found: {0}")]
    FontNotFound(i32),

    #[error("Mesanim not found: {0}")]
    MesanimNotFound(i32),

    #[error("Mesanim not found: {0}")]
    MesanimNotFoundName(String),

    #[error("Obj not found: {0}")]
    ObjNotFound(i32),

    #[error("Obj not found: {0}")]
    ObjNotFoundName(String),

    #[error("Enum not found: {0}")]
    EnumNotFound(i32),

    #[error("Enum not found: {0}")]
    EnumNotFoundName(String),

    #[error("Param not found: {0}")]
    ParamNotFound(i32),

    #[error("Param not found: {0}")]
    ParamNotFoundName(String),

    #[error("Loc not found: {0}")]
    LocNotFound(i32),

    #[error("Loc not found: {0}")]
    LocNotFoundName(String),

    #[error("Interface not found: {0}")]
    InterfaceNotFound(i32),

    #[error("Interface not found: {0}")]
    InterfaceNotFoundName(String),

    #[error("Spotanim not found: {0}")]
    SpotanimNotFound(i32),

    #[error("Spotanim not found: {0}")]
    SpotanimNotFoundName(String),

    #[error("Struct not found: {0}")]
    StructNotFound(i32),

    #[error("Struct not found: {0}")]
    StructNotFoundName(String),

    #[error("Varp not found: {0}")]
    VarpNotFound(i32),

    #[error("Varp not found: {0}")]
    VarpNotFoundName(String),

    #[error("Npc not found: {0}")]
    NpcNotFound(i32),

    #[error("Npc not found: {0}")]
    NpcNotFoundName(String),

    #[error("Stat not found: {0}")]
    StatNotFound(i32),

    #[error("Stat not found: {0}")]
    StatNotFoundName(String),

    #[error("Npcstat not found: {0}")]
    NpcstatNotFound(i32),

    #[error("Npcstat not found: {0}")]
    NpcstatNotFoundName(String),

    #[error("Instruction limit exceeded")]
    InstructionLimit,

    #[error("Trigger not found: {0:?}")]
    TriggerNotFound(ServerTriggerType),

    #[error("Missing subject")]
    NoSubject,

    #[error("Client error: {0}")]
    Client(String),
}

/// A convenience type alias for `std::result::Result<T, ScriptError>`.
///
/// Used throughout the VM crate so that functions can return `Result<T>`
/// without repeating the error type. Any function that can fail during
/// script loading or execution should return this type.
pub type Result<T> = std::result::Result<T, ScriptError>;
