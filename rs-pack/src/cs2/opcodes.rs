//! CS2 opcode constants and metadata.
//!
//! Constants verified against the rev-647 deob client (Class47.java / FileSystem.java).

use crate::cs2::ir::Expr;

// ---------------------------------------------------------------------------
// Core control flow (verified from Class47.java execute_script)
// ---------------------------------------------------------------------------
pub const PUSH_CONSTANT_INT: u16 = 0;
pub const PUSH_VAR: u16 = 1;
pub const POP_VAR: u16 = 2;
pub const PUSH_CONSTANT_STRING: u16 = 3;
pub const BRANCH: u16 = 6;
pub const BRANCH_NOT: u16 = 7;
pub const BRANCH_EQUALS: u16 = 8;
pub const BRANCH_LESS_THAN: u16 = 9;
pub const BRANCH_GREATER_THAN: u16 = 10;
pub const RETURN: u16 = 21;
pub const PUSH_VARBIT: u16 = 25;
pub const POP_VARBIT: u16 = 27;
pub const BRANCH_LESS_THAN_OR_EQUALS: u16 = 31;
pub const BRANCH_GREATER_THAN_OR_EQUALS: u16 = 32;
pub const PUSH_INT_LOCAL: u16 = 33;
pub const POP_INT_LOCAL: u16 = 34;
pub const PUSH_STRING_LOCAL: u16 = 35;
pub const POP_STRING_LOCAL: u16 = 36;
pub const JOIN_STRING: u16 = 37;
pub const POP_INT_DISCARD: u16 = 38;
pub const POP_STRING_DISCARD: u16 = 39;
pub const GOSUB_WITH_PARAMS: u16 = 40;
pub const PUSH_VARC_INT: u16 = 42;
pub const POP_VARC_INT: u16 = 43;
pub const DEFINE_ARRAY: u16 = 44;
pub const PUSH_ARRAY_INT: u16 = 45;
pub const POP_ARRAY_INT: u16 = 46;
pub const PUSH_VARC_STRING_OLD: u16 = 47;
pub const POP_VARC_STRING_OLD: u16 = 48;
pub const SWITCH: u16 = 51;
pub const PUSH_CONSTANT_LONG: u16 = 54;
pub const POP_LONG_DISCARD: u16 = 55;
pub const PUSH_LONG_LOCAL: u16 = 66;
pub const POP_LONG_LOCAL: u16 = 67;
pub const BRANCH_NOT_LONG: u16 = 68;
pub const BRANCH_EQUALS_LONG: u16 = 69;
pub const BRANCH_LESS_THAN_LONG: u16 = 70;
pub const BRANCH_GREATER_THAN_LONG: u16 = 71;
pub const BRANCH_LESS_THAN_OR_EQUALS_LONG: u16 = 72;
pub const BRANCH_GREATER_THAN_OR_EQUALS_LONG: u16 = 73;
pub const BRANCH_IF_TRUE: u16 = 86;
pub const BRANCH_IF_FALSE: u16 = 87;

// ---------------------------------------------------------------------------
// Component creation / deletion
// ---------------------------------------------------------------------------
pub const CC_CREATE: u16 = 100;
pub const CC_DELETE: u16 = 101;
pub const CC_DELETEALL: u16 = 102;

// ---------------------------------------------------------------------------
// cc_set* (component setters on active component) — 150+
// ---------------------------------------------------------------------------
pub const CC_SETPOSITION: u16 = 150;
pub const CC_FIND: u16 = 200;
pub const IF_FIND: u16 = 201;

// cc_set opcodes (1000-series)
pub const CC_SETPOSITION2: u16 = 1000;
pub const CC_SETSIZE: u16 = 1001;
pub const CC_SETHIDE: u16 = 1003;
pub const CC_SETASPECT_RATIO: u16 = 1004;
pub const CC_SETNOCLICKTHROUGH: u16 = 1005;
pub const CC_SETSCROLLPOS: u16 = 1100;
pub const CC_SETCOLOUR: u16 = 1101;
pub const CC_SETFILL: u16 = 1102;
pub const CC_SETTRANS: u16 = 1103;
pub const CC_SETGRAPHIC: u16 = 1105;
pub const CC_SET2DANGLE: u16 = 1106;
pub const CC_SETTILING: u16 = 1107;
pub const CC_SETMODEL: u16 = 1108;
pub const CC_SETMODELANGLE: u16 = 1109;
pub const CC_SETMODELANIM: u16 = 1110;
pub const CC_SETMODELORTHOG: u16 = 1111;
pub const CC_SETTEXT: u16 = 1112;
pub const CC_SETTEXTFONT: u16 = 1113;
pub const CC_SETTEXTALIGN: u16 = 1114;
pub const CC_SETTEXTSHADOW: u16 = 1115;
pub const CC_SETOUTLINE: u16 = 1116;
pub const CC_SETGRAPHICSHADOW: u16 = 1117;
pub const CC_SETVFLIP: u16 = 1118;
pub const CC_SETHFLIP: u16 = 1119;
pub const CC_SETSCROLLSIZE: u16 = 1120;
pub const CC_SETOBJECT: u16 = 1200;
pub const CC_SETOBJECT_NONUM: u16 = 1205;
pub const CC_SETOP: u16 = 1300;
pub const CC_SETDRAGGABLE: u16 = 1301;
pub const CC_SETDRAGDEADZONE: u16 = 1302;
pub const CC_SETDRAGDEADTIME: u16 = 1303;
pub const CC_SETOPBASE: u16 = 1305;
pub const CC_SETTARGETVERB: u16 = 1306;
pub const CC_CLEAROPS: u16 = 1307;
pub const CC_SETOPKEY: u16 = 1350;
pub const CC_SETOPTKEY: u16 = 1351;

// cc_seton* (event handlers)
pub const CC_SETONCLICK: u16 = 1400;
pub const CC_SETONHOLD: u16 = 1401;
pub const CC_SETONRELEASE: u16 = 1402;
pub const CC_SETONMOUSEOVER: u16 = 1403;
pub const CC_SETONMOUSELEAVE: u16 = 1404;
pub const CC_SETONDRAG: u16 = 1405;
pub const CC_SETONTARGETLEAVE: u16 = 1406;
pub const CC_SETONVARTRANSMIT: u16 = 1407;
pub const CC_SETONTIMER: u16 = 1408;
pub const CC_SETONOP: u16 = 1409;
pub const CC_SETONDRAGCOMPLETE: u16 = 1410;
pub const CC_SETONCLICKREPEAT: u16 = 1411;
pub const CC_SETONMOUSEREPEAT: u16 = 1412;
pub const CC_SETONINVTRANSMIT: u16 = 1414;
pub const CC_SETONSTATTRANSMIT: u16 = 1415;
pub const CC_SETONTARGETENTER: u16 = 1416;
pub const CC_SETONSCROLLWHEEL: u16 = 1417;
pub const CC_SETONCHATTRANSMIT: u16 = 1418;
pub const CC_SETONKEY: u16 = 1419;

// cc_get* (component getters)
pub const CC_GETX: u16 = 1500;
pub const CC_GETY: u16 = 1501;
pub const CC_GETWIDTH: u16 = 1502;
pub const CC_GETHEIGHT: u16 = 1503;
pub const CC_GETHIDE: u16 = 1504;
pub const CC_GETLAYER: u16 = 1505;
pub const CC_GETSCROLLX: u16 = 1600;
pub const CC_GETSCROLLY: u16 = 1601;
pub const CC_GETTEXT: u16 = 1602;
pub const CC_GETSCROLLWIDTH: u16 = 1603;
pub const CC_GETSCROLLHEIGHT: u16 = 1604;
pub const CC_GETMODELZOOM: u16 = 1605;
pub const CC_GETMODELANGLE_X: u16 = 1606;
pub const CC_GETMODELANGLE_Z: u16 = 1607;
pub const CC_GETMODELANGLE_Y: u16 = 1608;
pub const CC_GETTRANS: u16 = 1609;
pub const CC_GETINVOBJECT: u16 = 1700;
pub const CC_GETINVCOUNT: u16 = 1701;
pub const CC_GETID: u16 = 1702;

// if_set* (interface component setters by hash)
pub const IF_SETPOSITION: u16 = 2000;
pub const IF_SETSIZE: u16 = 2001;
pub const IF_SETHIDE: u16 = 2003;
pub const IF_SETSCROLLPOS: u16 = 2100;
pub const IF_SETCOLOUR: u16 = 2101;
pub const IF_SETFILL: u16 = 2102;
pub const IF_SETTRANS: u16 = 2103;
pub const IF_SETGRAPHIC: u16 = 2105;
pub const IF_SETMODEL: u16 = 2108;
pub const IF_SETMODELANGLE: u16 = 2109;
pub const IF_SETMODELANIM: u16 = 2110;
pub const IF_SETTEXT: u16 = 2112;
pub const IF_SETTEXTFONT: u16 = 2113;
pub const IF_SETTEXTALIGN: u16 = 2114;
pub const IF_SETOBJECT: u16 = 2200;
pub const IF_SETOBJECT_NONUM: u16 = 2205;
pub const IF_SETOP: u16 = 2300;
pub const IF_SETDRAGGABLE: u16 = 2301;
pub const IF_SETOPBASE: u16 = 2305;
pub const IF_SETTARGETVERB: u16 = 2306;
pub const IF_CLEAROPS: u16 = 2307;
pub const IF_SETOPKEY: u16 = 2350;

// if_seton*
pub const IF_SETONCLICK: u16 = 2400;
pub const IF_SETONHOLD: u16 = 2401;
pub const IF_SETONRELEASE: u16 = 2402;
pub const IF_SETONMOUSEOVER: u16 = 2403;
pub const IF_SETONMOUSELEAVE: u16 = 2404;
pub const IF_SETONDRAG: u16 = 2405;
pub const IF_SETONVARTRANSMIT: u16 = 2407;
pub const IF_SETONTIMER: u16 = 2408;
pub const IF_SETONOP: u16 = 2409;
pub const IF_SETONDRAGCOMPLETE: u16 = 2410;
pub const IF_SETONCLICKREPEAT: u16 = 2411;
pub const IF_SETONMOUSEREPEAT: u16 = 2412;
pub const IF_SETONINVTRANSMIT: u16 = 2414;
pub const IF_SETONSTATTRANSMIT: u16 = 2415;
pub const IF_SETONTARGETENTER: u16 = 2416;
pub const IF_SETONSCROLLWHEEL: u16 = 2417;
pub const IF_SETONCHATTRANSMIT: u16 = 2418;
pub const IF_SETONKEY: u16 = 2419;

