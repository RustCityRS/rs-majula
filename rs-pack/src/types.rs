use num_enum::TryFromPrimitive;
use rustc_hash::FxHashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// --- Example: ('m', 50, 50) ---
pub type MapSquare = (char, u8, u8);
// --- Packed zone keys ---
pub type MapSquareCsv = HashSet<u32>;
// --- Mapsquare raw bytes ---
pub type MapSquares = FxHashMap<MapSquare, Arc<[u8]>>;
// --- Mapsquare crc ---
pub type MapSquareCrcs = HashMap<MapSquare, i32>;

// --- Ondemand blobs ---
pub type OndemandBlobs = Vec<Vec<Box<[u8]>>>;

// --- bone type ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum BoneType {
    Translate = 0,
    Rotate = 1,
    Scale = 2,
    Alpha = 3,
    Origin = 5,
}

impl BoneType {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "translate" => Self::Translate,
            "rotate" => Self::Rotate,
            "scale" => Self::Scale,
            "alpha" => Self::Alpha,
            "origin" => Self::Origin,
            _ => panic!("unknown bone type: {s}"),
        }
    }

    pub const fn config_str(self) -> &'static str {
        match self {
            Self::Translate => "translate",
            Self::Rotate => "rotate",
            Self::Scale => "scale",
            Self::Alpha => "alpha",
            Self::Origin => "origin",
        }
    }
}

