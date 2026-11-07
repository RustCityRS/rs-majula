#[cfg(before_647)]
use crate::archive::{JAG_ARCHIVES, JagArchive};
#[cfg(before_647)]
use crate::cache::CacheStore;
#[cfg(before_647)]
use crate::cache::category::CategoryType;
#[cfg(before_647)]
use crate::cache::dbrow::DbRowType;
#[cfg(before_647)]
use crate::cache::dbtable::{DbTableIndex, DbTableType};
#[cfg(before_647)]
use crate::cache::r#enum::{EnumType, EnumTypeRaw};
#[cfg(before_647)]
use crate::cache::flo::{FloType, FloTypeRaw};
#[cfg(before_647)]
use crate::cache::font::FontTypeProvider;
#[cfg(before_647)]
use crate::cache::hunt::HuntType;
#[cfg(before_647)]
use crate::cache::idk::{IdkType, IdkTypeRaw};
#[cfg(before_647)]
use crate::cache::r#if::IfTypeProvider;
#[cfg(before_647)]
use crate::cache::inv::{InvType, InvTypeRaw};
#[cfg(before_647)]
use crate::cache::loc::{LocType, LocTypeRaw};
#[cfg(before_647)]
use crate::cache::mesanim::MesAnimType;
#[cfg(before_647)]
use crate::cache::midi::MidiProvider;
#[cfg(before_647)]
use crate::cache::npc::{NpcType, NpcTypeRaw};
#[cfg(before_647)]
use crate::cache::obj::{ObjContext, ObjType, ObjTypeRaw};
#[cfg(before_647)]
use crate::cache::param::{ParamType, ParamTypeRaw};
#[cfg(before_647)]
use crate::cache::provider::{CacheType, TypeProvider};
use crate::cache::script::ScriptProvider;
#[cfg(before_647)]
use crate::cache::seq::{SeqType, SeqTypeRaw};
#[cfg(rev = "225")]
use crate::cache::seq_frame::SeqFrameProvider;
#[cfg(all(since_244, before_647))]
use crate::cache::seq_frame::anim_frame_delays;
#[cfg(before_647)]
use crate::cache::spotanim::{SpotAnimType, SpotAnimTypeRaw};
#[cfg(before_647)]
use crate::cache::r#struct::{StructType, StructTypeRaw};
#[cfg(all(since_254, before_647))]
use crate::cache::varbit::VarbitType;
#[cfg(before_647)]
use crate::cache::varn::VarnType;
#[cfg(before_647)]
use crate::cache::varp::{VarPlayerType, VarPlayerTypeRaw};
#[cfg(before_647)]
use crate::cache::vars::VarsType;
#[cfg(before_647)]
use crate::cache::wordenc::WordEncProvider;
#[cfg(before_647)]
use crate::pack::media::pack_media_jag;
#[cfg(before_647)]
use crate::pack::model::pack_models;
#[cfg(all(since_244, before_647))]
use crate::pack::ondemand::{OndemandArtifacts, build_ondemand_artifacts};
#[cfg(before_647)]
use crate::pack::other::map::pack_maps;
#[cfg(before_647)]
use crate::pack::other::song::pack_midis;
#[cfg(before_647)]
use crate::pack::pack::pack_assets;
#[cfg(before_647)]
use crate::pack::pack_registry::{PackRegistry, PackedFile};
#[cfg(before_647)]
use crate::pack::sound::pack_sounds;
#[cfg(before_647)]
use crate::pack::texture::pack_textures_jag;
#[cfg(before_647)]
use crate::pack::title::pack_title_jag;
#[cfg(before_647)]
use crate::pack::wordenc::pack_wordenc;
#[cfg(all(since_244, before_647))]
use crate::types::OndemandBlobs;
#[cfg(all(since_244, before_647))]
use crate::unpack::model::load_existing_pack;
#[cfg(since_647)]
use rayon::ThreadPoolBuilder;
#[cfg(before_647)]
use rs_io::crc;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(before_647)]
use std::collections::HashMap;
#[cfg(before_647)]
use std::mem::take;
use std::path::Path;
#[cfg(before_647)]
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{env, fs, io, process};
use tracing::{debug, warn};

pub mod archive;
pub mod back;
pub mod cache;
#[cfg(since_647)]
pub mod cs2;
#[cfg(since_647)]
pub mod discover;
pub mod pack;
#[cfg(before_647)]
pub mod pyxel;
pub mod sheet;
#[cfg(since_647)]
pub mod startup;
pub mod tga;
pub mod types;
pub mod unpack;
#[cfg(all(since_244, before_647))]
pub mod versionlist;
pub mod zone_csv;

