use crate::cache::CacheStore;
use crate::cache::area::{AreaMap, AreaType, MULTIWAY};
use crate::cache::category::CategoryType;
use crate::cache::controller::ControllerType;
use crate::cache::dbrow::DbRowType;
use crate::cache::dbtable::{DbTableIndex, DbTableType};
use crate::cache::r#enum::{EnumType, EnumTypeRaw};
use crate::cache::flo::FloTypeRaw;
use crate::cache::font::FontTypeProvider;
use crate::cache::hunt::HuntType;
use crate::cache::idk::IdkTypeRaw;
use crate::cache::r#if::IfTypeProvider;
use crate::cache::inv::InvTypeRaw;
use crate::cache::js5::PackedCache;
use crate::cache::loader;
use crate::cache::loc::{LocType, LocTypeRaw};
use crate::cache::map::XteaKeyStore;
use crate::cache::mesanim::MesAnimType;
use crate::cache::midi::{self, MidiProvider};
use crate::cache::npc::NpcTypeRaw;
use crate::cache::obj::{ObjContext, ObjTypeRaw};
use crate::cache::param::{ParamType, ParamTypeRaw};
use crate::cache::provider::{CacheType, TypeProvider};
use crate::cache::script::ScriptProvider;
use crate::cache::seq::SeqTypeRaw;
use crate::cache::spotanim::SpotAnimTypeRaw;
use crate::cache::r#struct::StructTypeRaw;
use crate::cache::varbit::VarbitType;
use crate::cache::varc::VarClientTypeProvider;
use crate::cache::varcon::VarConType;
use crate::cache::varn::VarnType;
use crate::cache::varobj::VarObjType;
use crate::cache::varp::VarPlayerTypeRaw;
use crate::cache::vars::VarsType;
use crate::cache::wordenc::WordEncProvider;
use crate::pack::incremental::{Stage, files_under};
use crate::pack::names;
use crate::pack::other::map;
use crate::pack::pack;
use crate::pack::pack_registry::PackedFile;
use crate::pack::packed_data::PackedData;
use crate::types::ParamValue;
use crate::zone_csv::load_csv_zones;
use crate::{archive, compile_scripts, unwrap_thread};
use anyhow::{Context, Result};
use runec::symbol::identifier_form;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;
use std::{fs, thread};
use tracing::info;

pub struct PackResult {
    pub store: Box<CacheStore>,
    pub scripts: ScriptProvider,
    pub layout: InterfaceLayout,
    pub cache: PackedCache,
}

/// Well-known component indices per toplevel, derived from interface.pack.
///
/// Allows the engine to resolve `if_openmain` etc. to the correct
/// `if_opensub` parent component without hardcoded constants.
#[derive(Debug, Clone, Default)]
pub struct InterfaceLayout {
    pub main_screen: HashMap<u16, u16>,
    pub wide_screen: HashMap<u16, u16>,
    pub wide_screen_ifaces: HashSet<u16>,
    pub overlay_tab: HashMap<u16, u16>,
    pub overlay: HashMap<u16, u16>,
    pub chatbox: HashMap<u16, u16>,
    pub component_roots: HashMap<u16, i32>,
    pub component_names: HashMap<i32, Box<str>>,
    pub dialogue_parent_hash: i32,
    pub dialogue_slots: HashMap<u16, i32>,
    pub count_dialog: i32,
    pub world_map: u16,
    pub switch_side: u16,
    pub design: [u16; 12],
}

impl InterfaceLayout {
    /// Chatbox child component hash an interface mounts on when opened as a
    /// chat dialogue: its `dialogue_slots` override, else the default slot.
    pub fn dialogue_slot(&self, iface: u16) -> i32 {
        self.dialogue_slots
            .get(&iface)
            .copied()
            .unwrap_or(self.dialogue_parent_hash)
    }

    /// Component `com`'s name as scripts write it (`interface:component`), or
    /// its ids when interface.pack has none.
    pub fn component_name(&self, com: i32) -> Cow<'_, str> {
        match self.component_names.get(&com) {
            Some(name) => Cow::Borrowed(name),
            None => Cow::Owned(format!("{}:{}", (com as u32) >> 16, com & 0xFFFF)),
        }
    }
}

