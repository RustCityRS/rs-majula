use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about = "RuneScape Config Pack Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile text config files and build an in-memory CacheStore
    Pack {
        /// Source directory containing config files (.npc, .hunt, etc.)
        #[arg(short, long, default_value = rs_pack::CONTENT_DIR)]
        source: PathBuf,
        /// Pack directory for name-id resolution
        #[arg(long, default_value = rs_pack::PACK_DIR)]
        pack: PathBuf,
        /// Strict CRC verification against the original cache (`--verify false`
        /// to pack edited/custom content that intentionally differs)
        #[arg(long, action = clap::ArgAction::Set, default_value_t = true)]
        verify: bool,
        #[arg(long, default_value = "true")]
        members: bool,
    },
    /// Download the reference cache from the OpenRS2 archive into `expected/`
    /// and check it against the archive's own reference tables and keys
    #[cfg(since_647)]
    Fetch {
        /// OpenRS2 cache id (default: this revision's reference cache)
        #[arg(long)]
        cache: Option<u32>,
        /// Directory to extract the dat2 cache into
        #[arg(short, long, default_value = "expected")]
        output: PathBuf,
        /// Where the downloaded zip and API responses are kept
        #[arg(long, default_value = "target/openrs2")]
        work_dir: PathBuf,
        /// Committed XTEA keys to compare against OpenRS2's
        /// (default: this revision's `keys.json`)
        #[arg(long)]
        keys: Option<PathBuf>,
        /// Re-download and overwrite even if `output` already holds a cache
        #[arg(long)]
        force: bool,
    },
    /// Extract original JAG archives into content files for re-packing
    Unpack {
        /// Directory containing expected JAG files
        #[arg(short, long, default_value = "expected")]
        expected: PathBuf,
        /// Output directory for unpacked content
        #[arg(short, long, default_value = "content_unpack")]
        output: PathBuf,
        /// `.pack` name registries to name the output by (default:
        /// `<output>/pack`). Discovery is idempotent: hand-maintained names
        /// are kept, only missing entries are added.
        #[arg(long)]
        pack: Option<PathBuf>,
        /// Unpack only these domains, in place: any of `configs`, `maps`,
        /// `assets`, comma-separated. Each selected `_unpack` directory is
        /// cleared first, so it ends up a function of the cache alone.
        #[cfg(since_647)]
        #[arg(long)]
        only: Option<String>,
        /// Where a partial unpack keeps what must not land in `output`
        /// (decompiled clientscripts, the run report)
        #[cfg(since_647)]
        #[arg(long, default_value = "target/unpack-work")]
        work_dir: PathBuf,
    },
    /// Generate editable .pyxel sprite docs next to the content TGAs
    #[cfg(before_647)]
    ToPyxel {
        /// Content directory holding the sprite TGAs
        #[arg(short, long, default_value = rs_pack::CONTENT_DIR)]
        source: PathBuf,
    },
    /// Rebuild content TGAs from edited .pyxel docs (run before `pack`)
    #[cfg(before_647)]
    FromPyxel {
        /// Content directory holding the .pyxel docs
        #[arg(short, long, default_value = rs_pack::CONTENT_DIR)]
        source: PathBuf,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,runec=error")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        #[cfg(before_647)]
        Commands::Pack {
            source,
            pack,
            verify,
            members,
        } => {
            let (store, scripts) = rs_pack::pack_all(&source, &pack, verify, members)?;
            tracing::debug!(
                "CacheStore: {} packs, {} jingles, {} maps, {} songs, {} objs, {} invs, {} varps, {} scripts",
                store.jags.len(),
                store.jingles.count(),
                store.mapsquares.len(),
                store.songs.count(),
                store.objs.count(),
                store.invs.count(),
                store.varps.count(),
                scripts.count(),
            );
        }
        #[cfg(since_647)]
        Commands::Pack {
            source,
            pack,
            verify,
            members,
        } => {
            let result = rs_pack::pack_all(&source, &pack, verify, members)?;
            tracing::info!(
                "JS5 cache built at {}: {} npcs, {} objs, {} locs, {} seqs, {} enums, {} structs, {} invs, {} varbits",
                result.cache_dir.display(),
                result.config.npcs.count(),
                result.config.objs.count(),
                result.config.locs.count(),
                result.config.seqs.count(),
                result.config.enums.count(),
                result.config.structs.count(),
                result.config.invs.count(),
                result.config.varbits.count(),
            );
        }
        #[cfg(since_647)]
        Commands::Fetch {
            cache,
            output,
            work_dir,
            keys,
            force,
        } => fetch(cache, &output, &work_dir, keys, force)?,
        #[cfg(before_647)]
        Commands::ToPyxel { source } => rs_pack::pyxel::content_to_pyxel(&source)?,
        #[cfg(before_647)]
        Commands::FromPyxel { source } => rs_pack::pyxel::content_from_pyxel(&source)?,
        Commands::Unpack {
            expected,
            output,
            pack,
            #[cfg(since_647)]
            only,
            #[cfg(since_647)]
            work_dir,
        } => {
            let pack = pack.unwrap_or_else(|| output.join("pack"));
            #[cfg(before_647)]
            rs_pack::unpack::unpack_all(&expected, &output, &pack)?;
            #[cfg(since_647)]
            match only {
                None => rs_pack::unpack::unpack_all(&expected, &output, &pack)?,
                Some(list) => rs_pack::unpack::unpack(
                    &expected,
                    &output,
                    &pack,
                    rs_pack::unpack::Domains::parse(&list)?,
                    &work_dir,
                )?,
            }
        }
    }

    Ok(())
}

