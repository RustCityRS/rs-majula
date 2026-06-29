pub mod cache;
pub mod pack;
pub mod types;
pub mod unpack;
#[cfg(since_244)]
pub mod versionlist;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::pack::pack_registry::PackRegistry;
use cache::CacheStore;
use cache::category::CategoryType;
use cache::dbrow::DbRowType;
use cache::dbtable::{DbTableIndex, DbTableType};
use cache::r#enum::EnumType;
use cache::flo::FloType;
use cache::font::FontTypeProvider;
use cache::hunt::HuntType;
use cache::idk::{IdkType, IdkTypeRaw};
use cache::r#if::IfTypeProvider;
use cache::inv::InvType;
use cache::loc::{LocType, LocTypeRaw};
use cache::mesanim::MesAnimType;
use cache::midi::MidiProvider;
use cache::npc::{NpcType, NpcTypeRaw};
use cache::obj::{ObjContext, ObjType, ObjTypeRaw};
use cache::param::ParamType;
use cache::provider::TypeProvider;
use cache::seq::{SeqType, SeqTypeRaw};
use cache::spotanim::{SpotAnimType, SpotAnimTypeRaw};
use cache::r#struct::StructType;
#[cfg(since_254)]
use cache::varbit::VarbitType;
use cache::varn::VarnType;
use cache::varp::VarPlayerType;
use cache::vars::VarsType;
use cache::wordenc::WordEncProvider;
#[cfg(since_244)]
use pack::ondemand::{OndemandArtifacts, build_ondemand_artifacts};
use pack::other;
use rs_io::crc;
use rs_io::jag::{JagCompression, JagFile};
use tracing::debug;
#[cfg(since_244)]
use types::OndemandBlobs;
pub use types::ParamValue;
#[cfg(since_244)]
use unpack::model::load_existing_pack;

macro_rules! rev_content_path {
    ($suffix:literal) => {
        concat!("content/", env!("REV"), $suffix)
    };
}

pub const CONTENT_DIR: &str = rev_content_path!("");
pub const PACK_DIR: &str = rev_content_path!("/pack");

#[cfg(rev = "225")]
pub(crate) mod jag_crc {
    pub const TITLE: Option<i32> = Some(-430779560);
    pub const CONFIG: Option<i32> = Some(511217062);
    pub const INTERFACE: Option<i32> = Some(1614084464);
    pub const MEDIA: Option<i32> = Some(-343404987);
    pub const MODELS: Option<i32> = Some(-2000991154);
    pub const TEXTURES: Option<i32> = Some(1703545114);
    pub const WORDENC: Option<i32> = Some(1570981179);
    pub const SOUNDS: Option<i32> = Some(-1532605973);
}

#[cfg(rev = "244")]
pub(crate) mod jag_crc {
    pub const TITLE: Option<i32> = Some(126707642);
    pub const CONFIG: Option<i32> = Some(1573679574);
    pub const INTERFACE: Option<i32> = Some(2074207176);
    pub const MEDIA: Option<i32> = Some(-151945349);
    pub const MODELS: Option<i32> = None;
    pub const TEXTURES: Option<i32> = Some(245278618);
    pub const WORDENC: Option<i32> = Some(-87627495);
    pub const SOUNDS: Option<i32> = Some(-855112082);
    pub const VERSIONLIST: Option<i32> = Some(-390182005);
}

#[cfg(rev = "245.2")]
pub(crate) mod jag_crc {
    pub const TITLE: Option<i32> = Some(126707642);
    pub const CONFIG: Option<i32> = Some(219495412);
    pub const INTERFACE: Option<i32> = Some(1539972921);
    pub const MEDIA: Option<i32> = Some(353992155);
    pub const MODELS: Option<i32> = None;
    pub const TEXTURES: Option<i32> = Some(-1885459577);
    pub const WORDENC: Option<i32> = Some(-87627495);
    pub const SOUNDS: Option<i32> = Some(-1625923170);
    pub const VERSIONLIST: Option<i32> = Some(-1979342254);
}