pub fn pack_all(
    content_dir: &Path,
    pack_dir: &Path,
    verify: bool,
    members: bool,
) -> Result<PackResult> {
    let start = Instant::now();
    // Checking config names registers new ones, so everything that reads the
    // registries starts after it, from names loaded once.
    let config_files = pack::ConfigFiles::new(&content_dir.join("scripts"));
    pack::check_configs(&config_files, pack_dir, verify)?;
    let asset_names = names::AssetNames::load(pack_dir)?;
    let (packed, scripts) = thread::scope(|scope| {
        let scripts = scope.spawn(|| compile_scripts(content_dir, pack_dir));
        let packed = pack::pack_cache(content_dir, pack_dir, verify, &config_files, &asset_names);
        (packed, unwrap_thread("scripts", scripts.join()))
    });
    let (assets, cache) = packed?;
    let scripts = scripts?;
    info!(
        "Assets and scripts packed in {:.2}s",
        start.elapsed().as_secs_f64()
    );
    archive::verify_crcs(&cache, verify)?;
    // A strict gate that passed needs no rerun until its inputs change; a
    // lenient one runs every pack, to keep showing what it warns about.
    if verify {
        let gate = Stage::new("name gate", Vec::new());
        let mut sources = files_under(&[pack_dir.to_path_buf()]);
        sources.push(content_dir.join("versions/interfaces.fileids"));
        gate.unit(0, &sources, || {
            names::verify_names(content_dir, pack_dir, true).map(|()| Vec::new())
        })?;
        gate.finish(Vec::new());
    } else {
        names::verify_names(content_dir, pack_dir, false)?;
    }
    let map_dirs = map::source_dirs(content_dir);
    let spawns = map::pack_spawns(&map_dirs, &asset_names)?;
    let obj_spawns = map::pack_obj_spawns(&map_dirs, &asset_names)?;
    let area_map = map::pack_area_map(&map_dirs, pack_dir)?;
    let store = load_cache(
        &assets,
        &cache,
        content_dir,
        pack_dir,
        &spawns,
        &obj_spawns,
        area_map,
        members,
    )?;
    let mut layout = load_interface_layout(pack_dir);
    load_dialogue_slots(pack_dir, &store.enums, &mut layout);
    load_wide_screen_ifaces(pack_dir, &store.enums, &mut layout);
    layout.count_dialog = asset_names.id("clientscript", COUNT_DIALOG)? as i32;
    layout.switch_side = asset_names.id("varc", SWITCH_SIDE)? as u16;
    for (varc, name) in layout.design.iter_mut().zip(DESIGN) {
        *varc = asset_names.id("varc", name)? as u16;
    }
    info!(
        "Pack complete in {:.2}s: {} groups, {} scripts",
        start.elapsed().as_secs_f64(),
        cache.groups.len(),
        scripts.count()
    );
    Ok(PackResult {
        store,
        scripts,
        layout,
        cache,
    })
}

fn load_cache(
    assets: &HashMap<String, PackedFile>,
    cache: &PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    spawns: &PackedData,
    obj_spawns: &PackedData,
    area_map: AreaMap,
    members: bool,
) -> Result<Box<CacheStore>> {
    let npcs = load_config::<NpcTypeRaw, _>(assets, "npc", ())?;
    let params: TypeProvider<ParamType> = load_config::<ParamTypeRaw, _>(assets, "param", ())?;
    let objs = load_config::<ObjTypeRaw, _>(
        assets,
        "obj",
        ObjContext {
            members,
            autodisable_params: params.types.iter().map(|p| p.autodisable).collect(),
        },
    )?;
    let seqs = load_config::<SeqTypeRaw, _>(assets, "seq", ())?;
    let enums = load_config::<EnumTypeRaw, _>(assets, "enum", ())?;
    let structs = load_config::<StructTypeRaw, _>(assets, "struct", ())?;
    let varbits = load_config::<VarbitType, _>(assets, "varpbit", ())?;
    let varps = load_config::<VarPlayerTypeRaw, _>(assets, "varp", ())?;
    let varcs = VarClientTypeProvider::from_indexed_bytes(
        client_config(assets, "varc")?,
        client_config(assets, "varcstr")?,
    )?;
    let idks = load_config::<IdkTypeRaw, _>(assets, "idk", ())?;
    let hunts = load_config::<HuntType, _>(assets, "hunt", ())?;
    let flos = load_config::<FloTypeRaw, _>(assets, "flo", ())?;
    let spotanims = load_config::<SpotAnimTypeRaw, _>(assets, "spotanim", ())?;
    let invs = load_config::<InvTypeRaw, _>(assets, "inv", ())?;
    let mesanims = load_config::<MesAnimType, _>(assets, "mesanim", ())?;
    let varns = load_config::<VarnType, _>(assets, "varn", ())?;
    let varss = load_config::<VarsType, _>(assets, "vars", ())?;
    let dbtables = load_config::<DbTableType, _>(assets, "dbtable", ())?;
    let dbrows = load_config::<DbRowType, _>(assets, "dbrow", ())?;
    let categories = load_config::<CategoryType, _>(assets, "category", ())?;
    let areas: TypeProvider<AreaType> = load_config::<AreaType, _>(assets, "area", ())?;
    let varobjs: TypeProvider<VarObjType> = load_config::<VarObjType, _>(assets, "varobj", ())?;
    let varcons = load_config::<VarConType, _>(assets, "varcon", ())?;
    let varconbits = load_config::<VarbitType, _>(assets, "varconbit", ())?;
    let controllers = load_config::<ControllerType, _>(assets, "controller", ())?;

    let locs = load_config::<LocTypeRaw, LocType>(assets, "loc", ())?;

    let keys = XteaKeyStore::load(&content_dir.join("keys.json"))?;
    let (mut mapsquares, mapcrcs) = loader::load_mapsquares(cache, &keys)?;
    let huffman = loader::load_huffman(cache)?;
    let fonts = FontTypeProvider::from_cache(cache);
    loader::merge_server_layer(spawns, "map", 'n', &mut mapsquares);
    loader::merge_server_layer(obj_spawns, "objspawn", 'o', &mut mapsquares);

    let db_index = DbTableIndex::build(&dbtables, &dbrows);
    Ok(Box::new(CacheStore {
        mapsquares,
        mapcrcs,
        objs,
        invs,
        varps,
        varcs,
        varbits,
        dbrows,
        dbtables,
        db_index,
        enums,
        flos,
        hunts,
        idks,
        locs,
        mesanims,
        npcs,
        params,
        seqs,
        spotanims,
        structs,
        varns,
        varss,
        categories,
        interfaces: IfTypeProvider::from_cache(cache)?,
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
        midi_ids: midi::load_midi_ids(pack_dir),
        midi_tick_lengths: midi::load_midi_tick_lengths(cache),
        freemap: load_csv_zones(&content_dir.join("maps/free2play.csv")),
        huffman,
        multiway_area: areas.get_by_debugname(MULTIWAY).map(|area| area.id),
        areas,
        area_map,
        varobjs,
        varcons,
        varconbits,
        controllers,
    }))
}

