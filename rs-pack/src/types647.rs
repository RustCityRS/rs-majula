use num_enum::TryFromPrimitive;

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum ScriptVarType {
    Int = 105,       // i
    AutoInt = 255,   // ÿ
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
            "player" => Self::Player,
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

// --- npc behavior ---

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

// --- interface ---

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

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum Font {
    P11 = 0,
    P12 = 1,
    B12 = 2,
    Q8 = 3,
}

// --- hunt visibility ---

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, TryFromPrimitive)]
pub enum HuntCheckVis {
    Off = 0,
    LineOfSight = 1,
    LineOfWalk = 2,
}

// --- stats ---

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
    Hitpoints = 0,
    Attack = 1,
    Strength = 2,
    Defence = 3,
    Magic = 4,
    Ranged = 5,
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