#[cfg(rev = "254")]
pub(crate) mod jag_crc {
    pub const TITLE: Option<i32> = Some(1187152444);
    pub const CONFIG: Option<i32> = Some(1524313696);
    pub const INTERFACE: Option<i32> = Some(531876099);
    pub const MEDIA: Option<i32> = Some(-374324307);
    pub const MODELS: Option<i32> = None;
    pub const TEXTURES: Option<i32> = Some(-1826159457);
    pub const WORDENC: Option<i32> = Some(1385372455);
    pub const SOUNDS: Option<i32> = Some(392114586);
    pub const VERSIONLIST: Option<i32> = Some(-128580638);
}

#[cfg(rev = "274")]
pub(crate) mod jag_crc {
    pub const TITLE: Option<i32> = Some(410306098);
    pub const CONFIG: Option<i32> = Some(-433051697);
    pub const INTERFACE: Option<i32> = Some(2135735991);
    pub const MEDIA: Option<i32> = Some(1861649167);
    pub const MODELS: Option<i32> = None;
    pub const TEXTURES: Option<i32> = Some(915347346);
    pub const WORDENC: Option<i32> = Some(1386621111);
    pub const SOUNDS: Option<i32> = Some(-759577225);
    pub const VERSIONLIST: Option<i32> = Some(-322040827);
}

#[cfg(rev = "225")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = 1638136604;
    pub const LOC: i32 = 891497087;
    pub const FLO: i32 = 1976597026;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = 705633567;
    pub const NPC: i32 = -2140681882;
    pub const OBJ: i32 = -840233510;
    pub const SPOTANIM: i32 = -1279835623;
    pub const INTERFACE: i32 = -2146838800;
}

#[cfg(rev = "244")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = 1405403166;
    pub const LOC: i32 = 1195428820;
    pub const FLO: i32 = 1976597026;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = -1961744050;
    pub const NPC: i32 = -997428438;
    pub const OBJ: i32 = 1589810970;
    pub const SPOTANIM: i32 = 117013845;
    pub const INTERFACE: i32 = 316858560;
}

#[cfg(rev = "245.2")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = -1858954999;
    pub const LOC: i32 = 626415911;
    pub const FLO: i32 = -532285888;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = 1480086078;
    pub const NPC: i32 = 417024969;
    pub const OBJ: i32 = 344600333;
    pub const SPOTANIM: i32 = 96621343;
    pub const INTERFACE: i32 = 587792799;
}

#[cfg(rev = "254")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = -716271600;
    pub const LOC: i32 = -826309209;
    pub const FLO: i32 = -1566957964;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = 1039564548;
    pub const NPC: i32 = 1077655221;
    pub const OBJ: i32 = 535204494;
    pub const SPOTANIM: i32 = -555849646;
    pub const INTERFACE: i32 = 1728499832;
    pub const VARBIT: i32 = -1387031023;
}

#[cfg(rev = "274")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = -753410077;
    pub const LOC: i32 = 452815002;
    pub const FLO: i32 = 960212554;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = 703279713;
    pub const NPC: i32 = -1249602232;
    pub const OBJ: i32 = 128627047;
    pub const SPOTANIM: i32 = -1587698939;
    pub const INTERFACE: i32 = 2041671134;
    pub const VARBIT: i32 = -234977015;
    pub const MESANIM: i32 = 1747166838;
    pub const MES: i32 = 1145177955;
    pub const PARAM: i32 = 254004952;
    pub const HUNT: i32 = 1104745215;
}

#[cfg(rev = "225")]
const CRC_KEYS: [&str; 8] = [
    "title",
    "config",
    "interface",
    "media",
    "models",
    "textures",
    "wordenc",
    "sounds",
];

#[cfg(since_244)]
const CRC_KEYS: [&str; 8] = [
    "title",
    "config",
    "interface",
    "media",
    "versionlist",
    "textures",
    "wordenc",
    "sounds",
];