#[cfg(since_647)]
fn fetch(
    cache: Option<u32>,
    output: &std::path::Path,
    work_dir: &std::path::Path,
    keys: Option<PathBuf>,
    force: bool,
) -> Result<()> {
    use anyhow::{Context, bail};
    use rs_pack::openrs2;

    let id = cache
        .or(openrs2::default_cache_id())
        .context("this revision has no OpenRS2 reference cache; pass --cache")?;

    if output.join("main_file_cache.dat2").is_file() && !force {
        tracing::info!(
            "{} already holds a cache; verifying it (--force re-downloads)",
            output.display()
        );
    } else {
        let zip = work_dir.join(id.to_string()).join("disk.zip");
        if force || !zip.is_file() {
            let url = openrs2::disk_zip_url(id);
            tracing::info!("downloading {url}");
            let bytes = openrs2::download_to_file(&url, &zip)?;
            tracing::info!("  {bytes} bytes -> {}", zip.display());
        }
        let files = openrs2::extract_disk_zip(&zip, output)?;
        tracing::info!("extracted {} files into {}", files.len(), output.display());
    }

    let archives = rs_pack::startup::MASTER_INDEX_ORDER;
    let mismatches = openrs2::verify_reference_tables(output, id, &archives, work_dir)?;
    if !mismatches.is_empty() {
        for m in &mismatches {
            tracing::error!(
                "  archive {:>2}: local {:?}, OpenRS2 {:#010x}",
                m.archive,
                m.local.map(|c| format!("{c:#010x}")),
                m.remote
            );
        }
        bail!(
            "{} of {} reference tables in {} differ from OpenRS2 cache {id}; re-run with --force",
            mismatches.len(),
            archives.len(),
            output.display()
        );
    }
    tracing::info!(
        "all {} reference tables match OpenRS2 cache {id}",
        archives.len()
    );

    let keys = keys.unwrap_or_else(|| std::path::Path::new(rs_pack::CONTENT_DIR).join("keys.json"));
    let local = openrs2::parse_keys(
        &std::fs::read_to_string(&keys).with_context(|| format!("reading {}", keys.display()))?,
    )?;
    let remote = openrs2::parse_keys(std::str::from_utf8(&openrs2::get_bytes(
        &openrs2::keys_url(id),
    )?)?)?;
    // Extra local keys are expected: squares rebuilt from donor caches are
    // re-encrypted with their committed key. Only a missing or different key
    // means the committed file is wrong.
    let diff = openrs2::diff_keys(&local, &remote);
    if diff.only_remote.is_empty() && diff.different.is_empty() {
        tracing::info!(
            "{}: all {} OpenRS2 keys match ({} more held locally)",
            keys.display(),
            remote.len(),
            diff.only_local.len()
        );
    } else {
        tracing::warn!(
            "{} vs OpenRS2 keys: {} only on OpenRS2, {} different: {:?} {:?}",
            keys.display(),
            diff.only_remote.len(),
            diff.different.len(),
            diff.only_remote,
            diff.different
        );
    }
    Ok(())
}