/// Rev-647 chatbox child component that hosts dialogue_box interfaces
/// (npc/player chat, message box, multi-choice): `chattop:chatmodallarge`,
/// void's `dialogue_box parent="chat_box" index=13`.
const DIALOGUE_BOX_INDEX: i32 = 13;

const CHATBOX_INTERFACE: &str = "chattop";

const WORLD_MAP_INTERFACE: &str = "worldmap";

/// The varc the client keeps its selected side tab in.
const SWITCH_SIDE: &str = "if_switchside_varc";

/// The varcs of the character design screen, in the order `applydesign`
/// reads them: seven body kits, then five colours.
const DESIGN: [&str; 12] = [
    "playerdesign3_head",
    "playerdesign3_jaw",
    "playerdesign3_torso",
    "playerdesign3_arms",
    "playerdesign3_hands",
    "playerdesign3_legs",
    "playerdesign3_feet",
    "playerdesign3_haircol",
    "playerdesign3_torsocol",
    "playerdesign3_legscol",
    "playerdesign3_feetcol",
    "playerdesign3_skincol",
];

/// The clientscript `p_countdialog` runs to ask for a number in the chat box.
const COUNT_DIALOG: &str = "[clientscript,meslayer_mode7]";

/// The layout slot a component of the fixed (`toplevel`, 548) or resizable
/// (`toplevel_fullscreen`, 746) toplevel fills, by Jagex's names.
fn toplevel_slot<'a>(
    layout: &'a mut InterfaceLayout,
    interface: &str,
    component: &str,
) -> Option<&'a mut HashMap<u16, u16>> {
    Some(match (interface, component) {
        ("toplevel" | "toplevel_fullscreen", "mainmodal") => &mut layout.main_screen,
        ("toplevel", "mainoverlay_clipped") | ("toplevel_fullscreen", "mainmodal_fullscreen") => {
            &mut layout.wide_screen
        }
        ("toplevel" | "toplevel_fullscreen", "mainoverlay") => &mut layout.overlay,
        ("toplevel" | "toplevel_fullscreen", "sidemodal") => &mut layout.overlay_tab,
        ("toplevel" | "toplevel_fullscreen", "chattop") => &mut layout.chatbox,
        _ => return None,
    })
}