// --- loc ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum LocAngle {
    West = 0,
    North = 1,
    East = 2,
    South = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum LocLayer {
    Wall = 0,
    WallDecor = 1,
    Ground = 2,
    GroundDecor = 3,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum LocShape {
    WallStraight = 0,
    WallDiagonalCorner = 1,
    WallL = 2,
    WallSquareCorner = 3,
    WallDecorStraightNoOffset = 4,
    WallDecorStraightOffset = 5,
    WallDecorDiagonalOffset = 6,
    WallDecorDiagonalNoOffset = 7,
    WallDecorDiagonalBoth = 8,
    WallDiagonal = 9,
    CentrepieceStraight = 10,
    CentrepieceDiagonal = 11,
    RoofStraight = 12,
    RoofDiagonalWithRoofEdge = 13,
    RoofDiagonal = 14,
    RoofLConcave = 15,
    RoofLConvex = 16,
    RoofFlat = 17,
    RoofEdgeStraight = 18,
    RoofEdgeDiagonalCorner = 19,
    RoofEdgeL = 20,
    RoofEdgeSquareCorner = 21,
    GroundDecor = 22,
}

impl LocShape {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::WallStraight => "_1",
            Self::WallDiagonalCorner => "_2",
            Self::WallL => "_3",
            Self::WallSquareCorner => "_4",
            Self::WallDecorStraightNoOffset => "_q",
            Self::WallDecorStraightOffset => "_w",
            Self::WallDecorDiagonalOffset => "_r",
            Self::WallDecorDiagonalNoOffset => "_e",
            Self::WallDecorDiagonalBoth => "_t",
            Self::WallDiagonal => "_5",
            Self::CentrepieceStraight => "_8",
            Self::CentrepieceDiagonal => "_9",
            Self::RoofStraight => "_a",
            Self::RoofDiagonalWithRoofEdge => "_s",
            Self::RoofDiagonal => "_d",
            Self::RoofLConcave => "_f",
            Self::RoofLConvex => "_g",
            Self::RoofFlat => "_h",
            Self::RoofEdgeStraight => "_z",
            Self::RoofEdgeDiagonalCorner => "_x",
            Self::RoofEdgeL => "_c",
            Self::RoofEdgeSquareCorner => "_v",
            Self::GroundDecor => "_0",
        }
    }

    pub const fn layer(self) -> LocLayer {
        match self {
            Self::WallStraight
            | Self::WallDiagonalCorner
            | Self::WallL
            | Self::WallSquareCorner => LocLayer::Wall,
            Self::WallDecorStraightNoOffset
            | Self::WallDecorStraightOffset
            | Self::WallDecorDiagonalOffset
            | Self::WallDecorDiagonalNoOffset
            | Self::WallDecorDiagonalBoth => LocLayer::WallDecor,
            Self::WallDiagonal
            | Self::CentrepieceStraight
            | Self::CentrepieceDiagonal
            | Self::RoofStraight
            | Self::RoofDiagonalWithRoofEdge
            | Self::RoofDiagonal
            | Self::RoofLConcave
            | Self::RoofLConvex
            | Self::RoofFlat
            | Self::RoofEdgeStraight
            | Self::RoofEdgeDiagonalCorner
            | Self::RoofEdgeL
            | Self::RoofEdgeSquareCorner => LocLayer::Ground,
            Self::GroundDecor => LocLayer::GroundDecor,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum ForceApproach {
    None = 0,
    North = 0b1110,
    East = 0b1101,
    South = 0b1011,
    West = 0b0111,
}

impl ForceApproach {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "north" => Self::North,
            "east" => Self::East,
            "south" => Self::South,
            "west" => Self::West,
            _ => panic!("Invalid forceapproach value: {s}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ParamValue {
    Int(i32),
    String(Box<str>),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum ScriptVarType {
    Int = 105,       // i
    AutoInt = 255,   // ÿ - virtual type used for enum keys
    String = 115,    // s
    Enum = 103,      // g
    Obj = 111,       // o
    Loc = 108,       // l
    Component = 73,  // I
    NamedObj = 79,   // O
    Struct = 74,     // J
    Boolean = 49,    // 1
    Coord = 99,      // c
    Category = 121,  // y
    Spotanim = 116,  // t
    Npc = 110,       // n
    Inv = 118,       // v
    Synth = 80,      // P
    Seq = 65,        // A
    Stat = 83,       // S
    Varp = 86,       // V
    PlayerUid = 112, // p
    NpcUid = 78,     // N
    Interface = 97,  // a
    NpcStat = 254,   // þ
    Idkit = 75,      // K
    DbRow = 208,     // Ð
}

impl ScriptVarType {
    pub fn is_string(self) -> bool {
        matches!(self, Self::String)
    }

    pub fn to_char(self) -> char {
        (self as u8) as char
    }

    pub fn from_type_name(name: &str) -> Self {
        match name {
            "int" => Self::Int,
            "autoint" => Self::AutoInt,
            "string" => Self::String,
            "coord" => Self::Coord,
            "obj" | "namedobj" => Self::Obj,
            "npc" => Self::Npc,
            "loc" => Self::Loc,
            "component" | "interface" => Self::Component,
            "boolean" => Self::Boolean,
            "enum" => Self::Enum,
            "struct" => Self::Struct,
            "stat" | "npc_stat" => Self::Stat,
            "seq" => Self::Seq,
            "synth" => Self::Synth,
            "inv" => Self::Inv,
            "spotanim" => Self::Spotanim,
            "varp" => Self::Varp,
            "category" => Self::Category,
            "idkit" => Self::Idkit,
            "player_uid" => Self::PlayerUid,
            "npc_uid" => Self::NpcUid,
            "dbrow" => Self::DbRow,
            _ => panic!("Unknown script var type: '{name}'"),
        }
    }

    pub fn type_name(self) -> &'static str {
        match self {
            Self::Int => "int",
            Self::AutoInt => "autoint",
            Self::String => "string",
            Self::Coord => "coord",
            Self::Obj | Self::NamedObj => "obj",
            Self::Npc => "npc",
            Self::Loc => "loc",
            Self::Component => "component",
            Self::Boolean => "boolean",
            Self::Enum => "enum",
            Self::Struct => "struct",
            Self::Stat | Self::NpcStat => "stat",
            Self::Seq => "seq",
            Self::Synth => "synth",
            Self::Inv => "inv",
            Self::Spotanim => "spotanim",
            Self::Varp => "varp",
            Self::Category => "category",
            Self::Idkit => "idkit",
            Self::PlayerUid => "player_uid",
            Self::NpcUid => "npc_uid",
            Self::DbRow => "dbrow",
            Self::Interface => "interface",
        }
    }
}

// --- inventory / variable scopes ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum InvScope {
    Temp = 0,
    Perm = 1,
    Shared = 2,
}

impl InvScope {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "temp" => Self::Temp,
            "perm" => Self::Perm,
            "shared" => Self::Shared,
            _ => panic!("Invalid scope value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum VarPlayerScope {
    Temp = 0,
    Perm = 1,
}

impl VarPlayerScope {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "temp" => Self::Temp,
            "perm" => Self::Perm,
            _ => panic!("Invalid scope value: {s}"),
        }
    }
}

// --- npc behavior ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum MoveRestrict {
    Normal = 0,
    Blocked = 1,
    BlockedNormal = 2,
    Indoors = 3,
    Outdoors = 4,
    NoMove = 5,
    Passthru = 6,
    Player = 7,
}