// if_get*
pub const IF_GETX: u16 = 2500;
pub const IF_GETY: u16 = 2501;
pub const IF_GETWIDTH: u16 = 2502;
pub const IF_GETHEIGHT: u16 = 2503;
pub const IF_GETHIDE: u16 = 2504;
pub const IF_GETLAYER: u16 = 2505;
pub const IF_GETSCROLLX: u16 = 2600;
pub const IF_GETSCROLLY: u16 = 2601;
pub const IF_GETTEXT: u16 = 2602;
pub const IF_GETSCROLLWIDTH: u16 = 2603;
pub const IF_GETSCROLLHEIGHT: u16 = 2604;
pub const IF_GETINVOBJECT: u16 = 2700;
pub const IF_GETINVCOUNT: u16 = 2701;

// ---------------------------------------------------------------------------
// Server-related opcodes (3000-series)
// ---------------------------------------------------------------------------
pub const MES: u16 = 3100;
pub const ANIM: u16 = 3101;
pub const IF_CLOSE: u16 = 3103;
pub const RESUME_COUNTDIALOG: u16 = 3104;
pub const RESUME_NAMEDIALOG: u16 = 3105;
pub const RESUME_STRINGDIALOG: u16 = 3106;
pub const OPPLAYER: u16 = 3107;
pub const IF_OPENMAINSIDEMODAL: u16 = 3108;
pub const IF_OPENSIDEMODAL: u16 = 3109;
pub const IF_OPENMAINMODAL: u16 = 3110;
pub const IF_OPENMAINOVERLAY: u16 = 3115;
pub const SOUND_SYNTH: u16 = 3200;
pub const SOUND_SONG: u16 = 3201;
pub const SOUND_JINGLE: u16 = 3202;
pub const CLIENTCLOCK: u16 = 3300;
pub const INV_GETOBJ: u16 = 3301;
pub const INV_GETNUM: u16 = 3302;
pub const INV_TOTAL: u16 = 3303;
pub const INV_SIZE: u16 = 3304;
pub const STAT: u16 = 3305;
pub const STAT_BASE: u16 = 3306;
pub const STAT_XP: u16 = 3307;
pub const COORD: u16 = 3308;
pub const MAP_MEMBERS: u16 = 3312;
pub const WORLDID: u16 = 3314;
pub const STAFFMODLEVEL: u16 = 3316;
pub const REBOOTTIMER: u16 = 3317;
pub const ENUM_STRING: u16 = 3400;
pub const ENUM: u16 = 3408;
pub const ENUM_GETOUTPUTCOUNT: u16 = 3411;
pub const FRIEND_COUNT: u16 = 3600;
pub const FRIEND_GETNAME: u16 = 3601;
pub const FRIEND_GETWORLD: u16 = 3602;
pub const FRIEND_TEST: u16 = 3609;
pub const CLAN_GETCHATDISPLAYNAME: u16 = 3625;
pub const CLAN_GETCHATCOUNT: u16 = 3626;
pub const CLAN_GETCHATUSERNAME: u16 = 3627;
pub const CLAN_GETCHATUSERWORLD: u16 = 3628;
pub const CLAN_GETCHATUSERRANK: u16 = 3629;
pub const CLAN_GETMINKICK: u16 = 3630;
pub const CLAN_KICKUSER: u16 = 3631;
pub const CLAN_GETRANKS: u16 = 3632;
pub const CLAN_GETCHATCLANID: u16 = 3633;
pub const CLAN_LEAVECHAT: u16 = 3635;
pub const IGNORE_COUNT: u16 = 3700;
pub const IGNORE_GETNAME: u16 = 3701;
pub const IGNORE_TEST: u16 = 3709;

// ---------------------------------------------------------------------------
// Arithmetic / logic (4000-series)
// ---------------------------------------------------------------------------
pub const ADD: u16 = 4000;
pub const SUB: u16 = 4001;
pub const MULTIPLY: u16 = 4002;
pub const DIV: u16 = 4003;
pub const RANDOM: u16 = 4004;
pub const RANDOMINC: u16 = 4005;
pub const INTERPOLATE: u16 = 4006;
pub const ADDPERCENT: u16 = 4007;
pub const SETBIT: u16 = 4008;
pub const CLEARBIT: u16 = 4009;
pub const TESTBIT: u16 = 4010;
pub const MODULO: u16 = 4011;
pub const POW: u16 = 4012;
pub const INVPOW: u16 = 4013;
pub const AND: u16 = 4014;
pub const OR: u16 = 4015;
pub const MIN: u16 = 4016;
pub const MAX: u16 = 4017;
pub const SCALE: u16 = 4018;
pub const APPEND_NUM: u16 = 4100;
pub const APPEND: u16 = 4101;
pub const APPEND_SIGNNUM: u16 = 4102;
pub const LOWERCASE: u16 = 4103;
pub const TEXT_GENDER: u16 = 4104;
pub const TOSTRING: u16 = 4106;
pub const COMPARE: u16 = 4107;
pub const PARAHEIGHT: u16 = 4108;
pub const PARAWIDTH: u16 = 4109;
pub const TEXT_SWITCH: u16 = 4110;
pub const ESCAPE: u16 = 4112;
pub const STRING_LENGTH: u16 = 4117;
pub const SUBSTRING: u16 = 4118;
pub const REMOVETAGS: u16 = 4119;
pub const STRING_INDEXOF_CHAR: u16 = 4120;
pub const STRING_INDEXOF_STRING: u16 = 4121;
pub const CHAR_ISPRINTABLE: u16 = 4122;
pub const CHAR_ISALPHANUMERIC: u16 = 4123;
pub const CHAR_ISALPHA: u16 = 4124;
pub const CHAR_ISNUMERIC: u16 = 4125;

// ---------------------------------------------------------------------------
// OC / NC / LC param lookups (4200+ / 6500+)
// ---------------------------------------------------------------------------
pub const OC_NAME: u16 = 4200;
pub const OC_OP: u16 = 4201;
pub const OC_IOP: u16 = 4202;
pub const OC_COST: u16 = 4203;
pub const OC_STACKABLE: u16 = 4204;
pub const OC_CERT: u16 = 4205;
pub const OC_UNCERT: u16 = 4206;
pub const OC_MEMBERS: u16 = 4207;
pub const OC_WEIGHT: u16 = 4208;
pub const OC_WEARPOS: u16 = 4209;
pub const OC_WEARPOS2: u16 = 4210;

// ---------------------------------------------------------------------------
// Chat / settings / camera (5000-series)
// ---------------------------------------------------------------------------
pub const CHAT_GETFILTER_PUBLIC: u16 = 5000;
pub const CHAT_SETFILTER: u16 = 5001;
pub const CHAT_SENDABUSEREPORT: u16 = 5002;
pub const CHAT_GETHISTORY_BYTYPEANDLINE: u16 = 5003;
pub const CHAT_GETHISTORY_LENGTH: u16 = 5004;
pub const CHAT_PLAYERNAME: u16 = 5015;
pub const DOCHEAT: u16 = 5020;
pub const GETWINDOWMODE: u16 = 5306;
pub const SETWINDOWMODE: u16 = 5307;
pub const GETDEFAULTWINDOWMODE: u16 = 5308;
pub const CAM_FORCEANGLE: u16 = 5504;
pub const LOGOUT: u16 = 5630;

// ---------------------------------------------------------------------------
// Viewport / world list / param lookups (6000-series)
// ---------------------------------------------------------------------------
pub const VIEWPORT_SETFOV: u16 = 6200;
pub const WORLDLIST_FETCH: u16 = 6500;
pub const WORLDLIST_START: u16 = 6501;
pub const WORLDLIST_NEXT: u16 = 6502;
pub const NC_PARAM: u16 = 6513;
pub const LC_PARAM: u16 = 6514;
pub const OC_PARAM: u16 = 6515;
pub const STRUCT_PARAM: u16 = 6516;
pub const ON_MOBILE: u16 = 6518;
pub const CLIENTTYPE: u16 = 6519;

// ---------------------------------------------------------------------------
// Additional rev-647 opcodes (from client source)
// ---------------------------------------------------------------------------
pub const PUSH_CLANSETTING_VAR: u16 = 113;
pub const PLAYER_SETBODYTYPE: u16 = 403;
pub const IF_SETONVARCSTRINGTRANSMIT: u16 = 2426;
pub const IF_SETONVARCTRANSMIT: u16 = 2428;
pub const IF_GETTRANS2: u16 = 2609;
pub const IF_GETGRAPHIC: u16 = 2612;
pub const IF_GETCHILDCOUNT: u16 = 2703;
pub const COORDX: u16 = 3309;
pub const COORDY: u16 = 3310;
pub const COORDZ: u16 = 3311;
pub const PLAYER_ISFEMALE: u16 = 3327;
pub const COORDPACK: u16 = 3336;
pub const NC_PARAM2: u16 = 4300;
pub const STRUCT_PARAM2: u16 = 4500;
pub const PLAYER_GETDISPLAYNAME: u16 = 5010;
pub const CAM_ALLOCPATH: u16 = 5405;
pub const CAM_ADDWAYPOINT: u16 = 5406;
pub const CAM_STARTPATH: u16 = 5502;

// ---------------------------------------------------------------------------
// Opcode metadata
// ---------------------------------------------------------------------------

/// Stack I/O metadata for a single opcode.
#[derive(Clone, Copy)]
pub struct OpcodeInfo {
    pub name: &'static str,
    pub pops_int: u8,
    pub pops_string: u8,
    pub pushes_int: u8,
    pub pushes_string: u8,
}

