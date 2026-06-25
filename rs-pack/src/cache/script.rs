use rs_io::Packet;
use rustc_hash::FxHashMap;
use std::path::Path;
use std::sync::Arc;

pub struct ScriptProvider {
    pub names: FxHashMap<Box<str>, i32>,
    pub scripts: Box<[Option<Arc<Script>>]>,
    pub lookups: FxHashMap<i32, i32>,
}

impl ScriptProvider {
    pub fn from_bytes(dat_bytes: &[u8], idx_bytes: &[u8]) -> ScriptProvider {
        let mut dat = Packet::from(dat_bytes.to_vec());
        let mut idx = Packet::from(idx_bytes.to_vec());

        let count = dat.g4s() as usize;
        idx.pos += 4;

        dat.g4s(); // compiled version

        let mut names: FxHashMap<Box<str>, i32> =
            FxHashMap::with_capacity_and_hasher(count, Default::default());
        let mut scripts = vec![None; count];
        let mut lookups: FxHashMap<i32, i32> =
            FxHashMap::with_capacity_and_hasher(count, Default::default());

        for (index, slot) in scripts.iter_mut().enumerate() {
            let length = idx.g4s() as usize;
            if length == 0 {
                continue;
            }

            let id = index as i32;

            let start = dat.pos;
            let end = start + length;

            let script = Script::new(&mut dat, id, length);

            let info = &script.info;
            names.insert(info.name.clone(), id);
            if info.lookup != -1 {
                lookups.insert(info.lookup, id);
            }

            if dat.pos > end {
                panic!("Script {index} has read past end!");
            }

            dat.pos = end;

            *slot = Some(Arc::new(script));
        }

        ScriptProvider {
            names,
            scripts: Box::from(scripts),
            lookups,
        }
    }

    pub fn io(&mut self, dat_path: &Path, idx_path: &Path) -> anyhow::Result<()> {
        let dat = std::fs::read(dat_path)?;
        let idx = std::fs::read(idx_path)?;
        *self = Self::from_bytes(&dat, &idx);
        Ok(())
    }

    #[inline]
    pub fn get_by_id(&self, id: i32) -> Option<&Arc<Script>> {
        self.scripts.get(id as usize).and_then(|s| s.as_ref())
    }

    #[inline]
    pub fn get_by_lookup(&self, key: i32) -> Option<&Arc<Script>> {
        self.lookups.get(&key).and_then(|&id| self.get_by_id(id))
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Arc<Script>> {
        self.names.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.scripts.iter().filter(|s| s.is_some()).count()
    }
}

#[derive(Debug, Clone)]
pub struct ScriptInfo {
    pub name: Box<str>,
    pub path: Box<str>,
    pub lookup: i32,
    pub params: Box<[u8]>,
    pub pcs: Box<[i32]>,
    pub lines: Box<[i32]>,
}

impl ScriptInfo {
    pub fn line_number(&self, pc: i32) -> i32 {
        for (i, &pcs) in self.pcs.iter().enumerate() {
            if pcs > pc {
                return self.lines.get(i.wrapping_sub(1)).copied().unwrap_or(0);
            }
        }
        self.lines.last().copied().unwrap_or(0)
    }