impl MoveRestrict {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "normal" => Self::Normal,
            "blocked" => Self::Blocked,
            "blocked+normal" => Self::BlockedNormal,
            "indoors" => Self::Indoors,
            "outdoors" => Self::Outdoors,
            "nomove" => Self::NoMove,
            "passthru" => Self::Passthru,
            _ => panic!("Invalid moverestrict value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum BlockWalk {
    None = 0,
    All = 1,
    Npc = 2,
}

impl BlockWalk {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "all" => Self::All,
            "npc" => Self::Npc,
            _ => panic!("Invalid blockwalk value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum NpcMode {
    None = 0,
    Wander = 1,
    Patrol = 2,
    PlayerEscape = 3,
    PlayerFollow = 4,
    PlayerFace = 5,
    PlayerFaceClose = 6,
    OpPlayer1 = 7,
    OpPlayer2 = 8,
    OpPlayer3 = 9,
    OpPlayer4 = 10,
    OpPlayer5 = 11,
    ApPlayer1 = 12,
    ApPlayer2 = 13,
    ApPlayer3 = 14,
    ApPlayer4 = 15,
    ApPlayer5 = 16,
    OpLoc1 = 17,
    OpLoc2 = 18,
    OpLoc3 = 19,
    OpLoc4 = 20,
    OpLoc5 = 21,
    ApLoc1 = 22,
    ApLoc2 = 23,
    ApLoc3 = 24,
    ApLoc4 = 25,
    ApLoc5 = 26,
    OpObj1 = 27,
    OpObj2 = 28,
    OpObj3 = 29,
    OpObj4 = 30,
    OpObj5 = 31,
    ApObj1 = 32,
    ApObj2 = 33,
    ApObj3 = 34,
    ApObj4 = 35,
    ApObj5 = 36,
    OpNpc1 = 37,
    OpNpc2 = 38,
    OpNpc3 = 39,
    OpNpc4 = 40,
    OpNpc5 = 41,
    ApNpc1 = 42,
    ApNpc2 = 43,
    ApNpc3 = 44,
    ApNpc4 = 45,
    ApNpc5 = 46,
    Queue1 = 47,
    Queue2 = 48,
    Queue3 = 49,
    Queue4 = 50,
    Queue5 = 51,
    Queue6 = 52,
    Queue7 = 53,
    Queue8 = 54,
    Queue9 = 55,
    Queue10 = 56,
    Queue11 = 57,
    Queue12 = 58,
    Queue13 = 59,
    Queue14 = 60,
    Queue15 = 61,
    Queue16 = 62,
    Queue17 = 63,
    Queue18 = 64,
    Queue19 = 65,
    Queue20 = 66,
}

impl NpcMode {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "none" => Self::None,
            "wander" => Self::Wander,
            "patrol" => Self::Patrol,
            "playerescape" => Self::PlayerEscape,
            "playerfollow" => Self::PlayerFollow,
            "playerface" => Self::PlayerFace,
            "playerfaceclose" => Self::PlayerFaceClose,
            "opplayer1" => Self::OpPlayer1,
            "opplayer2" => Self::OpPlayer2,
            "opplayer3" => Self::OpPlayer3,
            "opplayer4" => Self::OpPlayer4,
            "opplayer5" => Self::OpPlayer5,
            "applayer1" => Self::ApPlayer1,
            "applayer2" => Self::ApPlayer2,
            "applayer3" => Self::ApPlayer3,
            "applayer4" => Self::ApPlayer4,
            "applayer5" => Self::ApPlayer5,
            "oploc1" => Self::OpLoc1,
            "oploc2" => Self::OpLoc2,
            "oploc3" => Self::OpLoc3,
            "oploc4" => Self::OpLoc4,
            "oploc5" => Self::OpLoc5,
            "aploc1" => Self::ApLoc1,
            "aploc2" => Self::ApLoc2,
            "aploc3" => Self::ApLoc3,
            "aploc4" => Self::ApLoc4,
            "aploc5" => Self::ApLoc5,
            "opobj1" => Self::OpObj1,
            "opobj2" => Self::OpObj2,
            "opobj3" => Self::OpObj3,
            "opobj4" => Self::OpObj4,
            "opobj5" => Self::OpObj5,
            "apobj1" => Self::ApObj1,
            "apobj2" => Self::ApObj2,
            "apobj3" => Self::ApObj3,
            "apobj4" => Self::ApObj4,
            "apobj5" => Self::ApObj5,
            "opnpc1" => Self::OpNpc1,
            "opnpc2" => Self::OpNpc2,
            "opnpc3" => Self::OpNpc3,
            "opnpc4" => Self::OpNpc4,
            "opnpc5" => Self::OpNpc5,
            "apnpc1" => Self::ApNpc1,
            "apnpc2" => Self::ApNpc2,
            "apnpc3" => Self::ApNpc3,
            "apnpc4" => Self::ApNpc4,
            "apnpc5" => Self::ApNpc5,
            "queue1" => Self::Queue1,
            "queue2" => Self::Queue2,
            "queue3" => Self::Queue3,
            "queue4" => Self::Queue4,
            "queue5" => Self::Queue5,
            "queue6" => Self::Queue6,
            "queue7" => Self::Queue7,
            "queue8" => Self::Queue8,
            "queue9" => Self::Queue9,
            "queue10" => Self::Queue10,
            "queue11" => Self::Queue11,
            "queue12" => Self::Queue12,
            "queue13" => Self::Queue13,
            "queue14" => Self::Queue14,
            "queue15" => Self::Queue15,
            "queue16" => Self::Queue16,
            "queue17" => Self::Queue17,
            "queue18" => Self::Queue18,
            "queue19" => Self::Queue19,
            "queue20" => Self::Queue20,
            _ => panic!("Unknown npc mode: {s}"),
        }
    }
}