/// Return the human-readable name for an opcode, or `""` if unknown.
pub fn opcode_name(opcode: u16) -> &'static str {
    match opcode {
        PUSH_CONSTANT_INT => "push_constant_int",
        PUSH_VAR => "push_var",
        POP_VAR => "pop_var",
        PUSH_CONSTANT_STRING => "push_constant_string",
        BRANCH => "branch",
        BRANCH_NOT => "branch_not",
        BRANCH_EQUALS => "branch_equals",
        BRANCH_LESS_THAN => "branch_less_than",
        BRANCH_GREATER_THAN => "branch_greater_than",
        RETURN => "return",
        PUSH_VARBIT => "push_varbit",
        POP_VARBIT => "pop_varbit",
        BRANCH_LESS_THAN_OR_EQUALS => "branch_less_than_or_equals",
        BRANCH_GREATER_THAN_OR_EQUALS => "branch_greater_than_or_equals",
        PUSH_INT_LOCAL => "push_int_local",
        POP_INT_LOCAL => "pop_int_local",
        PUSH_STRING_LOCAL => "push_string_local",
        POP_STRING_LOCAL => "pop_string_local",
        JOIN_STRING => "join_string",
        POP_INT_DISCARD => "pop_int_discard",
        POP_STRING_DISCARD => "pop_string_discard",
        GOSUB_WITH_PARAMS => "gosub_with_params",
        PUSH_VARC_INT => "push_varc_int",
        POP_VARC_INT => "pop_varc_int",
        DEFINE_ARRAY => "define_array",
        PUSH_ARRAY_INT => "push_array_int",
        POP_ARRAY_INT => "pop_array_int",
        PUSH_VARC_STRING_OLD => "push_varc_string_old",
        POP_VARC_STRING_OLD => "pop_varc_string_old",
        SWITCH => "switch",
        PUSH_CONSTANT_LONG => "push_constant_long",
        POP_LONG_DISCARD => "pop_long_discard",
        PUSH_LONG_LOCAL => "push_long_local",
        POP_LONG_LOCAL => "pop_long_local",
        BRANCH_NOT_LONG => "branch_not_long",
        BRANCH_EQUALS_LONG => "branch_equals_long",
        BRANCH_LESS_THAN_LONG => "branch_less_than_long",
        BRANCH_GREATER_THAN_LONG => "branch_greater_than_long",
        BRANCH_LESS_THAN_OR_EQUALS_LONG => "branch_less_than_or_equals_long",
        BRANCH_GREATER_THAN_OR_EQUALS_LONG => "branch_greater_than_or_equals_long",
        BRANCH_IF_TRUE => "branch_if_true",
        BRANCH_IF_FALSE => "branch_if_false",
        CC_CREATE => "cc_create",
        CC_DELETE => "cc_delete",
        CC_DELETEALL => "cc_deleteall",
        CC_SETPOSITION => "cc_setposition",
        CC_FIND => "cc_find",
        IF_FIND => "if_find",
        CC_SETPOSITION2 => "cc_setposition2",
        CC_SETSIZE => "cc_setsize",
        CC_SETHIDE => "cc_sethide",
        CC_SETASPECT_RATIO => "cc_setaspectratio",
        CC_SETNOCLICKTHROUGH => "cc_setnoclickthrough",
        CC_SETSCROLLPOS => "cc_setscrollpos",
        CC_SETCOLOUR => "cc_setcolour",
        CC_SETFILL => "cc_setfill",
        CC_SETTRANS => "cc_settrans",
        CC_SETGRAPHIC => "cc_setgraphic",
        CC_SET2DANGLE => "cc_set2dangle",
        CC_SETTILING => "cc_settiling",
        CC_SETMODEL => "cc_setmodel",
        CC_SETMODELANGLE => "cc_setmodelangle",
        CC_SETMODELANIM => "cc_setmodelanim",
        CC_SETMODELORTHOG => "cc_setmodelorthog",
        CC_SETTEXT => "cc_settext",
        CC_SETTEXTFONT => "cc_settextfont",
        CC_SETTEXTALIGN => "cc_settextalign",
        CC_SETTEXTSHADOW => "cc_settextshadow",
        CC_SETOUTLINE => "cc_setoutline",
        CC_SETGRAPHICSHADOW => "cc_setgraphicshadow",
        CC_SETVFLIP => "cc_setvflip",
        CC_SETHFLIP => "cc_sethflip",
        CC_SETSCROLLSIZE => "cc_setscrollsize",
        CC_SETOBJECT => "cc_setobject",
        CC_SETOBJECT_NONUM => "cc_setobject_nonum",
        CC_SETOP => "cc_setop",
        CC_SETDRAGGABLE => "cc_setdraggable",
        CC_SETDRAGDEADZONE => "cc_setdragdeadzone",
        CC_SETDRAGDEADTIME => "cc_setdragdeadtime",
        CC_SETOPBASE => "cc_setopbase",
        CC_SETTARGETVERB => "cc_settargetverb",
        CC_CLEAROPS => "cc_clearops",
        CC_SETOPKEY => "cc_setopkey",
        CC_SETOPTKEY => "cc_setoptkey",
        CC_SETONCLICK => "cc_setonclick",
        CC_SETONHOLD => "cc_setonhold",
        CC_SETONRELEASE => "cc_setonrelease",
        CC_SETONMOUSEOVER => "cc_setonmouseover",
        CC_SETONMOUSELEAVE => "cc_setonmouseleave",
        CC_SETONDRAG => "cc_setondrag",
        CC_SETONTARGETLEAVE => "cc_setontargetleave",
        CC_SETONVARTRANSMIT => "cc_setonvartransmit",
        CC_SETONTIMER => "cc_setontimer",
        CC_SETONOP => "cc_setonop",
        CC_SETONDRAGCOMPLETE => "cc_setondragcomplete",
        CC_SETONCLICKREPEAT => "cc_setonclickrepeat",
        CC_SETONMOUSEREPEAT => "cc_setonmouserepeat",
        CC_SETONINVTRANSMIT => "cc_setoninvtransmit",
        CC_SETONSTATTRANSMIT => "cc_setonstattransmit",
        CC_SETONTARGETENTER => "cc_setontargetenter",
        CC_SETONSCROLLWHEEL => "cc_setonscrollwheel",
        CC_SETONCHATTRANSMIT => "cc_setonchattransmit",
        CC_SETONKEY => "cc_setonkey",
        CC_GETX => "cc_getx",
        CC_GETY => "cc_gety",
        CC_GETWIDTH => "cc_getwidth",
        CC_GETHEIGHT => "cc_getheight",
        CC_GETHIDE => "cc_gethide",
        CC_GETLAYER => "cc_getlayer",
        CC_GETSCROLLX => "cc_getscrollx",
        CC_GETSCROLLY => "cc_getscrolly",
        CC_GETTEXT => "cc_gettext",
        CC_GETSCROLLWIDTH => "cc_getscrollwidth",
        CC_GETSCROLLHEIGHT => "cc_getscrollheight",
        CC_GETMODELZOOM => "cc_getmodelzoom",
        CC_GETMODELANGLE_X => "cc_getmodelangle_x",
        CC_GETMODELANGLE_Z => "cc_getmodelangle_z",
        CC_GETMODELANGLE_Y => "cc_getmodelangle_y",
        CC_GETTRANS => "cc_gettrans",
        CC_GETINVOBJECT => "cc_getinvobject",
        CC_GETINVCOUNT => "cc_getinvcount",
        CC_GETID => "cc_getid",
        IF_SETPOSITION => "if_setposition",
        IF_SETSIZE => "if_setsize",
        IF_SETHIDE => "if_sethide",
        IF_SETSCROLLPOS => "if_setscrollpos",
        IF_SETCOLOUR => "if_setcolour",
        IF_SETFILL => "if_setfill",
        IF_SETTRANS => "if_settrans",
        IF_SETGRAPHIC => "if_setgraphic",
        IF_SETMODEL => "if_setmodel",
        IF_SETMODELANGLE => "if_setmodelangle",
        IF_SETMODELANIM => "if_setmodelanim",
        IF_SETTEXT => "if_settext",
        IF_SETTEXTFONT => "if_settextfont",
        IF_SETTEXTALIGN => "if_settextalign",
        IF_SETOBJECT => "if_setobject",
        IF_SETOBJECT_NONUM => "if_setobject_nonum",
        IF_SETOP => "if_setop",
        IF_SETDRAGGABLE => "if_setdraggable",
        IF_SETOPBASE => "if_setopbase",
        IF_SETTARGETVERB => "if_settargetverb",
        IF_CLEAROPS => "if_clearops",
        IF_SETOPKEY => "if_setopkey",
        IF_SETONCLICK => "if_setonclick",
        IF_SETONHOLD => "if_setonhold",
        IF_SETONRELEASE => "if_setonrelease",
        IF_SETONMOUSEOVER => "if_setonmouseover",
        IF_SETONMOUSELEAVE => "if_setonmouseleave",
        IF_SETONDRAG => "if_setondrag",
        IF_SETONVARTRANSMIT => "if_setonvartransmit",
        IF_SETONTIMER => "if_setontimer",
        IF_SETONOP => "if_setonop",
        IF_SETONDRAGCOMPLETE => "if_setondragcomplete",
        IF_SETONCLICKREPEAT => "if_setonclickrepeat",
        IF_SETONMOUSEREPEAT => "if_setonmouserepeat",
        IF_SETONINVTRANSMIT => "if_setoninvtransmit",
        IF_SETONSTATTRANSMIT => "if_setonstattransmit",
        IF_SETONTARGETENTER => "if_setontargetenter",
        IF_SETONSCROLLWHEEL => "if_setonscrollwheel",
        IF_SETONCHATTRANSMIT => "if_setonchattransmit",
        IF_SETONKEY => "if_setonkey",
        IF_GETX => "if_getx",
        IF_GETY => "if_gety",
        IF_GETWIDTH => "if_getwidth",
        IF_GETHEIGHT => "if_getheight",
        IF_GETHIDE => "if_gethide",
        IF_GETLAYER => "if_getlayer",
        IF_GETSCROLLX => "if_getscrollx",
        IF_GETSCROLLY => "if_getscrolly",
        IF_GETTEXT => "if_gettext",
        IF_GETSCROLLWIDTH => "if_getscrollwidth",
        IF_GETSCROLLHEIGHT => "if_getscrollheight",
        IF_GETINVOBJECT => "if_getinvobject",
        IF_GETINVCOUNT => "if_getinvcount",
        MES => "mes",
        ANIM => "anim",
        IF_CLOSE => "if_close",
        RESUME_COUNTDIALOG => "resume_countdialog",
        RESUME_NAMEDIALOG => "resume_namedialog",
        RESUME_STRINGDIALOG => "resume_stringdialog",
        OPPLAYER => "opplayer",
        IF_OPENMAINSIDEMODAL => "if_openmainsidemodal",
        IF_OPENSIDEMODAL => "if_opensidemodal",
        IF_OPENMAINMODAL => "if_openmainmodal",
        IF_OPENMAINOVERLAY => "if_openmainoverlay",
        SOUND_SYNTH => "sound_synth",
        SOUND_SONG => "sound_song",
        SOUND_JINGLE => "sound_jingle",
        CLIENTCLOCK => "clientclock",
        INV_GETOBJ => "inv_getobj",
        INV_GETNUM => "inv_getnum",
        INV_TOTAL => "inv_total",
        INV_SIZE => "inv_size",
        STAT => "stat",
        STAT_BASE => "stat_base",
        STAT_XP => "stat_xp",
        COORD => "coord",
        MAP_MEMBERS => "map_members",
        WORLDID => "worldid",
        STAFFMODLEVEL => "staffmodlevel",
        REBOOTTIMER => "reboottimer",
        ENUM_STRING => "enum_string",
        ENUM => "enum",
        ENUM_GETOUTPUTCOUNT => "enum_getoutputcount",
        FRIEND_COUNT => "friend_count",
        FRIEND_GETNAME => "friend_getname",
        FRIEND_GETWORLD => "friend_getworld",
        FRIEND_TEST => "friend_test",
        CLAN_GETCHATDISPLAYNAME => "clan_getchatdisplayname",
        CLAN_GETCHATCOUNT => "clan_getchatcount",
        CLAN_GETCHATUSERNAME => "clan_getchatusername",
        CLAN_GETCHATUSERWORLD => "clan_getchatuserworld",
        CLAN_GETCHATUSERRANK => "clan_getchatuserrank",
        CLAN_GETMINKICK => "clan_getminkick",
        CLAN_KICKUSER => "clan_kickuser",
        CLAN_GETRANKS => "clan_getranks",
        CLAN_GETCHATCLANID => "clan_getchatclanid",
        CLAN_LEAVECHAT => "clan_leavechat",
        IGNORE_COUNT => "ignore_count",
        IGNORE_GETNAME => "ignore_getname",
        IGNORE_TEST => "ignore_test",
        ADD => "add",
        SUB => "sub",
        MULTIPLY => "multiply",
        DIV => "div",
        RANDOM => "random",
        RANDOMINC => "randominc",
        INTERPOLATE => "interpolate",
        ADDPERCENT => "addpercent",
        SETBIT => "setbit",
        CLEARBIT => "clearbit",
        TESTBIT => "testbit",
        MODULO => "mod",
        POW => "pow",
        INVPOW => "invpow",
        AND => "and",
        OR => "or",
        MIN => "min",
        MAX => "max",
        SCALE => "scale",
        APPEND_NUM => "append_num",
        APPEND => "append",
        APPEND_SIGNNUM => "append_signnum",
        LOWERCASE => "lowercase",
        TEXT_GENDER => "text_gender",
        TOSTRING => "tostring",
        COMPARE => "compare",
        PARAHEIGHT => "paraheight",
        PARAWIDTH => "parawidth",
        TEXT_SWITCH => "text_switch",
        ESCAPE => "escape",
        STRING_LENGTH => "string_length",
        SUBSTRING => "substring",
        REMOVETAGS => "removetags",
        STRING_INDEXOF_CHAR => "string_indexof_char",
        STRING_INDEXOF_STRING => "string_indexof_string",
        CHAR_ISPRINTABLE => "char_isprintable",
        CHAR_ISALPHANUMERIC => "char_isalphanumeric",
        CHAR_ISALPHA => "char_isalpha",
        CHAR_ISNUMERIC => "char_isnumeric",
        OC_NAME => "oc_name",
        OC_OP => "oc_op",
        OC_IOP => "oc_iop",
        OC_COST => "oc_cost",
        OC_STACKABLE => "oc_stackable",
        OC_CERT => "oc_cert",
        OC_UNCERT => "oc_uncert",
        OC_MEMBERS => "oc_members",
        OC_WEIGHT => "oc_weight",
        OC_WEARPOS => "oc_wearpos",
        OC_WEARPOS2 => "oc_wearpos2",
        CHAT_GETFILTER_PUBLIC => "chat_getfilter_public",
        CHAT_SETFILTER => "chat_setfilter",
        CHAT_SENDABUSEREPORT => "chat_sendabusereport",
        CHAT_GETHISTORY_BYTYPEANDLINE => "chat_gethistory_bytypeandline",
        CHAT_GETHISTORY_LENGTH => "chat_gethistory_length",
        CHAT_PLAYERNAME => "chat_playername",
        DOCHEAT => "docheat",
        GETWINDOWMODE => "getwindowmode",
        SETWINDOWMODE => "setwindowmode",
        GETDEFAULTWINDOWMODE => "getdefaultwindowmode",
        CAM_FORCEANGLE => "cam_forceangle",
        LOGOUT => "logout",
        VIEWPORT_SETFOV => "viewport_setfov",
        WORLDLIST_FETCH => "worldlist_fetch",
        WORLDLIST_START => "worldlist_start",
        WORLDLIST_NEXT => "worldlist_next",
        NC_PARAM => "nc_param",
        LC_PARAM => "lc_param",
        OC_PARAM => "oc_param",
        STRUCT_PARAM => "struct_param",
        ON_MOBILE => "on_mobile",
        CLIENTTYPE => "clienttype",
        PUSH_CLANSETTING_VAR => "push_clansetting_var",
        PLAYER_SETBODYTYPE => "player_setbodytype",
        IF_SETONVARCSTRINGTRANSMIT => "if_setonvarcstringtransmit",
        IF_SETONVARCTRANSMIT => "if_setonvarctransmit",
        IF_GETTRANS2 => "if_gettrans",
        IF_GETGRAPHIC => "if_getgraphic",
        IF_GETCHILDCOUNT => "if_getchildcount",
        COORDX => "coordx",
        COORDY => "coordy",
        COORDZ => "coordz",
        PLAYER_ISFEMALE => "player_isfemale",
        COORDPACK => "coordpack",
        NC_PARAM2 => "nc_param",
        STRUCT_PARAM2 => "struct_param",
        PLAYER_GETDISPLAYNAME => "player_getdisplayname",
        CAM_ALLOCPATH => "cam_allocpath",
        CAM_ADDWAYPOINT => "cam_addwaypoint",
        CAM_STARTPATH => "cam_startpath",
        _ => {
            let info = opcode_info(opcode);
            info.name
        }
    }
}