/// Build an [`InterfaceLayout`] from interface.pack: the toplevels' well-known
/// slots and every component's name.
fn load_interface_layout(pack_dir: &Path) -> InterfaceLayout {
    let mut layout = InterfaceLayout::default();
    let path = pack_dir.join("interface.pack");
    let Ok(contents) = fs::read_to_string(&path) else {
        return layout;
    };
    let entries: Vec<(&str, &str)> = contents
        .lines()
        .filter_map(|line| line.trim().split_once('='))
        .collect();
    let interfaces: HashMap<&str, &str> = entries
        .iter()
        .filter(|(id, _)| !id.contains(':'))
        .copied()
        .collect();
    for &(id_str, name) in &entries {
        let Some((iface_str, comp_str)) = id_str.split_once(':') else {
            // Top-level interface line `<id>=<name>` (no component). Capture the
            // chatbox interface id so dialogues can mount on its dialogue_box
            // child instead of clobbering the chatbox at `toplevel:chattop`.
            if let Ok(iface) = id_str.parse::<u16>() {
                match name {
                    CHATBOX_INTERFACE => {
                        layout.dialogue_parent_hash = ((iface as i32) << 16) | DIALOGUE_BOX_INDEX;
                    }
                    WORLD_MAP_INTERFACE => layout.world_map = iface,
                    _ => {}
                }
            }
            continue;
        };
        let (Ok(iface), Ok(comp)) = (iface_str.parse::<u16>(), comp_str.parse::<u16>()) else {
            continue;
        };
        let interface = interfaces.get(iface_str).copied().unwrap_or_default();
        if let Some(slot) = toplevel_slot(&mut layout, interface, name) {
            slot.insert(iface, comp);
        }
        layout.component_roots.insert(comp, iface as i32);
        layout.component_names.insert(
            ((iface as i32) << 16) | comp as i32,
            format!("{}:{}", identifier_form(interface), identifier_form(name)).into(),
        );
    }
    layout
}

/// Populate `layout.dialogue_slots` from the content `dialogue_slot_enum`.
///
/// The enum maps a dialogue interface id (key) → the chatbox child component
/// hash it mounts on (value), e.g. objbox/doubleobjbox →
/// `chattop:chatmodal` (752:12). The value is already a full
/// component hash (resolved symbolically at pack time), so it is the parent the
/// engine opens against — no slot-index math. The default `dialogue_box` slot
/// (752:13) is implicit; only exceptions appear in the enum. It's a server-only
/// enum, so its id is resolved from `enum.pack` by name and read out of the
/// already-loaded enum TypeProvider. Absent/empty enum → no overrides.
fn load_dialogue_slots(
    pack_dir: &Path,
    enums: &TypeProvider<EnumType>,
    layout: &mut InterfaceLayout,
) {
    const ENUM_NAME: &str = "dialogue_slot_enum";
    let Ok(contents) = fs::read_to_string(pack_dir.join("enum.pack")) else {
        return;
    };
    let enum_id = contents.lines().find_map(|line| {
        let (id, name) = line.trim().split_once('=')?;
        (name == ENUM_NAME)
            .then(|| id.parse::<u16>().ok())
            .flatten()
    });
    let Some(enum_id) = enum_id else { return };
    let Some(def) = enums.get_by_id(enum_id) else {
        return;
    };
    for (&iface, value) in &def.values {
        if let ParamValue::Int(parent_hash) = value {
            layout.dialogue_slots.insert(iface as u16, *parent_hash);
        }
    }
    info!(
        "  dialogue slots: {} override(s) from {ENUM_NAME}",
        layout.dialogue_slots.len()
    );
}

/// Populate `layout.wide_screen_ifaces` from the content `wide_screen_enum`:
/// interface id (key) → true for each main-screen interface the content opens
/// in the toplevel's full-size slot (void-634's `wide_screen` interfaces and
/// the level-up details). Absent/empty enum → none.
fn load_wide_screen_ifaces(
    pack_dir: &Path,
    enums: &TypeProvider<EnumType>,
    layout: &mut InterfaceLayout,
) {
    const ENUM_NAME: &str = "wide_screen_enum";
    let Ok(contents) = fs::read_to_string(pack_dir.join("enum.pack")) else {
        return;
    };
    let Some(def) = contents
        .lines()
        .find_map(|line| {
            let (id, name) = line.trim().split_once('=')?;
            (name == ENUM_NAME)
                .then(|| id.parse::<u16>().ok())
                .flatten()
        })
        .and_then(|id| enums.get_by_id(id))
    else {
        return;
    };
    for (&iface, value) in &def.values {
        if let ParamValue::Int(1) = value {
            layout.wide_screen_ifaces.insert(iface as u16);
        }
    }
}

fn client_config<'a>(
    assets: &'a HashMap<String, PackedFile>,
    name: &str,
) -> Result<&'a PackedData> {
    assets
        .get(name)
        .and_then(|asset| asset.client.as_ref())
        .with_context(|| format!("Missing {name} client config"))
}

fn load_config<Raw: CacheType, Stored: From<Raw>>(
    assets: &HashMap<String, PackedFile>,
    name: &str,
    ctx: Raw::Context,
) -> Result<TypeProvider<Stored>> {
    let data = &assets
        .get(name)
        .with_context(|| format!("Missing {name} config"))?
        .server;
    TypeProvider::from_indexed_bytes::<Raw>(&data.dat, &data.idx, ctx)
}