// --- hunt ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntModeType {
    Off = 0,
    Player = 1,
    Npc = 2,
    Obj = 3,
    Scenery = 4,
}

impl HuntModeType {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "player" => Self::Player,
            "npc" => Self::Npc,
            "obj" => Self::Obj,
            "scenery" => Self::Scenery,
            _ => panic!("Invalid hunt type value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntCheckVis {
    Off = 0,
    LineOfSight = 1,
    LineOfWalk = 2,
}

impl HuntCheckVis {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "lineofsight" => Self::LineOfSight,
            "lineofwalk" => Self::LineOfWalk,
            _ => panic!("Invalid check_vis value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntCheckNotTooStrong {
    Off = 0,
    OutsideWilderness = 1,
}

impl HuntCheckNotTooStrong {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "outside_wilderness" => Self::OutsideWilderness,
            _ => panic!("Invalid check_nottoostrong value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntNobodyNear {
    KeepHunting = 0,
    PauseHunt = 1,
}

impl HuntNobodyNear {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "keephunting" => Self::KeepHunting,
            "pausehunt" => Self::PauseHunt,
            _ => panic!("Invalid nobodynear value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntCheckNotBusy {
    Off = 0,
    On = 1,
}

impl HuntCheckNotBusy {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "on" => Self::On,
            _ => panic!("Invalid check_notbusy value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntFindKeepHunting {
    Off = 0,
    On = 1,
}

impl HuntFindKeepHunting {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "on" => Self::On,
            _ => panic!("Invalid find_keephunting value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntCheckAfk {
    On = 0,
    Off = 1,
}

impl HuntCheckAfk {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "off" => Self::Off,
            "on" => Self::On,
            _ => panic!("Invalid check_afk value: {s}"),
        }
    }
}

// --- obj ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum WearPos {
    Hat = 0,
    Back = 1,
    Front = 2,
    RightHand = 3,
    Torso = 4,
    LeftHand = 5,
    Arms = 6,
    Legs = 7,
    Head = 8,
    Hands = 9,
    Feet = 10,
    Jaw = 11,
    Ring = 12,
    Quiver = 13,
}

impl WearPos {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "hat" => Self::Hat,
            "back" => Self::Back,
            "front" => Self::Front,
            "righthand" => Self::RightHand,
            "torso" => Self::Torso,
            "lefthand" => Self::LeftHand,
            "arms" => Self::Arms,
            "legs" => Self::Legs,
            "head" => Self::Head,
            "hands" => Self::Hands,
            "feet" => Self::Feet,
            "jaw" => Self::Jaw,
            "ring" => Self::Ring,
            "quiver" => Self::Quiver,
            _ => panic!("Invalid wearpos value: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum DummyItem {
    None = 0,
    GraphicOnly = 1,
    InvOnly = 2,
}

impl DummyItem {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "graphic_only" => Self::GraphicOnly,
            "inv_only" => Self::InvOnly,
            _ => panic!("Invalid dummyitem value: {s}"),
        }
    }
}

// --- idk ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum BodyType {
    ManHair = 0,
    ManJaw = 1,
    ManTorso = 2,
    ManArms = 3,
    ManHands = 4,
    ManLegs = 5,
    ManFeet = 6,
    WomanHair = 7,
    WomanJaw = 8,
    WomanTorso = 9,
    WomanArms = 10,
    WomanHands = 11,
    WomanLegs = 12,
    WomanFeet = 13,
}

impl BodyType {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "man_hair" => Self::ManHair,
            "man_jaw" => Self::ManJaw,
            "man_torso" => Self::ManTorso,
            "man_arms" => Self::ManArms,
            "man_hands" => Self::ManHands,
            "man_legs" => Self::ManLegs,
            "man_feet" => Self::ManFeet,
            "woman_hair" => Self::WomanHair,
            "woman_jaw" => Self::WomanJaw,
            "woman_torso" => Self::WomanTorso,
            "woman_arms" => Self::WomanArms,
            "woman_hands" => Self::WomanHands,
            "woman_legs" => Self::WomanLegs,
            "woman_feet" => Self::WomanFeet,
            _ => panic!("Invalid idk type value: {s}"),
        }
    }
}