pub use crate::types::ParamValue;

macro_rules! rev_content_path {
    ($suffix:literal) => {
        concat!("content/", env!("REV"), $suffix)
    };
}

pub const CONTENT_DIR: &str = rev_content_path!("");
pub const PACK_DIR: &str = rev_content_path!("/pack");

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

#[cfg(rev = "289")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = -2029399626;
    pub const LOC: i32 = -795412965;
    pub const FLO: i32 = 960212554;
    pub const IDK: i32 = -359342366;
    pub const VARP: i32 = 266133304;
    pub const NPC: i32 = 424938091;
    pub const OBJ: i32 = 1565507783;
    pub const SPOTANIM: i32 = 1145838588;
    pub const INTERFACE: i32 = 1940627269;
    pub const VARBIT: i32 = -1746090972;
    pub const MESANIM: i32 = 1747166838;
    pub const MES: i32 = 1145177955;
    pub const PARAM: i32 = -918994068;
    pub const HUNT: i32 = 1104745215;
}

#[cfg(rev = "647")]
pub(crate) mod config_crc {
    pub const SEQ: i32 = -66602266;
    // Includes the restored loc definitions covered by the group CRC gate.
    pub const LOC: i32 = 538701052;
    pub const FLO: i32 = 321707212;
    pub const SPOTANIM: i32 = 1788539353;
    pub const OBJ: i32 = 896458165;
    pub const NPC: i32 = -1000029806;
    pub const IDK: i32 = 50920008;
    pub const VARP: i32 = 1704792051;
    pub const VARBIT: i32 = 763651066;
    pub const PARAM: i32 = 1671102909;
    pub const ENUM: i32 = -2002672615;
    pub const INV: i32 = -331361170;
    pub const STRUCT: i32 = 293771491;
    pub const FLU: i32 = -1744098420;

    pub const GENERIC: &[(&str, i32)] = &[
        ("bas", -1911964791),
        ("config15", -1218318004),
        ("config18", 402829966),
        ("config19", 57638886),
        ("config2", 1389808433),
        ("config20", -1949850762),
        ("config21", -1751999962),
        ("config22", -1236602203),
        ("config23", -1095511195),
        ("config24", 2129060492),
        ("config25", -1298439089),
        ("config37", 477744615),
        ("config38", -430249901),
        ("config39", 583011186),
        ("config40", 1145177955),
        ("config41", 2013865111),
        ("config42", -1179041704),
        ("config43", 1930184948),
        ("config44", 812315341),
        ("config45", 260531396),
        ("config47", -1864459835),
        ("config48", -430249901),
        ("config7", -1012851422),
        ("cursor", -719985808),
        ("hitmark", 313095873),
        ("light", -1541305497),
        ("mel", 1237876085),
        ("msi", 390525203),
        ("quest", -824626290),
        ("varclansetting", -904846536),
    ];
}

pub(crate) const CONFIG_TYPE_CRCS: &[(&str, i32)] = &[
    ("seq", config_crc::SEQ),
    ("loc", config_crc::LOC),
    ("flo", config_crc::FLO),
    ("spotanim", config_crc::SPOTANIM),
    ("obj", config_crc::OBJ),
    ("npc", config_crc::NPC),
    ("idk", config_crc::IDK),
    ("varp", config_crc::VARP),
    #[cfg(since_254)]
    ("varbit", config_crc::VARBIT),
    #[cfg(all(since_274, before_647))]
    ("mesanim", config_crc::MESANIM),
    #[cfg(all(since_274, before_647))]
    ("mes", config_crc::MES),
    #[cfg(since_274)]
    ("param", config_crc::PARAM),
    #[cfg(all(since_274, before_647))]
    ("hunt", config_crc::HUNT),
    #[cfg(since_647)]
    ("enum", config_crc::ENUM),
    #[cfg(since_647)]
    ("inv", config_crc::INV),
    #[cfg(since_647)]
    ("struct", config_crc::STRUCT),
    #[cfg(since_647)]
    ("flu", config_crc::FLU),
];