fn insert_jag(
    crcs: &mut HashMap<&'static str, i32>,
    packs: &mut HashMap<&'static str, Arc<[u8]>>,
    name: &'static str,
    data: Vec<u8>,
    expected_crc: Option<i32>,
    verify: bool,
) {
    if data.is_empty() {
        panic!("Jag file is empty");
    }
    if let (true, Some(expected)) = (verify, expected_crc) {
        let actual = crc::getcrc(&data, 0, data.len());
        if actual != expected {
            panic!("CRC mismatch ['{name}']: Got: {actual}, Expected: {expected}");
        }
        crcs.insert(name, expected);
    } else {
        crcs.insert(name, crc::getcrc(&data, 0, data.len()));
    }
    packs.insert(name, Arc::from(data));
}

fn unwrap_thread<T>(name: &str, result: std::thread::Result<T>) -> T {
    result.unwrap_or_else(|e| {
        let msg = if let Some(s) = e.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = e.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        panic!("{name} thread panicked: {msg}");
    })
}

pub fn pack_all(
    source: &Path,
    pack: &Path,
    verify: bool,
    members: bool,
) -> anyhow::Result<(Box<CacheStore>, cache::script::ScriptProvider)> {
    debug!("Packing assets...");
    debug!("  source: {}", source.display());
    debug!("  pack:   {}", pack.display());
    if verify {
        debug!("  mode:   VERIFY (strict)");
    }

    let registry = PackRegistry::load(pack)?;

    // Run independent packing tasks in parallel using scoped threads.
    // Script compilation runs alongside asset packing - it's only needed
    // at the end for ScriptProvider construction.

    #[cfg(since_244)]
    let mut ondemand: Option<OndemandArtifacts> = None;

    let (
        script_dat,
        script_idx,
        mut assets,
        media,
        textures,
        title,
        models,
        sounds,
        wordenc,
        jingles,
        songs,
        mapsquares,
        mapcrcs,
        multimap,
        freemap,
    ) = std::thread::scope(|s| {
        let h_scripts = s.spawn(|| {
            debug!("Compiling RuneScript sources...");
            match runec::compile_memory(source, Some(pack), false) {
                Ok(bytes) => bytes,
                Err(e) => panic!("RuneScript compilation failed: {}", e),
            }
        });
        let h_assets = s.spawn(|| pack::pack::pack_assets(&registry, source, verify));
        let h_media = s.spawn(|| pack::media::pack_media_jag(source));
        let h_textures = s.spawn(|| pack::texture::pack_textures_jag(&registry, source));
        let h_title = s.spawn(|| pack::title::pack_title_jag(source));
        let h_models = s.spawn(|| pack::model::pack_models(&registry, source, pack));
        let h_sounds = s.spawn(|| pack::sound::pack_sounds(&registry, source, pack));
        let h_wordenc = s.spawn(|| pack::wordenc::pack_wordenc(source));
        let h_jingles = s.spawn(|| other::jingle::pack_jingles(source));
        let h_songs = s.spawn(|| other::song::pack_songs(source));
        let h_maps = s.spawn(|| other::map::pack_maps(source));
        #[cfg(since_244)]
        let h_ondemand = s.spawn(|| build_ondemand_artifacts(source, pack, 5));

        let (script_dat, script_idx) = unwrap_thread("scripts", h_scripts.join());
        let assets = unwrap_thread("assets", h_assets.join()).unwrap();
        let media = unwrap_thread("media", h_media.join());
        let textures = unwrap_thread("textures", h_textures.join());
        let title = unwrap_thread("title", h_title.join());
        let models = unwrap_thread("models", h_models.join());
        let sounds = unwrap_thread("sounds", h_sounds.join());
        let wordenc = unwrap_thread("wordenc", h_wordenc.join());
        let jingles = unwrap_thread("jingles", h_jingles.join());
        let songs = unwrap_thread("songs", h_songs.join());
        let (mapsquares, mapcrcs, multimap, freemap) = unwrap_thread("maps", h_maps.join());
        #[cfg(since_244)]
        {
            ondemand = Some(unwrap_thread("ondemand", h_ondemand.join()));
        }

        (
            script_dat, script_idx, assets, media, textures, title, models, sounds, wordenc,
            jingles, songs, mapsquares, mapcrcs, multimap, freemap,
        )
    });

    #[cfg(since_244)]
    let ondemand = ondemand.expect("ondemand thread did not run");

    let mut crcs = HashMap::new();
    let mut jags = HashMap::new();

    debug!("Packing config...");
    insert_jag(
        &mut crcs,
        &mut jags,
        "config",
        assemble_config_jag(&mut assets, pack),
        jag_crc::CONFIG,
        verify,
    );

    debug!("Packing interface...");
    insert_jag(
        &mut crcs,
        &mut jags,
        "interface",
        assemble_interface_jag(&mut assets),
        jag_crc::INTERFACE,
        verify,
    );

    insert_jag(&mut crcs, &mut jags, "media", media, jag_crc::MEDIA, verify);
    insert_jag(
        &mut crcs,
        &mut jags,
        "textures",
        textures,
        jag_crc::TEXTURES,
        verify,
    );
    insert_jag(&mut crcs, &mut jags, "title", title, jag_crc::TITLE, verify);
    insert_jag(
        &mut crcs,
        &mut jags,
        "models",
        models,
        jag_crc::MODELS,
        verify,
    );
    insert_jag(
        &mut crcs,
        &mut jags,
        "sounds",
        sounds,
        jag_crc::SOUNDS,
        verify,
    );
    insert_jag(
        &mut crcs,
        &mut jags,
        "wordenc",
        wordenc,
        jag_crc::WORDENC,
        verify,
    );

    #[cfg(since_244)]
    insert_jag(
        &mut crcs,
        &mut jags,
        "versionlist",
        ondemand.version_list,
        jag_crc::VERSIONLIST,
        verify,
    );
    #[cfg(all(since_244, before_274))]
    let ondemand_zip: Arc<[u8]> = Arc::from(ondemand.zip);
    #[cfg(since_244)]
    let ondemand: OndemandBlobs = ondemand.blobs;

    debug!("Pack complete.");

    // Build CRC table
    let mut crctable = [0; 9];
    for (i, &key) in CRC_KEYS.iter().enumerate() {
        if let Some(&data) = crcs.get(key) {
            crctable[i + 1] = data;
        }
    }
    let crc_bytes: Vec<u8> = crctable.iter().flat_map(|n| n.to_be_bytes()).collect();
    let crc_buffer32 = crc::getcrc(&crc_bytes, 0, crc_bytes.len());
    #[cfg(since_274)]
    let crc_bytes = {
        let mut bytes = crc_bytes;
        bytes.extend_from_slice(&crc_table_footer(&crctable).to_be_bytes());
        bytes
    };
    let crctable_bytes = Arc::from(crc_bytes);

    let params = build_type_provider::<ParamType>(&assets, "param", ());
    let objs = build_type_provider_into::<ObjTypeRaw, ObjType>(
        &assets,
        "obj",
        ObjContext {
            members,
            autodisable_params: params.types.iter().map(|p| p.autodisable).collect(),
        },
    );
    let invs = build_type_provider::<InvType>(&assets, "inv", ());
    let varps = build_type_provider::<VarPlayerType>(&assets, "varp", ());
    #[cfg(since_254)]
    let varbits = build_type_provider::<VarbitType>(&assets, "varbit", ());
    let dbrows = build_type_provider::<DbRowType>(&assets, "dbrow", ());
    let dbtables = build_type_provider::<DbTableType>(&assets, "dbtable", ());
    let db_index = DbTableIndex::build(&dbtables, &dbrows);
    let enums = build_type_provider::<EnumType>(&assets, "enum", ());
    let flos = build_type_provider::<FloType>(&assets, "flo", ());
    let hunts = build_type_provider::<HuntType>(&assets, "hunt", ());
    let idks = build_type_provider_into::<IdkTypeRaw, IdkType>(&assets, "idk", ());
    let locs = build_type_provider_into::<LocTypeRaw, LocType>(&assets, "loc", ());
    let mesanims = build_type_provider::<MesAnimType>(&assets, "mesanim", ());
    let npcs = build_type_provider_into::<NpcTypeRaw, NpcType>(&assets, "npc", ());
    #[cfg(rev = "225")]
    let seq_frames = cache::seq_frame::SeqFrameProvider::from_jag(
        jags.get("models")
            .expect("Missing models JAG for seq frames"),
    );
    #[cfg(rev = "225")]
    let seq_frame_delays = seq_frames.delays.clone();
    #[cfg(since_244)]
    let seq_frame_delays = cache::seq_frame::anim_frame_delays(source);
    let seqs = build_type_provider_into::<SeqTypeRaw, SeqType>(&assets, "seq", seq_frame_delays);
    let spotanims =
        build_type_provider_into::<SpotAnimTypeRaw, SpotAnimType>(&assets, "spotanim", ());
    let structs = build_type_provider::<StructType>(&assets, "struct", ());
    let varns = build_type_provider::<VarnType>(&assets, "varn", ());
    let varss = build_type_provider::<VarsType>(&assets, "vars", ());
    let categories = build_type_provider::<CategoryType>(&assets, "category", ());
    let interfaces = {
        let packed_file = assets
            .get("interface")
            .expect("Missing packed data for interface");
        IfTypeProvider::from_bytes(&packed_file.server.dat)
    };
    let fonts = {
        let title_jag = jags.get("title").expect("Missing title JAG for fonts");
        FontTypeProvider::from_jag(title_jag)
    };
    let wordenc = {
        let wordenc_jag = jags.get("wordenc").expect("Missing wordenc JAG");
        WordEncProvider::from_jag(wordenc_jag)
    };
    let midi_songs = MidiProvider::from_compressed(songs);
    let midi_jingles = MidiProvider::from_compressed(jingles);

    #[cfg(since_244)]
    let midi_ids: HashMap<Box<str>, u16> = load_existing_pack(pack, "midi")
        .into_iter()
        .map(|(id, name)| (name.into_boxed_str(), id))
        .collect();

    #[cfg(since_274)]
    let midi_tick_lengths: Box<[Option<u16>]> = {
        let size = midi_ids
            .values()
            .map(|&id| id as usize + 1)
            .max()
            .unwrap_or(0);
        let mut lengths: Vec<Option<u16>> = vec![None; size];
        for (name, &id) in &midi_ids {
            if let Some(midi) = midi_songs
                .get_by_name(name)
                .or_else(|| midi_jingles.get_by_name(name))
            {
                lengths[id as usize] = Some(midi.tick_length() as u16);
            }
        }
        lengths.into_boxed_slice()
    };

    debug!(
        "TypeProviders: objs={} invs={} varps={} dbrows={} dbtables={} enums={} flos={} hunts={} idks={} locs={} mesanims={} npcs={} params={} seqs={} spotanims={} structs={} varns={} varss={} categories={} interfaces={} fonts={} wordenc=bad:{}/frag:{}/tld:{}/dom:{} songs={} jingles={}",
        objs.count(),
        invs.count(),
        varps.count(),
        dbrows.count(),
        dbtables.count(),
        enums.count(),
        flos.count(),
        hunts.count(),
        idks.count(),
        locs.count(),
        mesanims.count(),
        npcs.count(),
        params.count(),
        seqs.count(),
        spotanims.count(),
        structs.count(),
        varns.count(),
        varss.count(),
        categories.count(),
        interfaces.count(),
        fonts.count(),
        wordenc.bads.len(),
        wordenc.fragments.len(),
        wordenc.tlds.len(),
        wordenc.domains.len(),
        midi_songs.count(),
        midi_jingles.count(),
    );

    // Script bytecode -> ScriptProvider
    let scripts = cache::script::ScriptProvider::from_bytes(&script_dat, &script_idx);
    debug!("Scripts: {} loaded", scripts.count());

    #[cfg(all(since_244, before_274))]
    let build: Arc<[u8]> = {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0);
        Arc::from(secs.to_be_bytes().to_vec())
    };

    let store = Box::new(CacheStore {
        crctable_bytes,
        crc_buffer32,
        #[cfg(all(since_244, before_274))]
        ondemand_zip,
        #[cfg(all(since_244, before_274))]
        build,
        #[cfg(since_244)]
        ondemand,
        crcs,
        jags,
        mapsquares,
        mapcrcs,
        objs,
        invs,
        varps,
        #[cfg(since_254)]
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
        #[cfg(rev = "225")]
        seq_frames,
        seqs,
        spotanims,
        structs,
        varns,
        varss,
        categories,
        interfaces,
        fonts,
        wordenc,
        songs: midi_songs,
        jingles: midi_jingles,
        #[cfg(since_244)]
        midi_ids,
        #[cfg(since_274)]
        midi_tick_lengths,
        static_assets: load_static_assets(),
        multimap,
        freemap,
    });

    debug!("CacheStore built successfully");
    Ok((store, scripts))
}