// --- interface ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum IfComponentType {
    Layer = 0,
    Inv = 2,
    Rect = 3,
    Text = 4,
    Graphic = 5,
    Model = 6,
    InvText = 7,
}

impl IfComponentType {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "layer" | "overlay" => Self::Layer,
            "inv" => Self::Inv,
            "rect" => Self::Rect,
            "text" => Self::Text,
            "graphic" => Self::Graphic,
            "model" => Self::Model,
            "invtext" => Self::InvText,
            _ => panic!("Unknown component type: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum IfButtonType {
    None = 0,
    Normal = 1,
    Target = 2,
    Close = 3,
    Toggle = 4,
    Select = 5,
    Pause = 6,
}

impl IfButtonType {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "" => Self::None,
            "normal" => Self::Normal,
            "target" => Self::Target,
            "close" => Self::Close,
            "toggle" => Self::Toggle,
            "select" => Self::Select,
            "pause" => Self::Pause,
            _ => panic!("Unknown button type: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum Font {
    P11 = 0,
    P12 = 1,
    B12 = 2,
    Q8 = 3,
}

impl Font {
    pub const ALL: [Font; 4] = [Self::P11, Self::P12, Self::B12, Self::Q8];

    pub const fn name(self) -> &'static str {
        match self {
            Self::P11 => "p11",
            Self::P12 => "p12",
            Self::B12 => "b12",
            Self::Q8 => "q8",
        }
    }