/// Return stack I/O metadata for known opcodes.
///
/// For unknown opcodes the name will be empty and all counts zero (the caller
/// should fall back to treating the opcode as a generic operation).
pub fn opcode_info(opcode: u16) -> OpcodeInfo {
    // Many opcodes have simple stack signatures. We encode the common ones
    // explicitly; everything else gets a conservative (0,0,0,0).
    match opcode {
        // --- Core ---
        PUSH_CONSTANT_INT => OpcodeInfo {
            name: "push_constant_int",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        PUSH_VAR => OpcodeInfo {
            name: "push_var",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_VAR => OpcodeInfo {
            name: "pop_var",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_CONSTANT_STRING => OpcodeInfo {
            name: "push_constant_string",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        BRANCH => OpcodeInfo {
            name: "branch",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_NOT => OpcodeInfo {
            name: "branch_not",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_EQUALS => OpcodeInfo {
            name: "branch_equals",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_LESS_THAN => OpcodeInfo {
            name: "branch_less_than",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_GREATER_THAN => OpcodeInfo {
            name: "branch_greater_than",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_LESS_THAN_OR_EQUALS => OpcodeInfo {
            name: "branch_less_than_or_equals",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_GREATER_THAN_OR_EQUALS => OpcodeInfo {
            name: "branch_greater_than_or_equals",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        RETURN => OpcodeInfo {
            name: "return",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_VARBIT => OpcodeInfo {
            name: "push_varbit",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_VARBIT => OpcodeInfo {
            name: "pop_varbit",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_INT_LOCAL => OpcodeInfo {
            name: "push_int_local",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_INT_LOCAL => OpcodeInfo {
            name: "pop_int_local",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_STRING_LOCAL => OpcodeInfo {
            name: "push_string_local",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        POP_STRING_LOCAL => OpcodeInfo {
            name: "pop_string_local",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        JOIN_STRING => OpcodeInfo {
            name: "join_string",
            pops_int: 0,
            pops_string: 0, /* dynamic */
            pushes_int: 0,
            pushes_string: 1,
        },
        POP_INT_DISCARD => OpcodeInfo {
            name: "pop_int_discard",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        POP_STRING_DISCARD => OpcodeInfo {
            name: "pop_string_discard",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        GOSUB_WITH_PARAMS => OpcodeInfo {
            name: "gosub_with_params",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_VARC_INT => OpcodeInfo {
            name: "push_varc_int",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_VARC_INT => OpcodeInfo {
            name: "pop_varc_int",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        DEFINE_ARRAY => OpcodeInfo {
            name: "define_array",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_ARRAY_INT => OpcodeInfo {
            name: "push_array_int",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_ARRAY_INT => OpcodeInfo {
            name: "pop_array_int",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_VARC_STRING_OLD => OpcodeInfo {
            name: "push_varc_string_old",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        POP_VARC_STRING_OLD => OpcodeInfo {
            name: "pop_varc_string_old",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        SWITCH => OpcodeInfo {
            name: "switch",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_CONSTANT_LONG => OpcodeInfo {
            name: "push_constant_long",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_LONG_DISCARD => OpcodeInfo {
            name: "pop_long_discard",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PUSH_LONG_LOCAL => OpcodeInfo {
            name: "push_long_local",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POP_LONG_LOCAL => OpcodeInfo {
            name: "pop_long_local",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_NOT_LONG => OpcodeInfo {
            name: "branch_not_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_EQUALS_LONG => OpcodeInfo {
            name: "branch_equals_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_LESS_THAN_LONG => OpcodeInfo {
            name: "branch_less_than_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_GREATER_THAN_LONG => OpcodeInfo {
            name: "branch_greater_than_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_LESS_THAN_OR_EQUALS_LONG => OpcodeInfo {
            name: "branch_less_than_or_equals_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_GREATER_THAN_OR_EQUALS_LONG => OpcodeInfo {
            name: "branch_greater_than_or_equals_long",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_IF_TRUE => OpcodeInfo {
            name: "branch_if_true",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        BRANCH_IF_FALSE => OpcodeInfo {
            name: "branch_if_false",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },

        // --- Arithmetic ---
        ADD => OpcodeInfo {
            name: "add",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        SUB => OpcodeInfo {
            name: "sub",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        MULTIPLY => OpcodeInfo {
            name: "multiply",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        DIV => OpcodeInfo {
            name: "div",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        RANDOM => OpcodeInfo {
            name: "random",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        RANDOMINC => OpcodeInfo {
            name: "randominc",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INTERPOLATE => OpcodeInfo {
            name: "interpolate",
            pops_int: 5,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        ADDPERCENT => OpcodeInfo {
            name: "addpercent",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        SETBIT => OpcodeInfo {
            name: "setbit",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLEARBIT => OpcodeInfo {
            name: "clearbit",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        TESTBIT => OpcodeInfo {
            name: "testbit",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        MODULO => OpcodeInfo {
            name: "mod",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        POW => OpcodeInfo {
            name: "pow",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INVPOW => OpcodeInfo {
            name: "invpow",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        AND => OpcodeInfo {
            name: "and",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OR => OpcodeInfo {
            name: "or",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        MIN => OpcodeInfo {
            name: "min",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        MAX => OpcodeInfo {
            name: "max",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        SCALE => OpcodeInfo {
            name: "scale",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // --- String operations ---
        APPEND_NUM => OpcodeInfo {
            name: "append_num",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        APPEND => OpcodeInfo {
            name: "append",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 1,
        },
        APPEND_SIGNNUM => OpcodeInfo {
            name: "append_signnum",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        LOWERCASE => OpcodeInfo {
            name: "lowercase",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        TOSTRING => OpcodeInfo {
            name: "tostring",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        COMPARE => OpcodeInfo {
            name: "compare",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 1,
            pushes_string: 0,
        },
        PARAHEIGHT => OpcodeInfo {
            name: "paraheight",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        PARAWIDTH => OpcodeInfo {
            name: "parawidth",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        TEXT_SWITCH => OpcodeInfo {
            name: "text_switch",
            pops_int: 1,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 1,
        },
        ESCAPE => OpcodeInfo {
            name: "escape",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        STRING_LENGTH => OpcodeInfo {
            name: "string_length",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        SUBSTRING => OpcodeInfo {
            name: "substring",
            pops_int: 2,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        REMOVETAGS => OpcodeInfo {
            name: "removetags",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        STRING_INDEXOF_CHAR => OpcodeInfo {
            name: "string_indexof_char",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        STRING_INDEXOF_STRING => OpcodeInfo {
            name: "string_indexof_string",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAR_ISPRINTABLE => OpcodeInfo {
            name: "char_isprintable",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAR_ISALPHANUMERIC => OpcodeInfo {
            name: "char_isalphanumeric",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAR_ISALPHA => OpcodeInfo {
            name: "char_isalpha",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAR_ISNUMERIC => OpcodeInfo {
            name: "char_isnumeric",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // --- Object config lookups ---
        OC_NAME => OpcodeInfo {
            name: "oc_name",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        OC_OP => OpcodeInfo {
            name: "oc_op",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        OC_IOP => OpcodeInfo {
            name: "oc_iop",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        OC_COST => OpcodeInfo {
            name: "oc_cost",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OC_STACKABLE => OpcodeInfo {
            name: "oc_stackable",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OC_CERT => OpcodeInfo {
            name: "oc_cert",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OC_UNCERT => OpcodeInfo {
            name: "oc_uncert",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OC_MEMBERS => OpcodeInfo {
            name: "oc_members",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // --- Server info ---
        MES => OpcodeInfo {
            name: "mes",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        ANIM => OpcodeInfo {
            name: "anim",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_CLOSE => OpcodeInfo {
            name: "if_close",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        RESUME_COUNTDIALOG => OpcodeInfo {
            name: "resume_countdialog",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        RESUME_NAMEDIALOG => OpcodeInfo {
            name: "resume_namedialog",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        RESUME_STRINGDIALOG => OpcodeInfo {
            name: "resume_stringdialog",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        SOUND_SYNTH => OpcodeInfo {
            name: "sound_synth",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        SOUND_SONG => OpcodeInfo {
            name: "sound_song",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        SOUND_JINGLE => OpcodeInfo {
            name: "sound_jingle",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CLIENTCLOCK => OpcodeInfo {
            name: "clientclock",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INV_GETOBJ => OpcodeInfo {
            name: "inv_getobj",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INV_GETNUM => OpcodeInfo {
            name: "inv_getnum",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INV_TOTAL => OpcodeInfo {
            name: "inv_total",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        INV_SIZE => OpcodeInfo {
            name: "inv_size",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        STAT => OpcodeInfo {
            name: "stat",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        STAT_BASE => OpcodeInfo {
            name: "stat_base",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        STAT_XP => OpcodeInfo {
            name: "stat_xp",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        COORD => OpcodeInfo {
            name: "coord",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        MAP_MEMBERS => OpcodeInfo {
            name: "map_members",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        STAFFMODLEVEL => OpcodeInfo {
            name: "staffmodlevel",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        REBOOTTIMER => OpcodeInfo {
            name: "reboottimer",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        ENUM_STRING => OpcodeInfo {
            name: "enum_string",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        ENUM => OpcodeInfo {
            name: "enum",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        ENUM_GETOUTPUTCOUNT => OpcodeInfo {
            name: "enum_getoutputcount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        FRIEND_COUNT => OpcodeInfo {
            name: "friend_count",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        FRIEND_GETNAME => OpcodeInfo {
            name: "friend_getname",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        FRIEND_GETWORLD => OpcodeInfo {
            name: "friend_getworld",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        FRIEND_TEST => OpcodeInfo {
            name: "friend_test",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAT_PLAYERNAME => OpcodeInfo {
            name: "chat_playername",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        DOCHEAT => OpcodeInfo {
            name: "docheat",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        GETWINDOWMODE => OpcodeInfo {
            name: "getwindowmode",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        SETWINDOWMODE => OpcodeInfo {
            name: "setwindowmode",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        LOGOUT => OpcodeInfo {
            name: "logout",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        WORLDLIST_FETCH => OpcodeInfo {
            name: "worldlist_fetch",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        WORLDLIST_START => OpcodeInfo {
            name: "worldlist_start",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        WORLDLIST_NEXT => OpcodeInfo {
            name: "worldlist_next",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        NC_PARAM => OpcodeInfo {
            name: "nc_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        LC_PARAM => OpcodeInfo {
            name: "lc_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        OC_PARAM => OpcodeInfo {
            name: "oc_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        STRUCT_PARAM => OpcodeInfo {
            name: "struct_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        ON_MOBILE => OpcodeInfo {
            name: "on_mobile",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLIENTTYPE => OpcodeInfo {
            name: "clienttype",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // --- Component setters (various stack signatures) ---
        CC_CREATE => OpcodeInfo {
            name: "cc_create",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_DELETE => OpcodeInfo {
            name: "cc_delete",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_DELETEALL => OpcodeInfo {
            name: "cc_deleteall",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_FIND => OpcodeInfo {
            name: "cc_find",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_FIND => OpcodeInfo {
            name: "if_find",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // cc_set* that take various int args
        CC_SETPOSITION => OpcodeInfo {
            name: "cc_setposition",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETPOSITION2 => OpcodeInfo {
            name: "cc_setposition2",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETSIZE => OpcodeInfo {
            name: "cc_setsize",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETHIDE => OpcodeInfo {
            name: "cc_sethide",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETSCROLLPOS => OpcodeInfo {
            name: "cc_setscrollpos",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETCOLOUR => OpcodeInfo {
            name: "cc_setcolour",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETFILL => OpcodeInfo {
            name: "cc_setfill",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTRANS => OpcodeInfo {
            name: "cc_settrans",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETGRAPHIC => OpcodeInfo {
            name: "cc_setgraphic",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETMODEL => OpcodeInfo {
            name: "cc_setmodel",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETMODELANGLE => OpcodeInfo {
            name: "cc_setmodelangle",
            pops_int: 6,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETMODELANIM => OpcodeInfo {
            name: "cc_setmodelanim",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTEXT => OpcodeInfo {
            name: "cc_settext",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTEXTFONT => OpcodeInfo {
            name: "cc_settextfont",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTEXTALIGN => OpcodeInfo {
            name: "cc_settextalign",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTEXTSHADOW => OpcodeInfo {
            name: "cc_settextshadow",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETSCROLLSIZE => OpcodeInfo {
            name: "cc_setscrollsize",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETOBJECT => OpcodeInfo {
            name: "cc_setobject",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETOBJECT_NONUM => OpcodeInfo {
            name: "cc_setobject_nonum",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETOP => OpcodeInfo {
            name: "cc_setop",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETDRAGGABLE => OpcodeInfo {
            name: "cc_setdraggable",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETOPBASE => OpcodeInfo {
            name: "cc_setopbase",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_SETTARGETVERB => OpcodeInfo {
            name: "cc_settargetverb",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        CC_CLEAROPS => OpcodeInfo {
            name: "cc_clearops",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },

        // cc_get* (pushes one value)
        CC_GETX => OpcodeInfo {
            name: "cc_getx",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETY => OpcodeInfo {
            name: "cc_gety",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETWIDTH => OpcodeInfo {
            name: "cc_getwidth",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETHEIGHT => OpcodeInfo {
            name: "cc_getheight",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETHIDE => OpcodeInfo {
            name: "cc_gethide",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETLAYER => OpcodeInfo {
            name: "cc_getlayer",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETSCROLLX => OpcodeInfo {
            name: "cc_getscrollx",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETSCROLLY => OpcodeInfo {
            name: "cc_getscrolly",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETTEXT => OpcodeInfo {
            name: "cc_gettext",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        CC_GETINVOBJECT => OpcodeInfo {
            name: "cc_getinvobject",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETINVCOUNT => OpcodeInfo {
            name: "cc_getinvcount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CC_GETID => OpcodeInfo {
            name: "cc_getid",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // if_set* (mirror cc_set* but take a component hash first)
        IF_SETPOSITION => OpcodeInfo {
            name: "if_setposition",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETSIZE => OpcodeInfo {
            name: "if_setsize",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETHIDE => OpcodeInfo {
            name: "if_sethide",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETSCROLLPOS => OpcodeInfo {
            name: "if_setscrollpos",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETCOLOUR => OpcodeInfo {
            name: "if_setcolour",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETTRANS => OpcodeInfo {
            name: "if_settrans",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETGRAPHIC => OpcodeInfo {
            name: "if_setgraphic",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETTEXT => OpcodeInfo {
            name: "if_settext",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETTEXTFONT => OpcodeInfo {
            name: "if_settextfont",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETTEXTALIGN => OpcodeInfo {
            name: "if_settextalign",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETOBJECT => OpcodeInfo {
            name: "if_setobject",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETOP => OpcodeInfo {
            name: "if_setop",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_SETDRAGGABLE => OpcodeInfo {
            name: "if_setdraggable",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_CLEAROPS => OpcodeInfo {
            name: "if_clearops",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },

        // if_get*
        IF_GETX => OpcodeInfo {
            name: "if_getx",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETY => OpcodeInfo {
            name: "if_gety",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETWIDTH => OpcodeInfo {
            name: "if_getwidth",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETHEIGHT => OpcodeInfo {
            name: "if_getheight",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETHIDE => OpcodeInfo {
            name: "if_gethide",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETTEXT => OpcodeInfo {
            name: "if_gettext",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        IF_GETINVOBJECT => OpcodeInfo {
            name: "if_getinvobject",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IF_GETINVCOUNT => OpcodeInfo {
            name: "if_getinvcount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // --- Named constants missing from explicit match arms ---
        OPPLAYER => OpcodeInfo {
            name: "opplayer",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_OPENMAINSIDEMODAL => OpcodeInfo {
            name: "if_openmainsidemodal",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_OPENSIDEMODAL => OpcodeInfo {
            name: "if_opensidemodal",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_OPENMAINMODAL => OpcodeInfo {
            name: "if_openmainmodal",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        IF_OPENMAINOVERLAY => OpcodeInfo {
            name: "if_openmainoverlay",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CLAN_GETCHATDISPLAYNAME => OpcodeInfo {
            name: "clan_getchatdisplayname",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        CLAN_GETCHATCOUNT => OpcodeInfo {
            name: "clan_getchatcount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_GETCHATUSERNAME => OpcodeInfo {
            name: "clan_getchatusername",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        CLAN_GETCHATUSERWORLD => OpcodeInfo {
            name: "clan_getchatuserworld",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_GETCHATUSERRANK => OpcodeInfo {
            name: "clan_getchatuserrank",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_GETMINKICK => OpcodeInfo {
            name: "clan_getminkick",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_KICKUSER => OpcodeInfo {
            name: "clan_kickuser",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        CLAN_GETRANKS => OpcodeInfo {
            name: "clan_getranks",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_GETCHATCLANID => OpcodeInfo {
            name: "clan_getchatclanid",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CLAN_LEAVECHAT => OpcodeInfo {
            name: "clan_leavechat",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // 3700-3701 are clan channel existence checks in rev-647, not ignore ops
        IGNORE_COUNT => OpcodeInfo {
            name: "clanchannel_find",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IGNORE_GETNAME => OpcodeInfo {
            name: "clanchannel_find2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        IGNORE_TEST => OpcodeInfo {
            name: "ignore_test",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAT_GETFILTER_PUBLIC => OpcodeInfo {
            name: "chat_getfilter_public",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CHAT_SETFILTER => OpcodeInfo {
            name: "chat_setfilter",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        CHAT_SENDABUSEREPORT => OpcodeInfo {
            name: "chat_sendabusereport",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        CHAT_GETHISTORY_BYTYPEANDLINE => OpcodeInfo {
            name: "chat_gethistory_bytypeandline",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 3,
            pushes_string: 2,
        },
        CHAT_GETHISTORY_LENGTH => OpcodeInfo {
            name: "chat_gethistory_length",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        TEXT_GENDER => OpcodeInfo {
            name: "text_gender",
            pops_int: 1,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 1,
        },
        GETDEFAULTWINDOWMODE => OpcodeInfo {
            name: "getdefaultwindowmode",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        CAM_FORCEANGLE => OpcodeInfo {
            name: "cam_forceangle",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        VIEWPORT_SETFOV => OpcodeInfo {
            name: "viewport_setfov",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        PLAYER_SETBODYTYPE => OpcodeInfo {
            name: "player_setbodytype",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },

        // --- Complete rev-647 opcode metadata (from client source) ---
        // Core
        112 => OpcodeInfo {
            name: "push_clansetting_intvar",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        114 => OpcodeInfo {
            name: "push_clansetting_longvar",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        115 => OpcodeInfo {
            name: "push_clansetting_stringvar",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // Component creation/find
        151 => OpcodeInfo {
            name: "cc_delete",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        152 => OpcodeInfo {
            name: "cc_deleteall",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        202 => OpcodeInfo {
            name: "if_setzorder_front",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        203 => OpcodeInfo {
            name: "if_setzorder_back",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Player appearance
        404 => OpcodeInfo {
            name: "player_setcolour",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        410 => OpcodeInfo {
            name: "player_settitleoverride",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        411 => OpcodeInfo {
            name: "player_setwornobject",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // cc_set* (active component)
        1104 => OpcodeInfo {
            name: "cc_setlinewidth",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1122 => OpcodeInfo {
            name: "cc_setalpha",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1124 => OpcodeInfo {
            name: "cc_setlinertl",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1126 => OpcodeInfo {
            name: "cc_setmaxlines",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1127 => OpcodeInfo {
            name: "cc_setparam_int",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1128 => OpcodeInfo {
            name: "cc_setparam_string",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        1201 => OpcodeInfo {
            name: "cc_setnpcmodel",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1202 => OpcodeInfo {
            name: "cc_setplayermodel_self",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1208 => OpcodeInfo {
            name: "cc_setobject_alwaysnum",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1211 => OpcodeInfo {
            name: "cc_setplayermodel_snapshot",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1212 => OpcodeInfo {
            name: "cc_setobject_scaled",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1304 => OpcodeInfo {
            name: "cc_setdragdeadtime",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1308 => OpcodeInfo {
            name: "cc_settargetcursor",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1309 => OpcodeInfo {
            name: "cc_setcursor_op",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        1311 => OpcodeInfo {
            name: "cc_setpausetext_hash",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // cc_get* (active component)
        1610 => OpcodeInfo {
            name: "cc_getmodelxof",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1611 => OpcodeInfo {
            name: "cc_getmodelyof",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1612 => OpcodeInfo {
            name: "cc_getgraphic",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1613 => OpcodeInfo {
            name: "cc_getparam",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1614 => OpcodeInfo {
            name: "cc_get2dangle",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1800 => OpcodeInfo {
            name: "cc_gettargetmask",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        1802 => OpcodeInfo {
            name: "cc_getopbase",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // if_set* (by component hash, +1 pops_int vs cc_*)
        2005 => OpcodeInfo {
            name: "if_setnoclickthrough",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2106 => OpcodeInfo {
            name: "if_set2dangle",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2107 => OpcodeInfo {
            name: "if_settiling",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2115 => OpcodeInfo {
            name: "if_settextshadow",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2116 => OpcodeInfo {
            name: "if_setoutline",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2117 => OpcodeInfo {
            name: "if_setgraphicshadow",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2118 => OpcodeInfo {
            name: "if_setvflip",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2119 => OpcodeInfo {
            name: "if_sethflip",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2120 => OpcodeInfo {
            name: "if_setscrollsize",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2122 => OpcodeInfo {
            name: "if_setalpha",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2123 => OpcodeInfo {
            name: "if_setmodelzoom",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2124 => OpcodeInfo {
            name: "if_setlinertl",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2127 => OpcodeInfo {
            name: "if_setparam_int",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2131 => OpcodeInfo {
            name: "if_setmodel_recolour",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2132 => OpcodeInfo {
            name: "if_setmodel_retexture",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2201 => OpcodeInfo {
            name: "if_setnpcmodel",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2202 => OpcodeInfo {
            name: "if_setplayermodel_self",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2208 => OpcodeInfo {
            name: "if_setobject_alwaysnum",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2211 => OpcodeInfo {
            name: "if_setplayermodel_snapshot",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2212 => OpcodeInfo {
            name: "if_setobject_scaled",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2308 => OpcodeInfo {
            name: "if_settargetcursor",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2309 => OpcodeInfo {
            name: "if_setcursor_op",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        2310 => OpcodeInfo {
            name: "if_setpausetext",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        2314 => OpcodeInfo {
            name: "if_setmouseover_cursor",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // if_get* (by component hash)
        2605 => OpcodeInfo {
            name: "if_getmodelzoom",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2606 => OpcodeInfo {
            name: "if_getmodelangle_x",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2607 => OpcodeInfo {
            name: "if_getmodelangle_z",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2608 => OpcodeInfo {
            name: "if_getmodelangle_y",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2609 => OpcodeInfo {
            name: "if_gettrans",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2610 => OpcodeInfo {
            name: "if_getmodelxof",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2611 => OpcodeInfo {
            name: "if_getmodelyof",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2612 => OpcodeInfo {
            name: "if_getgraphic",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2702 => OpcodeInfo {
            name: "if_hasoverlay",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2703 => OpcodeInfo {
            name: "if_getchildcount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2704 | 2705 => OpcodeInfo {
            name: "if_hasoverlay_ver",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2800 => OpcodeInfo {
            name: "if_gettargetmask",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        2801 => OpcodeInfo {
            name: "if_getop",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // Server commands
        3111 => OpcodeInfo {
            name: "if_openoverlay",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3112 => OpcodeInfo {
            name: "if_closeoverlay",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3114 => OpcodeInfo {
            name: "tutorial_openchat",
            pops_int: 2,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3116 => OpcodeInfo {
            name: "resume_hsldialog",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3117 => OpcodeInfo {
            name: "resume_clanforumqfcdialog",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Sound
        3206 => OpcodeInfo {
            name: "sound_song_alt",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3209 => OpcodeInfo {
            name: "sound_song_alt2",
            pops_int: 5,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Game state
        3309 => OpcodeInfo {
            name: "coordx",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3310 => OpcodeInfo {
            name: "coordy",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3311 => OpcodeInfo {
            name: "coordz",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3313 => OpcodeInfo {
            name: "inv_getobj_alwaysnoted",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3318 => OpcodeInfo {
            name: "active_toplevelinterface",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3321 => OpcodeInfo {
            name: "headicon_count",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3322 => OpcodeInfo {
            name: "opcount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3323 => OpcodeInfo {
            name: "autosetup_isactive",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3324 => OpcodeInfo {
            name: "autosetup_getvalue",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3325 => OpcodeInfo {
            name: "is_logged_in",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3326 => OpcodeInfo {
            name: "player_getcomlevel",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3327 => OpcodeInfo {
            name: "player_isfemale",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3329 => OpcodeInfo {
            name: "is_quickchat_only",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3330 => OpcodeInfo {
            name: "player_getlevel_byxp",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3331 => OpcodeInfo {
            name: "inv_getnum_alwaysnoted",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3335 => OpcodeInfo {
            name: "getlanguage",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3336 => OpcodeInfo {
            name: "coordpack",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3337 => OpcodeInfo {
            name: "get_affiliateid",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3338 => OpcodeInfo {
            name: "get_fps",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3340 => OpcodeInfo {
            name: "has_focus",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3341 => OpcodeInfo {
            name: "frombilling",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3342 => OpcodeInfo {
            name: "mouse_getx",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3343 => OpcodeInfo {
            name: "mouse_gety",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Enum
        3409 => OpcodeInfo {
            name: "enum_contains_output",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Friends/ignore/clan
        3603 => OpcodeInfo {
            name: "friend_getrank",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3604 => OpcodeInfo {
            name: "friend_setrank",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3605 => OpcodeInfo {
            name: "friend_add",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3606 => OpcodeInfo {
            name: "friend_del",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3607 => OpcodeInfo {
            name: "ignore_add",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3608 => OpcodeInfo {
            name: "ignore_del",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3610 => OpcodeInfo {
            name: "friend_getprevname",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3611 => OpcodeInfo {
            name: "clan_getchatowner",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3612 => OpcodeInfo {
            name: "clan_getchatsize",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3613 => OpcodeInfo {
            name: "clan_getchatuserdisplay",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3614 => OpcodeInfo {
            name: "clan_getchatuserrankicon",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3615 => OpcodeInfo {
            name: "clan_getchatuserrank2",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3616 => OpcodeInfo {
            name: "clan_getminkick2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3617 => OpcodeInfo {
            name: "clan_joinchat",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3618 => OpcodeInfo {
            name: "clan_getranks2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3619 => OpcodeInfo {
            name: "clan_leavechat2",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        3620 => OpcodeInfo {
            name: "clan_getchatleavechat",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3621 => OpcodeInfo {
            name: "ignores_count",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3622 => OpcodeInfo {
            name: "ignores_getname",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 2,
        },
        3623 => OpcodeInfo {
            name: "ignores_test",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        3624 => OpcodeInfo {
            name: "clan_isself",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Clan channel
        3702 => OpcodeInfo {
            name: "clanchannel_getname",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3703 => OpcodeInfo {
            name: "clanchannel_isguest",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3704 => OpcodeInfo {
            name: "clanchannel_getbyte1",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3705 => OpcodeInfo {
            name: "clanchannel_getbyte2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3710 => OpcodeInfo {
            name: "clanchannel_getstring",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3711 => OpcodeInfo {
            name: "clanchannel_getbyte_indexed",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3712 => OpcodeInfo {
            name: "clanchannel_getint1",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3713 => OpcodeInfo {
            name: "clanchannel_getstring2",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3714 => OpcodeInfo {
            name: "clanchannel_lookup",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3715 => OpcodeInfo {
            name: "clanchannel_getint2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3717 => OpcodeInfo {
            name: "clanchannel_findname",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        3718 => OpcodeInfo {
            name: "clanchannel_getsortedindex",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3750 => OpcodeInfo {
            name: "clanchannel2_find",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3751 => OpcodeInfo {
            name: "clanchannel2_find_affined",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3752 => OpcodeInfo {
            name: "clanchannel2_getname",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3753 => OpcodeInfo {
            name: "clanchannel2_getrankkick",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3754 => OpcodeInfo {
            name: "clanchannel2_getranktalk",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3755 => OpcodeInfo {
            name: "clanchannel2_getusercount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3756 => OpcodeInfo {
            name: "clanchannel2_getusername",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        3757 => OpcodeInfo {
            name: "clanchannel2_getuserrank",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3758 => OpcodeInfo {
            name: "clanchannel2_getuserworld",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3759 => OpcodeInfo {
            name: "clanchannel2_kickuser",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        3760 => OpcodeInfo {
            name: "clanchannel2_findname",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        3761 => OpcodeInfo {
            name: "clanchannel2_getsortedindex",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Grand Exchange
        3903 => OpcodeInfo {
            name: "stockmarket_getoffertype",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3904 => OpcodeInfo {
            name: "stockmarket_getofferitem",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3905 => OpcodeInfo {
            name: "stockmarket_getofferprice",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3906 => OpcodeInfo {
            name: "stockmarket_getoffercount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3907 => OpcodeInfo {
            name: "stockmarket_getoffercompleted",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3908 => OpcodeInfo {
            name: "stockmarket_getoffergold",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3910 => OpcodeInfo {
            name: "stockmarket_isempty",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3912 => OpcodeInfo {
            name: "stockmarket_iscomplete",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        3913 => OpcodeInfo {
            name: "stockmarket_isbuying",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Arithmetic extras
        4019 => OpcodeInfo {
            name: "rare_table_roll",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4020 => OpcodeInfo {
            name: "hsv_to_rgb",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // String extras
        4111 => OpcodeInfo {
            name: "escape2",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 1,
        },
        4113 => OpcodeInfo {
            name: "char_isprintable2",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4114 => OpcodeInfo {
            name: "char_isalphanumeric2",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4116 => OpcodeInfo {
            name: "char_isnumeric2",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4126 => OpcodeInfo {
            name: "date_to_string",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        4127 => OpcodeInfo {
            name: "long_to_base36",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // OC/NC extras
        4211 => OpcodeInfo {
            name: "oc_search_next",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4300 => OpcodeInfo {
            name: "nc_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4500 => OpcodeInfo {
            name: "struct_param",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        4600 => OpcodeInfo {
            name: "bas_getidle",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Chat
        5005 => OpcodeInfo {
            name: "chat_getfilter_private",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5006 => OpcodeInfo {
            name: "chat_setmode",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5008 => OpcodeInfo {
            name: "chat_sendpublic",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        5009 => OpcodeInfo {
            name: "chat_sendprivate",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        5010 => OpcodeInfo {
            name: "player_getdisplayname",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5011 => OpcodeInfo {
            name: "chat_gethistory_name",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5012 => OpcodeInfo {
            name: "chat_gethistory_rank",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5016 => OpcodeInfo {
            name: "chat_getfilter_trade",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5017 => OpcodeInfo {
            name: "chat_getofflinepm",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5019 => OpcodeInfo {
            name: "chat_gethistory_message",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5024 => OpcodeInfo {
            name: "chat_gethistory_crown",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Quick chat
        5050 => OpcodeInfo {
            name: "quickchat_cat_getdesc",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5051 => OpcodeInfo {
            name: "quickchat_cat_getsubcatcount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5052 => OpcodeInfo {
            name: "quickchat_cat_getsubcat",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5053 => OpcodeInfo {
            name: "quickchat_cat_getphrasecount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5054 => OpcodeInfo {
            name: "quickchat_cat_getphrase",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5055 => OpcodeInfo {
            name: "quickchat_phrase_gettext",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5056 => OpcodeInfo {
            name: "quickchat_phrase_getautocount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5057 => OpcodeInfo {
            name: "quickchat_phrase_getautoresponse",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5058 => OpcodeInfo {
            name: "quickchat_phrase_init",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5059 => OpcodeInfo {
            name: "quickchat_sendpublic",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5060 => OpcodeInfo {
            name: "quickchat_sendprivate",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        5061 => OpcodeInfo {
            name: "quickchat_sendfriendsonly",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5062 => OpcodeInfo {
            name: "quickchat_cat_getsubcatshortcut",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5066 => OpcodeInfo {
            name: "quickchat_phrase_getcmdcount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5067 => OpcodeInfo {
            name: "quickchat_phrase_getcmdid",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5068 | 5069 => OpcodeInfo {
            name: "quickchat_phrase_setcmd",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5070 => OpcodeInfo {
            name: "quickchat_phrase_enumlookup",
            pops_int: 3,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5071 => OpcodeInfo {
            name: "quickchat_search",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        5072 => OpcodeInfo {
            name: "quickchat_search_next",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5074 | 5075 => OpcodeInfo {
            name: "quickchat_sendclan",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Key detection
        5101 => OpcodeInfo {
            name: "key_isdown_shift",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5102 => OpcodeInfo {
            name: "key_isdown_ctrl",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Minimap / worldmap
        5200 => OpcodeInfo {
            name: "minimap_setmark",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5205 => OpcodeInfo {
            name: "minimap_flash",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5206 => OpcodeInfo {
            name: "maparea_getbycoord",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5207 => OpcodeInfo {
            name: "maparea_getname",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        5208 | 5209 | 5212 | 5213 => OpcodeInfo {
            name: "worldmap_getxy",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        5214 | 5221 | 5230 => OpcodeInfo {
            name: "worldmap_setcoord",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5215 => OpcodeInfo {
            name: "worldmap_coord_hasarea",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5218 => OpcodeInfo {
            name: "maparea_getdefaultzoom",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5220 | 5229 | 5235 => OpcodeInfo {
            name: "worldmap_getstate",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5224 => OpcodeInfo {
            name: "worldmap_coord_to_display",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        5227 | 5231 => OpcodeInfo {
            name: "worldmap_setflag",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Display/screen
        5300 => OpcodeInfo {
            name: "fullscreen_enter",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5302 => OpcodeInfo {
            name: "screenmode_getcount",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5303 => OpcodeInfo {
            name: "screenmode_getresolution",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        5305 => OpcodeInfo {
            name: "screenmode_getcurrent",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5309 => OpcodeInfo {
            name: "setdefaultwindowmode",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // URL/misc
        5400 => OpcodeInfo {
            name: "url_request",
            pops_int: 1,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        5405 => OpcodeInfo {
            name: "cam_allocpath",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5406 => OpcodeInfo {
            name: "cam_addwaypoint",
            pops_int: 7,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5407 => OpcodeInfo {
            name: "cam_getpathcount",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5411 => OpcodeInfo {
            name: "logout_ex",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5420 => OpcodeInfo {
            name: "is_unsigned",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5421 => OpcodeInfo {
            name: "logout_url",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        5424 => OpcodeInfo {
            name: "hud_setsprites",
            pops_int: 11,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5426 | 5427 => OpcodeInfo {
            name: "hud_setoffsets",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5428 => OpcodeInfo {
            name: "hud_test",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5430 | 5431 => OpcodeInfo {
            name: "account_event",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5432 => OpcodeInfo {
            name: "clipboard_gettext",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // Camera
        5500 => OpcodeInfo {
            name: "cam_moveto",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5501 => OpcodeInfo {
            name: "cam_lookat",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5502 => OpcodeInfo {
            name: "cam_startpath",
            pops_int: 6,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5503 | 5507 | 5508 | 5509 | 5510 | 5512 => OpcodeInfo {
            name: "cam_clear",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5505 => OpcodeInfo {
            name: "cam_getangle_ya",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5506 => OpcodeInfo {
            name: "cam_getangle_xa",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5511 => OpcodeInfo {
            name: "cam_followcoord",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Login
        5600 => OpcodeInfo {
            name: "login_start",
            pops_int: 1,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        5601 | 5602 | 5606 | 5616 => OpcodeInfo {
            name: "login_void",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5626 => OpcodeInfo {
            name: "set_under_13",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5604 | 5619 => OpcodeInfo {
            name: "login_string",
            pops_int: 0,
            pops_string: 1,
            pushes_int: 0,
            pushes_string: 0,
        },
        5605 => OpcodeInfo {
            name: "login_createaccount",
            pops_int: 2,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        5607 | 5608 | 5609 | 5611 | 5613 | 5617 | 5623 => OpcodeInfo {
            name: "login_getint",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5625 => OpcodeInfo {
            name: "get_under_13",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        5612 => OpcodeInfo {
            name: "login_hopworld",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        5615 => OpcodeInfo {
            name: "login_start_reconnect",
            pops_int: 0,
            pops_string: 2,
            pushes_int: 0,
            pushes_string: 0,
        },
        5624 => OpcodeInfo {
            name: "login_getuserflow",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        // Client option setters (6000-series: 1 int pop)
        6001 | 6003 | 6005 | 6008 | 6010 | 6011 | 6012 | 6014 | 6015 | 6016 | 6017 | 6018
        | 6019 | 6020 | 6021 | 6023 | 6024 | 6027 | 6028 | 6029 | 6030 | 6031 | 6033 | 6034
        | 6035 | 6036 | 6038 | 6039 => OpcodeInfo {
            name: "set_clientopt",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        6032 => OpcodeInfo {
            name: "setdesiredtoolkit",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        // Client option getters (6100-series: 0 pop, 1 push)
        6101 | 6102 | 6103 | 6105 | 6108 | 6110 | 6111 | 6112 | 6114 | 6115 | 6116 | 6117
        | 6118 | 6119 | 6120 | 6123 | 6124 | 6129 | 6130 | 6131 | 6132 | 6133 | 6135 | 6136
        | 6139 | 6143 | 6144 | 6147 | 6148 => OpcodeInfo {
            name: "get_clientopt",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Viewport
        6201 => OpcodeInfo {
            name: "viewport_setfov2",
            pops_int: 2,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        6202 => OpcodeInfo {
            name: "viewport_setzoom",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        6203 => OpcodeInfo {
            name: "viewport_getmousecoord",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        // Date/time
        6300 => OpcodeInfo {
            name: "date_getminutes",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        6301 => OpcodeInfo {
            name: "date_getdays",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        6305 => OpcodeInfo {
            name: "date_split",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 3,
            pushes_string: 0,
        },
        6306 => OpcodeInfo {
            name: "date_tominutes",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Auth
        6405 | 6406 => OpcodeInfo {
            name: "has_auth",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Worldlist extras
        6503 => OpcodeInfo {
            name: "worldlist_hop",
            pops_int: 1,
            pops_string: 1,
            pushes_int: 1,
            pushes_string: 0,
        },
        6506 => OpcodeInfo {
            name: "worldlist_getbyid",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 4,
            pushes_string: 3,
        },
        6507 => OpcodeInfo {
            name: "worldlist_sort",
            pops_int: 4,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        6509 => OpcodeInfo {
            name: "worldlist_setfavorite",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        6510 => OpcodeInfo {
            name: "worldlist_getflags",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // MEL
        6800 => OpcodeInfo {
            name: "mel_gettext",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // Account/subscription
        6900 => OpcodeInfo {
            name: "is_subscriber",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        6901 => OpcodeInfo {
            name: "subscription_info",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 3,
            pushes_string: 0,
        },
        6902 | 6903 | 6904 | 6906 | 6907 | 6908 | 6910 | 6912 => OpcodeInfo {
            name: "get_accountinfo",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        6905 => OpcodeInfo {
            name: "get_clipboard",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 1,
        },
        // Auto setup
        7000 => OpcodeInfo {
            name: "autosetup_begin",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 2,
            pushes_string: 0,
        },
        7001..=7006 => OpcodeInfo {
            name: "autosetup_level",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
        7007 => OpcodeInfo {
            name: "autosetup_getvalue2",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        // Client option queries
        7206 | 7211 | 7214 => OpcodeInfo {
            name: "clientopt_isavailable",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },
        7306 | 7311 | 7314 => OpcodeInfo {
            name: "clientopt_canset",
            pops_int: 1,
            pops_string: 0,
            pushes_int: 1,
            pushes_string: 0,
        },

        // Default: unknown opcode
        _ => OpcodeInfo {
            name: "",
            pops_int: 0,
            pops_string: 0,
            pushes_int: 0,
            pushes_string: 0,
        },
    }
}

/// Returns `true` if the opcode is a branch instruction (conditional or unconditional).
pub fn is_branch(opcode: u16) -> bool {
    matches!(
        opcode,
        BRANCH
            | BRANCH_NOT
            | BRANCH_EQUALS
            | BRANCH_LESS_THAN
            | BRANCH_GREATER_THAN
            | BRANCH_LESS_THAN_OR_EQUALS
            | BRANCH_GREATER_THAN_OR_EQUALS
            | BRANCH_NOT_LONG
            | BRANCH_EQUALS_LONG
            | BRANCH_LESS_THAN_LONG
            | BRANCH_GREATER_THAN_LONG
            | BRANCH_LESS_THAN_OR_EQUALS_LONG
            | BRANCH_GREATER_THAN_OR_EQUALS_LONG
            | BRANCH_IF_TRUE
            | BRANCH_IF_FALSE
    )
}

/// Returns `true` if the opcode is a conditional branch (not unconditional BRANCH).
pub fn is_conditional_branch(opcode: u16) -> bool {
    is_branch(opcode) && opcode != BRANCH
}

/// For a comparison-branch opcode, return the textual comparison operator.
pub fn branch_operator(opcode: u16) -> &'static str {
    match opcode {
        BRANCH_EQUALS | BRANCH_EQUALS_LONG => "=",
        BRANCH_NOT | BRANCH_NOT_LONG => "!",
        BRANCH_LESS_THAN | BRANCH_LESS_THAN_LONG => "<",
        BRANCH_GREATER_THAN | BRANCH_GREATER_THAN_LONG => ">",
        BRANCH_LESS_THAN_OR_EQUALS | BRANCH_LESS_THAN_OR_EQUALS_LONG => "<=",
        BRANCH_GREATER_THAN_OR_EQUALS | BRANCH_GREATER_THAN_OR_EQUALS_LONG => ">=",
        BRANCH_IF_TRUE => "=",
        BRANCH_IF_FALSE => "=",
        _ => "??",
    }
}

/// Infix operators valid inside calc() — arithmetic only, NOT bitwise.
pub fn calc_operator(opcode: u16) -> Option<&'static str> {
    match opcode {
        ADD => Some("+"),
        SUB => Some("-"),
        MULTIPLY => Some("*"),
        DIV => Some("/"),
        MODULO => Some("%"),
        _ => None,
    }
}

/// Whether this opcode is arithmetic (valid inside calc).
pub fn is_calc_arithmetic(opcode: u16) -> bool {
    calc_operator(opcode).is_some()
}

/// Whether an expression tree contains nested arithmetic (needs calc wrapper).
pub fn needs_calc(expr: &Expr) -> bool {
    match expr {
        Expr::Operation { opcode, args } if is_calc_arithmetic(*opcode) => args
            .iter()
            .any(|a| matches!(a, Expr::Operation { opcode: op, .. } if is_calc_arithmetic(*op))),
        _ => false,
    }
}