    pub fn file_name(&self) -> &str {
        Path::new(&*self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&self.path)
    }
}

#[derive(Debug)]
pub struct Script {
    pub id: i32,
    pub int_arg_count: u16,
    pub string_arg_count: u16,
    pub int_local_count: u16,
    pub string_local_count: u16,
    pub opcodes: Box<[u16]>,
    pub int_operands: Box<[i32]>,
    pub string_operands: Box<[Box<str>]>,
    pub switch_tables: Box<[FxHashMap<i32, i32>]>,
    pub info: ScriptInfo,
}

impl Script {
    pub fn new(dat: &mut Packet, id: i32, length: usize) -> Self {
        let start = dat.pos;
        let end = start + length;

        if end < 16 {
            panic!("Invalid script file (minimum length) must be 16 bytes.");
        }

        dat.pos = end - 2;

        let trailer_len = dat.g2() as usize;
        let trailer_pos = end - trailer_len - 12 - 2;

        if trailer_pos >= end {
            panic!("Invalid script file (bad trailer pos).");
        }

        dat.pos = trailer_pos;

        let instructions = dat.g4s() as usize;
        let int_local_count = dat.g2();
        let string_local_count = dat.g2();
        let int_arg_count = dat.g2();
        let string_arg_count = dat.g2();
        let switch_count = dat.g1() as usize;

        let mut switch_tables: Vec<FxHashMap<i32, i32>> = Vec::with_capacity(switch_count);
        for _ in 0..switch_count {
            let count = dat.g2() as usize;
            let mut table: FxHashMap<i32, i32> =
                FxHashMap::with_capacity_and_hasher(count, Default::default());

            for _ in 0..count {
                table.insert(dat.g4s(), dat.g4s());
            }

            switch_tables.push(table);
        }

        dat.pos = start;
        let name = dat.gjstr(0);
        let path = dat.gjstr(0);
        let lookup = dat.g4s();

        let params_count = dat.g1() as usize;
        let mut params = vec![0; params_count];
        for param in params.iter_mut() {
            *param = dat.g1();
        }

        let lines_count = dat.g2() as usize;
        let mut pcs = vec![0; lines_count];
        let mut lines = vec![0; lines_count];
        for index in 0..lines_count {
            pcs[index] = dat.g4s();
            lines[index] = dat.g4s();
        }

        let info = ScriptInfo {
            name: name.into_boxed_str(),
            path: path.into_boxed_str(),
            lookup,
            params: params.into_boxed_slice(),
            pcs: pcs.into_boxed_slice(),
            lines: lines.into_boxed_slice(),
        };

        let mut opcodes: Vec<u16> = vec![0; instructions];
        let mut int_operands: Vec<i32> = vec![0; instructions];
        let empty: Box<str> = String::new().into_boxed_str();
        let mut string_operands: Vec<Box<str>> = vec![empty; instructions];

        let mut pc: usize = 0;
        while trailer_pos > dat.pos {
            let code = dat.g2();

            if code == PUSH_CONSTANT_STRING {
                string_operands[pc] = dat.gjstr(0).into_boxed_str();
            } else if is_large_operand(code) {
                int_operands[pc] = dat.g4s();
            } else {
                int_operands[pc] = dat.g1() as i32;
            }

            opcodes[pc] = code;
            pc += 1;
        }

        Script {
            id,
            int_arg_count,
            string_arg_count,
            int_local_count,
            string_local_count,
            opcodes: opcodes.into_boxed_slice(),
            int_operands: int_operands.into_boxed_slice(),
            string_operands: string_operands.into_boxed_slice(),
            switch_tables: switch_tables.into_boxed_slice(),
            info,
        }
    }
}

pub const PUSH_CONSTANT_INT: u16 = 0;
pub const PUSH_VARP: u16 = 1;
pub const POP_VARP: u16 = 2;
pub const PUSH_CONSTANT_STRING: u16 = 3;
pub const PUSH_VARN: u16 = 4;
pub const POP_VARN: u16 = 5;
pub const BRANCH: u16 = 6;
pub const BRANCH_NOT: u16 = 7;
pub const BRANCH_EQUALS: u16 = 8;
pub const BRANCH_LESS_THAN: u16 = 9;
pub const BRANCH_GREATER_THAN: u16 = 10;
pub const PUSH_VARS: u16 = 11;
pub const POP_VARS: u16 = 12;

pub const RETURN: u16 = 21;
pub const GOSUB: u16 = 22;
pub const JUMP: u16 = 23;
pub const SWITCH: u16 = 24;

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
pub const JUMP_WITH_PARAMS: u16 = 41;

pub const DEFINE_ARRAY: u16 = 44;
pub const PUSH_ARRAY_INT: u16 = 45;
pub const POP_ARRAY_INT: u16 = 46;

// ── Server ops (1003-1999) ──────────────────────────────────────────────────

pub const COORDX: u16 = 1000;
pub const COORDY: u16 = 1001;
pub const COORDZ: u16 = 1002;
pub const DISTANCE: u16 = 1003;
pub const INZONE: u16 = 1004;
pub const LINEOFSIGHT: u16 = 1005;
pub const LINEOFWALK: u16 = 1006;
pub const MAP_BLOCKED: u16 = 1007;
pub const MAP_CLOCK: u16 = 1008;
pub const MAP_FINDSQUARE: u16 = 1009;
pub const MAP_INDOORS: u16 = 1010;
pub const MAP_LIVE: u16 = 1011;
pub const MAP_LOCADDUNSAFE: u16 = 1012;
pub const MAP_MEMBERS: u16 = 1013;
pub const MAP_MULTIWAY: u16 = 1014;
pub const MAP_PLAYERCOUNT: u16 = 1015;
pub const MOVECOORD: u16 = 1016;
pub const PLAYERCOUNT: u16 = 1017;
pub const PROJANIM_MAP: u16 = 1018;
pub const SEQLENGTH: u16 = 1019;
pub const SPOTANIM_MAP: u16 = 1020;
pub const WORLD_DELAY: u16 = 1021;

// ── Player ops (2000-2499) ──────────────────────────────────────────────────

pub const AFK_EVENT: u16 = 2000;
pub const ALLOWDESIGN: u16 = 2001;
pub const ANIM: u16 = 2002;
pub const BOTH_HEROPOINTS: u16 = 2003;
pub const BUILDAPPEARANCE: u16 = 2004;
pub const BUSY: u16 = 2005;
pub const BUSY2: u16 = 2006;
pub const CAM_LOOKAT: u16 = 2007;
pub const CAM_MOVETO: u16 = 2008;
pub const CAM_RESET: u16 = 2009;
pub const CAM_SHAKE: u16 = 2010;
pub const CLEARQUEUE: u16 = 2011;
pub const CLEARSOFTTIMER: u16 = 2012;
pub const CLEARTIMER: u16 = 2013;
pub const COORD: u16 = 2014;
pub const DAMAGE: u16 = 2015;
pub const DISPLAYNAME: u16 = 2016;
pub const FACESQUARE: u16 = 2017;
pub const FINDHERO: u16 = 2018;
pub const FINDUID: u16 = 2019;
pub const GENDER: u16 = 2020;
pub const GETQUEUE: u16 = 2021;
pub const GETTIMER: u16 = 2022;
pub const GETWALKTRIGGER: u16 = 2023;
pub const HEADICONS_GET: u16 = 2024;
pub const HEADICONS_SET: u16 = 2025;
pub const HEALENERGY: u16 = 2026;
pub const HINT_COORD: u16 = 2027;
pub const HINT_NPC: u16 = 2028;
pub const HINT_PL: u16 = 2029;
pub const HINT_STOP: u16 = 2030;
pub const HUNTALL: u16 = 2031;
pub const HUNTNEXT: u16 = 2032;
pub const IF_CLOSE: u16 = 2033;
pub const IF_OPENCHAT: u16 = 2034;
pub const IF_OPENMAIN_SIDE: u16 = 2035;
pub const IF_OPENMAIN: u16 = 2036;
pub const IF_OPENSIDE: u16 = 2037;
pub const IF_SETANIM: u16 = 2038;
pub const IF_SETCOLOUR: u16 = 2039;
pub const IF_SETHIDE: u16 = 2040;
pub const IF_SETMODEL: u16 = 2041;
pub const IF_SETNPCHEAD: u16 = 2042;
pub const IF_SETOBJECT: u16 = 2043;
pub const IF_SETPLAYERHEAD: u16 = 2044;
pub const IF_SETPOSITION: u16 = 2045;
pub const IF_SETRECOL: u16 = 2046;
pub const IF_SETRESUMEBUTTONS: u16 = 2047;
pub const IF_SETTAB: u16 = 2048;
pub const IF_SETTABACTIVE: u16 = 2049;
pub const IF_SETTEXT: u16 = 2050;
pub const LAST_COM: u16 = 2051;
pub const LAST_INT: u16 = 2052;
pub const LAST_ITEM: u16 = 2053;
pub const LAST_LOGIN_INFO: u16 = 2054;
pub const LAST_SLOT: u16 = 2055;
pub const LAST_TARGETSLOT: u16 = 2056;
pub const LAST_USEITEM: u16 = 2057;
pub const LAST_USESLOT: u16 = 2058;
pub const LONGQUEUE: u16 = 2059;
pub const LONGQUEUEVARARG: u16 = 2060;
pub const LOWMEM: u16 = 2061;
pub const MES: u16 = 2062;
pub const MIDI_JINGLE: u16 = 2063;
pub const MIDI_SONG: u16 = 2064;
pub const NAME: u16 = 2065;
pub const P_ANIMPROTECT: u16 = 2066;
pub const P_APRANGE: u16 = 2067;
pub const P_ARRIVEDELAY: u16 = 2068;
pub const P_CLEARPENDINGACTION: u16 = 2069;
pub const P_COUNTDIALOG: u16 = 2070;
pub const P_DELAY: u16 = 2071;
pub const P_EXACTMOVE: u16 = 2072;
pub const P_FINDUID: u16 = 2073;
pub const P_LOCMERGE: u16 = 2074;
pub const P_LOGOUT: u16 = 2075;
pub const P_OPHELD: u16 = 2076;
pub const P_OPLOC: u16 = 2077;
pub const P_OPNPC: u16 = 2078;
pub const P_OPNPCT: u16 = 2079;
pub const P_OPOBJ: u16 = 2080;
pub const P_OPPLAYER: u16 = 2081;
pub const P_OPPLAYERT: u16 = 2082;
pub const P_PAUSEBUTTON: u16 = 2083;
pub const P_PREVENTLOGOUT: u16 = 2084;
pub const P_RUN: u16 = 2085;
pub const P_STOPACTION: u16 = 2086;
pub const P_TELEJUMP: u16 = 2087;
pub const P_TELEPORT: u16 = 2088;
pub const P_WALK: u16 = 2089;
pub const PLAYERMEMBER: u16 = 2090;
pub const PROJANIM_PL: u16 = 2091;
pub const QUEUE: u16 = 2092;
pub const QUEUEVARARG: u16 = 2093;
pub const READYANIM: u16 = 2094;
pub const RUNANIM: u16 = 2095;
pub const RUNENERGY: u16 = 2096;
pub const SAY: u16 = 2097;
pub const SESSION_LOG: u16 = 2098;
pub const SETGENDER: u16 = 2099;
pub const SETIDKIT: u16 = 2100;
pub const SETSKINCOLOUR: u16 = 2101;
pub const SETTIMER: u16 = 2102;
pub const SOFTTIMER: u16 = 2103;
pub const SOUND_SYNTH: u16 = 2104;
pub const SPOTANIM_PL: u16 = 2105;
pub const STAFFMODLEVEL: u16 = 2106;
pub const STAT_ADD: u16 = 2107;
pub const STAT_ADVANCE: u16 = 2108;
pub const STAT_BASE: u16 = 2109;
pub const STAT_BOOST: u16 = 2110;
pub const STAT_DRAIN: u16 = 2111;
pub const STAT_HEAL: u16 = 2112;
pub const STAT_RANDOM: u16 = 2113;
pub const STAT_SUB: u16 = 2114;
pub const STAT_TOTAL: u16 = 2115;
pub const STAT: u16 = 2116;
pub const STRONGQUEUE: u16 = 2117;
pub const STRONGQUEUEVARARG: u16 = 2118;
pub const TURNANIM: u16 = 2119;
pub const TUT_CLOSE: u16 = 2120;
pub const TUT_FLASH: u16 = 2121;
pub const TUT_OPEN: u16 = 2122;
pub const UID: u16 = 2123;
pub const WALKANIM_B: u16 = 2124;
pub const WALKANIM_L: u16 = 2125;
pub const WALKANIM_R: u16 = 2126;
pub const WALKANIM: u16 = 2127;
pub const WALKTRIGGER: u16 = 2128;
pub const WEAKQUEUE: u16 = 2129;
pub const WEAKQUEUEVARARG: u16 = 2130;
pub const WEALTH_EVENT: u16 = 2131;
pub const WEIGHT: u16 = 2132;
pub const SETIDKCOLOUR: u16 = 2133;
pub const BUFFER_FULL: u16 = 2134;
pub const LOWMEMORY: u16 = 2135; // TODO
pub const HINT_PLAYER: u16 = 2136; // TODO
pub const BAS_RUNNING: u16 = 2137; // TODO
pub const BAS_READYANIM: u16 = 2138; // TODO
pub const BAS_WALK_F: u16 = 2139; // TODO
pub const BAS_WALK_B: u16 = 2140; // TODO
pub const BAS_WALK_L: u16 = 2141; // TODO
pub const BAS_WALK_R: u16 = 2142; // TODO
pub const BAS_TURNONSPOT: u16 = 2143; // TODO

// ── Npc ops (2500-2999) ─────────────────────────────────────────────────────

pub const NPC_ADD: u16 = 2500;
pub const NPC_ANIM: u16 = 2501;
pub const NPC_ARRIVEDELAY: u16 = 2502;
pub const NPC_ATTACKRANGE: u16 = 2503;
pub const NPC_BASESTAT: u16 = 2504;
pub const NPC_CATEGORY: u16 = 2505;
pub const NPC_CHANGETYPE_KEEPALL: u16 = 2506;
pub const NPC_CHANGETYPE: u16 = 2507;
pub const NPC_COORD: u16 = 2508;
pub const NPC_DAMAGE: u16 = 2509;
pub const NPC_DEL: u16 = 2510;
pub const NPC_DELAY: u16 = 2511;
pub const NPC_FACESQUARE: u16 = 2512;
pub const NPC_FIND: u16 = 2513;
pub const NPC_FINDALL: u16 = 2514;
pub const NPC_FINDALLANY: u16 = 2515;
pub const NPC_FINDALLZONE: u16 = 2516;
pub const NPC_FINDCAT: u16 = 2517;
pub const NPC_FINDEXACT: u16 = 2518;
pub const NPC_FINDHERO: u16 = 2519;
pub const NPC_FINDNEXT: u16 = 2520;
pub const NPC_FINDUID: u16 = 2521;
pub const NPC_GETMODE: u16 = 2522;
pub const NPC_HASOP: u16 = 2523;
pub const NPC_HEROPOINTS: u16 = 2524;
pub const NPC_HUNT: u16 = 2525;
pub const NPC_HUNTALL: u16 = 2526;
pub const NPC_INRANGE: u16 = 2527;
pub const NPC_NAME: u16 = 2528;
pub const NPC_PARAM: u16 = 2529;
pub const NPC_QUEUE: u16 = 2530;
pub const NPC_RANGE: u16 = 2531;
pub const NPC_SAY: u16 = 2532;
pub const NPC_SETHUNT: u16 = 2533;
pub const NPC_SETHUNTMODE: u16 = 2534;
pub const NPC_SETMODE: u16 = 2535;
pub const NPC_SETTIMER: u16 = 2536;
pub const NPC_STAT: u16 = 2537;
pub const NPC_STATADD: u16 = 2538;
pub const NPC_STATHEAL: u16 = 2539;
pub const NPC_STATSUB: u16 = 2540;
pub const NPC_TELE: u16 = 2541;
pub const NPC_TYPE: u16 = 2542;
pub const NPC_UID: u16 = 2543;
pub const NPC_WALK: u16 = 2544;
pub const NPC_WALKTRIGGER: u16 = 2545;
pub const PROJANIM_NPC: u16 = 2546;
pub const SPOTANIM_NPC: u16 = 2547;
pub const NPC_DESTINATION: u16 = 2548;
pub const NPC_HUNTNEXT: u16 = 2549; // TODO

// ── Loc ops (3000-3499) ─────────────────────────────────────────────────────

pub const LOC_ADD: u16 = 3000;
pub const LOC_ANGLE: u16 = 3001;
pub const LOC_ANIM: u16 = 3002;
pub const LOC_CATEGORY: u16 = 3003;
pub const LOC_CHANGE: u16 = 3004;
pub const LOC_COORD: u16 = 3005;
pub const LOC_DEL: u16 = 3006;
pub const LOC_FIND: u16 = 3007;
pub const LOC_FINDALLZONE: u16 = 3008;
pub const LOC_FINDNEXT: u16 = 3009;
pub const LOC_NAME: u16 = 3010;
pub const LOC_PARAM: u16 = 3011;
pub const LOC_SHAPE: u16 = 3012;
pub const LOC_TYPE: u16 = 3013;

// ── Obj ops (3500-3999) ─────────────────────────────────────────────────────

pub const OBJ_ADD: u16 = 3500;
pub const OBJ_ADDALL: u16 = 3501;
pub const OBJ_COORD: u16 = 3502;
pub const OBJ_COUNT: u16 = 3503;
pub const OBJ_DEL: u16 = 3504;
pub const OBJ_FIND: u16 = 3505;
pub const OBJ_FINDALLZONE: u16 = 3506;
pub const OBJ_FINDNEXT: u16 = 3507;
pub const OBJ_NAME: u16 = 3508;
pub const OBJ_PARAM: u16 = 3509;
pub const OBJ_TAKEITEM: u16 = 3510;
pub const OBJ_TYPE: u16 = 3511;

// ── Npc config ops (4000-4099) ──────────────────────────────────────────────

pub const NC_CATEGORY: u16 = 4000;
pub const NC_DEBUGNAME: u16 = 4001;
pub const NC_DESC: u16 = 4002;
pub const NC_NAME: u16 = 4003;
pub const NC_OP: u16 = 4004;
pub const NC_PARAM: u16 = 4005;
pub const NC_SIZE: u16 = 4006;
pub const NC_VISLEVEL: u16 = 4007;

// ── Loc config ops (4100-4199) ──────────────────────────────────────────────

pub const LC_CATEGORY: u16 = 4100;
pub const LC_DEBUGNAME: u16 = 4101;
pub const LC_DESC: u16 = 4102;
pub const LC_LENGTH: u16 = 4103;
pub const LC_NAME: u16 = 4104;
pub const LC_OP: u16 = 4105;
pub const LC_PARAM: u16 = 4106;
pub const LC_WIDTH: u16 = 4107;

// ── Obj config ops (4200-4299) ──────────────────────────────────────────────

pub const OC_CATEGORY: u16 = 4200;
pub const OC_CERT: u16 = 4201;
pub const OC_COST: u16 = 4202;
pub const OC_DEBUGNAME: u16 = 4203;
pub const OC_DESC: u16 = 4204;
pub const OC_IOP: u16 = 4205;
pub const OC_MEMBERS: u16 = 4206;
pub const OC_NAME: u16 = 4207;
pub const OC_OP: u16 = 4208;
pub const OC_PARAM: u16 = 4209;
pub const OC_STACKABLE: u16 = 4210;
pub const OC_TRADEABLE: u16 = 4211;
pub const OC_UNCERT: u16 = 4212;
pub const OC_WEARPOS: u16 = 4213;
pub const OC_WEARPOS2: u16 = 4214;
pub const OC_WEARPOS3: u16 = 4215;
pub const OC_WEIGHT: u16 = 4216;

// ── Inventory ops (4300-4399) ───────────────────────────────────────────────

pub const BOTH_DROPSLOT: u16 = 4300;
pub const BOTH_MOVEINV: u16 = 4301;
pub const INV_ADD: u16 = 4302;
pub const INV_ALLSTOCK: u16 = 4303;
pub const INV_CHANGESLOT: u16 = 4304;
pub const INV_CLEAR: u16 = 4305;
pub const INV_DEBUGNAME: u16 = 4306;
pub const INV_DEL: u16 = 4307;
pub const INV_DELSLOT: u16 = 4308;
pub const INV_DROPALL: u16 = 4309;
pub const INV_DROPITEM_DELAYED: u16 = 4310;
pub const INV_DROPITEM: u16 = 4311;
pub const INV_DROPSLOT: u16 = 4312;
pub const INV_FREESPACE: u16 = 4313;
pub const INV_GETNUM: u16 = 4314;
pub const INV_GETOBJ: u16 = 4315;
pub const INV_ITEMSPACE: u16 = 4316;
pub const INV_ITEMSPACE2: u16 = 4317;
pub const INV_MOVEFROMSLOT: u16 = 4318;
pub const INV_MOVEITEM_CERT: u16 = 4319;
pub const INV_MOVEITEM_UNCERT: u16 = 4320;
pub const INV_MOVEITEM: u16 = 4321;
pub const INV_MOVETOSLOT: u16 = 4322;
pub const INV_SETSLOT: u16 = 4323;
pub const INV_SIZE: u16 = 4324;
pub const INV_STOCKBASE: u16 = 4325;
pub const INV_STOPTRANSMIT: u16 = 4326;
pub const INV_TOTAL: u16 = 4327;
pub const INV_TOTALCAT: u16 = 4328;
pub const INV_TOTALPARAM_STACK: u16 = 4329;
pub const INV_TOTALPARAM: u16 = 4330;
pub const INV_TRANSMIT: u16 = 4331;
pub const INVOTHER_TRANSMIT: u16 = 4332;

// ── Enum ops (4400-4499) ────────────────────────────────────────────────────

pub const ENUM: u16 = 4400;
pub const ENUM_GETOUTPUTCOUNT: u16 = 4401;

// ── String ops (4500-4599) ──────────────────────────────────────────────────

pub const APPEND_NUM: u16 = 4500;
pub const APPEND: u16 = 4501;
pub const APPEND_SIGNNUM: u16 = 4502;
pub const LOWERCASE: u16 = 4503;
pub const TEXT_GENDER: u16 = 4504;
pub const TOSTRING: u16 = 4505;
pub const COMPARE: u16 = 4506;
pub const TEXT_SWITCH: u16 = 4507;
pub const APPEND_CHAR: u16 = 4508;
pub const STRING_LENGTH: u16 = 4509;
pub const SUBSTRING: u16 = 4510;
pub const STRING_INDEXOF_CHAR: u16 = 4511;
pub const STRING_INDEXOF_STRING: u16 = 4512;
pub const SPLIT_GET: u16 = 4513;
pub const SPLIT_GETANIM: u16 = 4514;
pub const SPLIT_INIT: u16 = 4515;
pub const SPLIT_LINECOUNT: u16 = 4516;
pub const SPLIT_PAGECOUNT: u16 = 4517;

// ── Number ops (4600-4699) ──────────────────────────────────────────────────

pub const ADD: u16 = 4600;
pub const SUB: u16 = 4601;
pub const MULTIPLY: u16 = 4602;
pub const DIVIDE: u16 = 4603;
pub const RANDOM: u16 = 4604;
pub const RANDOMINC: u16 = 4605;
pub const INTERPOLATE: u16 = 4606;
pub const ADDPERCENT: u16 = 4607;
pub const SETBIT: u16 = 4608;
pub const CLEARBIT: u16 = 4609;
pub const TESTBIT: u16 = 4610;
pub const MODULO: u16 = 4611;
pub const POW: u16 = 4612;
pub const INVPOW: u16 = 4613;
pub const AND: u16 = 4614;
pub const OR: u16 = 4615;
pub const MIN: u16 = 4616;
pub const MAX: u16 = 4617;
pub const SCALE: u16 = 4618;
pub const BITCOUNT: u16 = 4619;
pub const TOGGLEBIT: u16 = 4620;
pub const SETBIT_RANGE: u16 = 4621;
pub const CLEARBIT_RANGE: u16 = 4622;
pub const GETBIT_RANGE: u16 = 4623;
pub const SETBIT_RANGE_TOINT: u16 = 4624;
pub const SIN_DEG: u16 = 4625;
pub const COS_DEG: u16 = 4626;
pub const ATAN2_DEG: u16 = 4627;
pub const ABS: u16 = 4628;

// ── Struct ops (4700) ───────────────────────────────────────────────────────

pub const STRUCT_PARAM: u16 = 4700;

// ── DB ops (7500-7510) ──────────────────────────────────────────────────────

pub const DB_FIND_WITH_COUNT: u16 = 7500;
pub const DB_FINDNEXT: u16 = 7501;
pub const DB_GETFIELD: u16 = 7502;
pub const DB_GETFIELDCOUNT: u16 = 7503;
pub const DB_LISTALL_WITH_COUNT: u16 = 7504;
pub const DB_GETROWTABLE: u16 = 7505;
pub const DB_FINDBYINDEX: u16 = 7506;
pub const DB_FIND_REFINE_WITH_COUNT: u16 = 7507;
pub const DB_FIND: u16 = 7508;
pub const DB_FIND_REFINE: u16 = 7509;
pub const DB_LISTALL: u16 = 7510;

// ── Debug ops (10000-10003) ─────────────────────────────────────────────────

pub const CONSOLE: u16 = 10000;
pub const ERROR: u16 = 10001;
pub const GETTIMESPENT: u16 = 10002;
pub const TIMESPENT: u16 = 10003;
pub const NPCCOUNT: u16 = 10004;
pub const ZONECOUNT: u16 = 10005;
pub const LOCCOUNT: u16 = 10006;
pub const MAP_LASTCLOCK: u16 = 10007;
pub const MAP_LASTWORLD: u16 = 10008;
pub const MAP_LASTCLIENTIN: u16 = 10009;
pub const MAP_LASTNPC: u16 = 10010;
pub const MAP_LASTPLAYER: u16 = 10011;
pub const MAP_LASTLOGIN: u16 = 10012;
pub const MAP_LASTLOGOUT: u16 = 10013;
pub const MAP_LASTZONE: u16 = 10014;
pub const MAP_LASTCLIENTOUT: u16 = 10015;
pub const MAP_LAST_CLEANUP: u16 = 10016;
pub const MAP_LAST_BANDWIDTHIN: u16 = 10017;
pub const MAP_LAST_BANDWIDTHOUT: u16 = 10018;
pub const MAP_PRODUCTION: u16 = 10019; // TODO.
pub const OBJCOUNT: u16 = 10020;

// ── DO NOT ADD ANY OPCODES AFTER THIS ONE (11000) ─────────────────────────────────────────────────

pub const LAST: u16 = 11000;

// ── Helpers ─────────────────────────────────────────────────────────────────

pub fn is_large_operand(opcode: u16) -> bool {
    if opcode > 100 {
        return false;
    }
    !matches!(
        opcode,
        RETURN | GOSUB | JUMP | POP_INT_DISCARD | POP_STRING_DISCARD
    )
}

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
    )
}
