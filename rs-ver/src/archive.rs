#[cfg(all(since_244, before_647))]
pub const JAG: u8 = 0;
#[cfg(all(since_244, before_647))]
pub const MIDI: u8 = 3;
#[cfg(since_244)]
pub const ANIMS: u8 = if cfg!(since_647) { 0 } else { 2 };
#[cfg(since_647)]
pub const BASES: u8 = 1;
#[cfg(since_647)]
pub const CONFIG: u8 = 2;
#[cfg(since_647)]
pub const INTERFACES: u8 = 3;
#[cfg(since_647)]
pub const SYNTH: u8 = 4;
#[cfg(since_244)]
pub const MAPS: u8 = if cfg!(since_647) { 5 } else { 4 };
#[cfg(since_647)]
pub const SONGS: u8 = 6;
#[cfg(since_244)]
pub const MODELS: u8 = if cfg!(since_647) { 7 } else { 1 };
#[cfg(since_647)]
pub const SPRITES: u8 = 8;
#[cfg(since_647)]
pub const TEXTURES: u8 = 9;
#[cfg(since_647)]
pub const BINARY: u8 = 10;
#[cfg(since_647)]
pub const JINGLES: u8 = 11;
#[cfg(since_647)]
pub const CLIENTSCRIPT: u8 = 12;
#[cfg(since_647)]
pub const FONTMETRICS: u8 = 13;
#[cfg(since_647)]
pub const VORBIS: u8 = 14;
#[cfg(since_647)]
pub const INSTRUMENTS: u8 = 15;
#[cfg(since_647)]
pub const CONFIG_LOC: u8 = 16;
#[cfg(since_647)]
pub const CONFIG_ENUM: u8 = 17;
#[cfg(since_647)]
pub const CONFIG_NPC: u8 = 18;
#[cfg(since_647)]
pub const CONFIG_OBJ: u8 = 19;
#[cfg(since_647)]
pub const CONFIG_SEQ: u8 = 20;
#[cfg(since_647)]
pub const CONFIG_SPOTANIM: u8 = 21;
#[cfg(since_647)]
pub const CONFIG_VARBIT: u8 = 22;
#[cfg(since_647)]
pub const WORLDMAP: u8 = 23;
#[cfg(since_647)]
pub const QUICKCHAT: u8 = 24;
#[cfg(since_647)]
pub const QUICKCHAT_GLOBAL: u8 = 25;
#[cfg(since_647)]
pub const MATERIALS: u8 = 26;
#[cfg(since_647)]
pub const PARTICLES: u8 = 27;
#[cfg(since_647)]
pub const DEFAULTS: u8 = 28;
#[cfg(since_647)]
pub const BILLBOARDS: u8 = 29;
#[cfg(since_647)]
pub const DLLS: u8 = 30;
#[cfg(since_647)]
pub const SHADERS: u8 = 31;
#[cfg(since_647)]
pub const LOADING_SPRITES: u8 = 32;
#[cfg(since_647)]
pub const LOADING_SCREENS: u8 = 33;
#[cfg(since_647)]
pub const LOADING_SPRITES_RAW: u8 = 34;
#[cfg(since_647)]
pub const WORLDMAP_LOCS: u8 = 36;
#[cfg(since_647)]
pub const REFERENCE_TABLES: u8 = 255;

#[cfg(since_647)]
pub const CONFIG_GROUP_FLU: u32 = 1;
#[cfg(since_647)]
pub const CONFIG_GROUP_HUNT: u32 = 2;
#[cfg(since_647)]
pub const CONFIG_GROUP_IDK: u32 = 3;
#[cfg(since_647)]
pub const CONFIG_GROUP_FLO: u32 = 4;
#[cfg(since_647)]
pub const CONFIG_GROUP_INV: u32 = 5;
#[cfg(since_647)]
pub const CONFIG_GROUP_MESANIM: u32 = 7;
#[cfg(since_647)]
pub const CONFIG_GROUP_PARAM: u32 = 11;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARCSTR: u32 = 15;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARP: u32 = 16;
#[cfg(since_647)]
pub const CONFIG_GROUP_AREA: u32 = 18;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARC: u32 = 19;
#[cfg(since_647)]
pub const CONFIG_GROUP_VAROBJ: u32 = 20;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARPSTR: u32 = 21;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARS: u32 = 22;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARSSTR: u32 = 23;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARN: u32 = 24;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARNBIT: u32 = 25;
#[cfg(since_647)]
pub const CONFIG_GROUP_STRUCT: u32 = 26;
#[cfg(since_647)]
pub const CONFIG_GROUP_LIGHT: u32 = 31;
#[cfg(since_647)]
pub const CONFIG_GROUP_BAS: u32 = 32;
#[cfg(since_647)]
pub const CONFIG_GROUP_CURSOR: u32 = 33;
#[cfg(since_647)]
pub const CONFIG_GROUP_MSI: u32 = 34;
#[cfg(since_647)]
pub const CONFIG_GROUP_QUEST: u32 = 35;
#[cfg(since_647)]
pub const CONFIG_GROUP_MEL: u32 = 36;
#[cfg(since_647)]
pub const CONFIG_GROUP_DBTABLE: u32 = 40;
#[cfg(since_647)]
pub const CONFIG_GROUP_DBROW: u32 = 41;
#[cfg(since_647)]
pub const CONFIG_GROUP_CONTROLLER: u32 = 42;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARCON: u32 = 43;
#[cfg(since_647)]
pub const CONFIG_GROUP_CONFIG44: u32 = 44;
#[cfg(since_647)]
pub const CONFIG_GROUP_HITMARK: u32 = 46;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARCLAN: u32 = 47;
#[cfg(since_647)]
pub const CONFIG_GROUP_ITEMCODE: u32 = 48;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARCLANSETTING: u32 = 54;