#[cfg(before_647)]
fn insert_jag(
    crcs: &mut HashMap<&'static str, i32>,
    packs: &mut HashMap<&'static str, Arc<[u8]>>,
    kind: JagArchive,
    data: Vec<u8>,
    verify: bool,
) {
    let name = kind.name();
    if data.is_empty() {
        panic!("Jag file is empty");
    }
    if let (true, Some(expected)) = (verify, kind.expected_crc()) {
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

fn unwrap_thread<T>(name: &str, result: thread::Result<T>) -> T {
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

/// Compiles the RuneScript sources under `source` alone, for swapping into
/// a running world: the cache it already serves stays as it is. Errors are
/// the compiler's, as it reports them.
pub fn compile_scripts(source: &Path, pack: &Path) -> anyhow::Result<ScriptProvider> {
    let (dat, idx) = compile_runescript(source, pack)?;
    Ok(ScriptProvider::from_bytes(&dat, &idx))
}

fn compile_runescript(source: &Path, pack: &Path) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let start = Instant::now();
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let private = env::temp_dir().join(format!(
        "rs-pack-{}-{nanos}-{}",
        process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&private).map_err(|e| anyhow::anyhow!("creating {}: {e}", private.display()))?;
    let compiled = copy_pack(pack, &private).and_then(|()| {
        debug!(
            "Prepared script registries in {:.3}s",
            start.elapsed().as_secs_f64()
        );
        let start = Instant::now();
        let bytes =
            runec::compile_memory(&source.join("scripts/runescript"), Some(&private), false)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
        debug!("Compiled scripts in {:.2}s", start.elapsed().as_secs_f64());
        pin_script_ids(pack, &private);
        Ok(bytes)
    });
    if let Err(e) = fs::remove_dir_all(&private) {
        warn!("Couldn't remove {}: {e}", private.display());
    }
    compiled
}

/// Copies the files of `pack` into the empty directory `to`.
fn copy_pack(pack: &Path, to: &Path) -> anyhow::Result<()> {
    let copy = || -> io::Result<()> {
        for entry in fs::read_dir(pack)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                fs::copy(entry.path(), to.join(entry.file_name()))?;
            }
        }
        Ok(())
    };
    copy().map_err(|e| anyhow::anyhow!("copying {} to compile against: {e}", pack.display()))
}

fn pin_script_ids(pack: &Path, private: &Path) {
    let Ok(pinned) = fs::read(private.join("script.pack")) else {
        return;
    };
    let shipped = pack.join("script.pack");
    if fs::read(&shipped).is_ok_and(|old| old == pinned) {
        return;
    }
    let tag = private.file_name().unwrap_or_default().to_string_lossy();
    let tmp = pack.join(format!("script.pack.{tag}.tmp"));
    if let Err(e) = fs::write(&tmp, &pinned).and_then(|()| fs::rename(&tmp, &shipped)) {
        warn!("Couldn't update {}: {e}", shipped.display());
        let _ = fs::remove_file(&tmp);
    }
}

#[cfg(since_647)]
pub fn pack_all(
    source: &Path,
    pack: &Path,
    verify: bool,
    members: bool,
) -> anyhow::Result<startup::PackResult> {
    ThreadPoolBuilder::new()
        .build()?
        .install(|| startup::pack_all(source, pack, verify, members))
}

#[cfg(before_647)]
pub fn pack_all(
    source: &Path,
    pack: &Path,
    verify: bool,
    members: bool,
) -> anyhow::Result<(Box<CacheStore>, ScriptProvider)> {
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
        worldmap,
    ) = thread::scope(|s| {
        let h_scripts = s.spawn(|| {
            debug!("Compiling RuneScript sources...");
            match compile_runescript(source, pack) {
                Ok(bytes) => bytes,
                Err(e) => panic!("RuneScript compilation failed: {}", e),
            }
        });
        let h_assets = s.spawn(|| pack_assets(&registry, &source.join("scripts"), verify));
        let h_media = s.spawn(|| pack_media_jag(source));
        let h_textures = s.spawn(|| pack_textures_jag(&registry, source));
        let h_title = s.spawn(|| pack_title_jag(source));
        let h_models = s.spawn(|| pack_models(&registry, source, pack));
        let h_sounds = s.spawn(|| pack_sounds(&registry, source, pack));
        let h_wordenc = s.spawn(|| pack_wordenc(source));
        let h_jingles = s.spawn(|| pack_midis(source, "jingles"));
        let h_songs = s.spawn(|| pack_midis(source, "songs"));
        let h_maps = s.spawn(|| pack_maps(source));
        #[cfg(since_244)]
        let h_ondemand = s.spawn(|| build_ondemand_artifacts(source, pack));

        let assets = unwrap_thread("assets", h_assets.join()).unwrap();
        let (mapsquares, mapcrcs, multimap, freemap, worldmap) =
            unwrap_thread("maps", h_maps.join());
        // The world map needs packed configs, so it starts once they are done
        // and overlaps the rest (script compilation is the long pole).
        let h_worldmap = {
            let dat = |name: &str| assets[name].server.dat.clone();
            let (flo, loc, npc) = (dat("flo"), dat("loc"), dat("npc"));
            let (multimap, freemap) = (multimap.clone(), freemap.clone());
            s.spawn(move || worldmap.build(source, &flo, &loc, &npc, &multimap, &freemap))
        };
        let (script_dat, script_idx) = unwrap_thread("scripts", h_scripts.join());
        let media = unwrap_thread("media", h_media.join());
        let textures = unwrap_thread("textures", h_textures.join());
        let title = unwrap_thread("title", h_title.join());
        let models = unwrap_thread("models", h_models.join());
        let sounds = unwrap_thread("sounds", h_sounds.join());
        let wordenc = unwrap_thread("wordenc", h_wordenc.join());
        let jingles = unwrap_thread("jingles", h_jingles.join());
        let songs = unwrap_thread("songs", h_songs.join());
        let worldmap = unwrap_thread("worldmap", h_worldmap.join());
        #[cfg(since_244)]
        {
            ondemand = Some(unwrap_thread("ondemand", h_ondemand.join()));
        }

        (
            script_dat, script_idx, assets, media, textures, title, models, sounds, wordenc,
            jingles, songs, mapsquares, mapcrcs, multimap, freemap, worldmap,
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
        JagArchive::Config,
        assemble_config_jag(&mut assets, pack),
        verify,
    );

    debug!("Packing interface...");
    insert_jag(
        &mut crcs,
        &mut jags,
        JagArchive::Interface,
        assemble_interface_jag(&mut assets),
        verify,
    );

    insert_jag(&mut crcs, &mut jags, JagArchive::Media, media, verify);
    // Served for the website's map viewer; not in the client's CRC table.
    jags.insert("worldmap", Arc::from(worldmap));
    insert_jag(&mut crcs, &mut jags, JagArchive::Textures, textures, verify);
    insert_jag(&mut crcs, &mut jags, JagArchive::Title, title, verify);
    insert_jag(&mut crcs, &mut jags, JagArchive::Models, models, verify);
    insert_jag(&mut crcs, &mut jags, JagArchive::Sounds, sounds, verify);
    insert_jag(&mut crcs, &mut jags, JagArchive::Wordenc, wordenc, verify);

    #[cfg(since_244)]
    insert_jag(
        &mut crcs,
        &mut jags,
        JagArchive::VersionList,
        ondemand.version_list,
        verify,
    );
    #[cfg(all(since_244, before_274))]
    let ondemand_zip: Arc<[u8]> = Arc::from(ondemand.zip);
    #[cfg(since_244)]
    let ondemand: OndemandBlobs = ondemand.blobs;

    debug!("Pack complete.");

    // Build CRC table
    let mut crctable = [0; 9];
    for (i, kind) in JAG_ARCHIVES.iter().enumerate() {
        if let Some(&data) = crcs.get(kind.name()) {
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

    let params = build_type_provider_into::<ParamTypeRaw, ParamType>(&assets, "param", ());
    let objs = build_type_provider_into::<ObjTypeRaw, ObjType>(
        &assets,
        "obj",
        ObjContext {
            members,
            autodisable_params: params.types.iter().map(|p| p.autodisable).collect(),
        },
    );
    let invs = build_type_provider_into::<InvTypeRaw, InvType>(&assets, "inv", ());
    let varps = build_type_provider_into::<VarPlayerTypeRaw, VarPlayerType>(&assets, "varp", ());
    #[cfg(since_254)]
    let varbits = build_type_provider::<VarbitType>(&assets, "varbit", ());
    let dbrows = build_type_provider::<DbRowType>(&assets, "dbrow", ());
    let dbtables = build_type_provider::<DbTableType>(&assets, "dbtable", ());
    let db_index = DbTableIndex::build(&dbtables, &dbrows);
    let enums = build_type_provider_into::<EnumTypeRaw, EnumType>(&assets, "enum", ());
    let flos = build_type_provider_into::<FloTypeRaw, FloType>(&assets, "flo", ());
    let hunts = build_type_provider::<HuntType>(&assets, "hunt", ());
    let idks = build_type_provider_into::<IdkTypeRaw, IdkType>(&assets, "idk", ());
    let locs = build_type_provider_into::<LocTypeRaw, LocType>(&assets, "loc", ());
    let mesanims = build_type_provider::<MesAnimType>(&assets, "mesanim", ());
    let npcs = build_type_provider_into::<NpcTypeRaw, NpcType>(&assets, "npc", ());
    #[cfg(rev = "225")]
    let seq_frames = SeqFrameProvider::from_jag(
        jags.get("models")
            .expect("Missing models JAG for seq frames"),
    );
    #[cfg(rev = "225")]
    let seq_frame_delays = seq_frames.delays.clone();
    #[cfg(since_244)]
    let seq_frame_delays = anim_frame_delays(source);
    let seqs = build_type_provider_into::<SeqTypeRaw, SeqType>(&assets, "seq", seq_frame_delays);
    let spotanims =
        build_type_provider_into::<SpotAnimTypeRaw, SpotAnimType>(&assets, "spotanim", ());
    let structs = build_type_provider_into::<StructTypeRaw, StructType>(&assets, "struct", ());
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

    #[cfg(since_254)]
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
    let scripts = ScriptProvider::from_bytes(&script_dat, &script_idx);
    debug!("Scripts: {} loaded", scripts.count());

    #[cfg(all(since_244, before_274))]
    let build: Arc<[u8]> = {
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
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
        #[cfg(since_254)]
        midi_tick_lengths,
        static_assets: load_static_assets(),
        multimap,
        freemap,
    });

    debug!("CacheStore built successfully");
    Ok((store, scripts))
}

#[cfg(before_647)]
fn load_static_assets() -> HashMap<Box<str>, Arc<[u8]>> {
    let dir = Path::new("public");
    let mut assets = HashMap::new();
    if !dir.exists() {
        return assets;
    }
    // The shared files at the top, then only this revision's folder: a world
    // never serves another revision's client.
    load_assets_recursive(dir, dir, false, &mut assets);
    load_assets_recursive(dir, &dir.join(env!("REV")), true, &mut assets);
    debug!("Loaded {} static assets into memory", assets.len());
    assets
}

#[cfg(before_647)]
fn load_assets_recursive(
    base: &Path,
    dir: &Path,
    recurse: bool,
    assets: &mut HashMap<Box<str>, Arc<[u8]>>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if recurse {
                load_assets_recursive(base, &path, recurse, assets);
            }
        } else if let Ok(data) = fs::read(&path) {
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

#[cfg(before_647)]
fn build_type_provider<T: CacheType>(
    assets: &HashMap<String, PackedFile>,
    name: &str,
    ctx: T::Context,
) -> TypeProvider<T> {
    build_type_provider_into::<T, T>(assets, name, ctx)
}

#[cfg(before_647)]
fn build_type_provider_into<Raw, Stored>(
    assets: &HashMap<String, PackedFile>,
    name: &str,
    ctx: Raw::Context,
) -> TypeProvider<Stored>
where
    Raw: CacheType,
    Stored: From<Raw>,
{
    let packed_file = assets
        .get(name)
        .unwrap_or_else(|| panic!("Missing packed data for {name}"));
    TypeProvider::from_indexed_bytes::<Raw>(&packed_file.server.dat, &packed_file.server.idx, ctx)
        .unwrap_or_else(|error| panic!("Invalid {name} configs: {error}"))
}

#[cfg(before_647)]
fn assemble_config_jag(assets: &mut HashMap<String, PackedFile>, pack_dir: &Path) -> Vec<u8> {
    let order: Vec<String> = fs::read_to_string(pack_dir.join("config.order"))
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
                take(&mut client.idx)
            } else {
                take(&mut client.dat)
            };
            jag.write(entry, data);
            has_data = true;
        }
    }
    if has_data {
        jag.build(JagArchive::Config.compression())
    } else {
        Vec::new()
    }
}

#[cfg(before_647)]
fn assemble_interface_jag(assets: &mut HashMap<String, PackedFile>) -> Vec<u8> {
    if let Some(packed_file) = assets.get_mut("interface")
        && let Some(client) = packed_file.client.take()
    {
        let mut jag = JagFile::new();
        jag.write("data", client.dat);
        return jag.build(JagArchive::Interface.compression());
    }
    Vec::new()
}

#[cfg(all(since_274, before_647))]
fn crc_table_footer(crctable: &[i32]) -> i32 {
    let mut acc: i32 = 1234;
    for &c in crctable {
        acc = (acc << 1).wrapping_add(c);
    }
    acc
}