fn load_static_assets() -> HashMap<Box<str>, Arc<[u8]>> {
    let dir = Path::new("public");
    let mut assets = HashMap::new();
    if !dir.exists() {
        return assets;
    }
    load_assets_recursive(dir, dir, &mut assets);
    debug!("Loaded {} static assets into memory", assets.len());
    assets
}

fn load_assets_recursive(base: &Path, dir: &Path, assets: &mut HashMap<Box<str>, Arc<[u8]>>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            load_assets_recursive(base, &path, assets);
        } else if let Ok(data) = std::fs::read(&path) {
            let key = path
                .strip_prefix(base)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let key = format!("/{key}");
            assets.insert(Box::from(key), Arc::from(data));
        }
    }
}

fn build_type_provider<T: cache::provider::CacheType>(
    assets: &HashMap<String, pack::pack_registry::PackedFile>,
    name: &str,
    ctx: T::Context,
) -> TypeProvider<T> {
    build_type_provider_into::<T, T>(assets, name, ctx)
}

fn build_type_provider_into<Raw, Stored>(
    assets: &HashMap<String, pack::pack_registry::PackedFile>,
    name: &str,
    ctx: Raw::Context,
) -> TypeProvider<Stored>
where
    Raw: cache::provider::CacheType,
    Stored: From<Raw>,
{
    let packed_file = assets
        .get(name)
        .unwrap_or_else(|| panic!("Missing packed data for {name}"));
    TypeProvider::from_bytes::<Raw>(&packed_file.server.dat, ctx)
}