    pub fn from_config_str(s: &str) -> Self {
        match s {
            "p11" => Self::P11,
            "p12" => Self::P12,
            "b12" => Self::B12,
            "q8" => Self::Q8,
            _ => panic!("Unknown font: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum PlayerStat {
    Attack = 0,
    Defence = 1,
    Strength = 2,
    Hitpoints = 3,
    Ranged = 4,
    Prayer = 5,
    Magic = 6,
    Cooking = 7,
    Woodcutting = 8,
    Fletching = 9,
    Fishing = 10,
    Firemaking = 11,
    Crafting = 12,
    Smithing = 13,
    Mining = 14,
    Herblore = 15,
    Agility = 16,
    Thieving = 17,
    Stat18 = 18,
    Stat19 = 19,
    Runecraft = 20,
}

impl PlayerStat {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "attack" => Self::Attack,
            "defence" => Self::Defence,
            "strength" => Self::Strength,
            "hitpoints" => Self::Hitpoints,
            "ranged" => Self::Ranged,
            "prayer" => Self::Prayer,
            "magic" => Self::Magic,
            "cooking" => Self::Cooking,
            "woodcutting" => Self::Woodcutting,
            "fletching" => Self::Fletching,
            "fishing" => Self::Fishing,
            "firemaking" => Self::Firemaking,
            "crafting" => Self::Crafting,
            "smithing" => Self::Smithing,
            "mining" => Self::Mining,
            "herblore" => Self::Herblore,
            "agility" => Self::Agility,
            "thieving" => Self::Thieving,
            "stat18" => Self::Stat18,
            "stat19" => Self::Stat19,
            "runecraft" => Self::Runecraft,
            _ => panic!("Unknown player stat name: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum NpcStat {
    Attack = 0,
    Defence = 1,
    Strength = 2,
    Hitpoints = 3,
    Ranged = 4,
    Magic = 5,
}

impl NpcStat {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "hitpoints" => Self::Hitpoints,
            "attack" => Self::Attack,
            "strength" => Self::Strength,
            "defence" => Self::Defence,
            "magic" => Self::Magic,
            "ranged" => Self::Ranged,
            _ => panic!("Unknown npc stat name: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum IfScriptOp {
    StatLevel = 1,
    StatBaseLevel = 2,
    StatXp = 3,
    InvCount = 4,
    PushVar = 5,
    StatXpRemaining = 6,
    Op7 = 7,
    Op8 = 8,
    Op9 = 9,
    InvContains = 10,
    RunEnergy = 11,
    RunWeight = 12,
    TestBit = 13,
}

impl IfScriptOp {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "stat_level" => Self::StatLevel,
            "stat_base_level" => Self::StatBaseLevel,
            "stat_xp" => Self::StatXp,
            "inv_count" => Self::InvCount,
            "pushvar" => Self::PushVar,
            "stat_xp_remaining" => Self::StatXpRemaining,
            "op7" => Self::Op7,
            "op8" => Self::Op8,
            "op9" => Self::Op9,
            "inv_contains" => Self::InvContains,
            "runenergy" => Self::RunEnergy,
            "runweight" => Self::RunWeight,
            "testbit" => Self::TestBit,
            _ => panic!("Unknown script op: {s}"),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum IfComparator {
    Eq = 1,
    Lt = 2,
    Gt = 3,
    Neq = 4,
}

impl IfComparator {
    pub fn from_config_str(s: &str) -> Self {
        match s {
            "eq" => Self::Eq,
            "lt" => Self::Lt,
            "gt" => Self::Gt,
            "neq" => Self::Neq,
            _ => panic!("Unknown comparator: {s}"),
        }
    }
}