fn assemble_config_jag(
    assets: &mut HashMap<String, pack::pack_registry::PackedFile>,
    pack_dir: &Path,
) -> Vec<u8> {
    let order: Vec<String> = std::fs::read_to_string(pack_dir.join("config.order"))
        .expect("Missing config.order")
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();

    let mut clients: HashMap<&str, _> = HashMap::new();
    for entry in &order {
        let kind = entry.split_once('.').map_or(entry.as_str(), |(k, _)| k);
        if !clients.contains_key(kind)
            && let Some(client) = assets.get_mut(kind).and_then(|pf| pf.client.take())
        {
            clients.insert(kind, client);
        }
    }

    let mut jag = JagFile::new();
    let mut has_data = false;
    for entry in &order {
        let Some((kind, ext)) = entry.split_once('.') else {
            continue;
        };
        if let Some(client) = clients.get_mut(kind) {
            let data = if ext == "idx" {
                std::mem::take(&mut client.idx)
            } else {
                std::mem::take(&mut client.dat)
            };
            jag.write(entry, data);
            has_data = true;
        }
    }
    if has_data {
        jag.build(JagCompression::PerFile)
    } else {
        Vec::new()
    }
}

fn assemble_interface_jag(
    assets: &mut HashMap<String, pack::pack_registry::PackedFile>,
) -> Vec<u8> {
    if let Some(packed_file) = assets.get_mut("interface")
        && let Some(client) = packed_file.client.take()
    {
        let mut jag = JagFile::new();
        jag.write("data", client.dat);
        return jag.build(JagCompression::WholeArchive);
    }
    Vec::new()
}

#[cfg(since_274)]
fn crc_table_footer(crctable: &[i32]) -> i32 {
    let mut acc: i32 = 1234;
    for &c in crctable {
        acc = (acc << 1).wrapping_add(c);
    }
    acc
}
