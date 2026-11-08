use anyhow::Result;
use clap::{ArgAction, Parser, Subcommand};
use rs_io::jag::JagFile;
use rs_io::js5::java_string_hash;
#[cfg(before_647)]
use rs_pack::pyxel;
use rs_pack::unpack;
use std::io::{BufRead, stdin};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(author, version, about = "RuneScape Config Pack Tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Recover missing registry names by matching dictionary candidates to cache hashes
    #[cfg(since_647)]
    RecoverNames {
        #[arg(long, default_value = rs_pack::PACK_DIR)]
        pack: PathBuf,
        #[arg(long)]
        kind: String,
        #[arg(long, default_value_t = 4096)]
        max_index: u32,
        #[arg(required = true)]
        dictionaries: Vec<PathBuf>,
    },
    /// Decompile only the configs, to compare them with another dump
    #[cfg(since_647)]
    UnpackConfigs {
        #[arg(short, long, default_value = "expected")]
        expected: PathBuf,
        #[arg(short, long, default_value = "content_unpack")]
        output: PathBuf,
        #[arg(long, default_value = rs_pack::PACK_DIR)]
        pack: PathBuf,
    },
    /// Decompile client scripts to editable RuneScript source and verify their bytes
    #[cfg(since_647)]
    UnpackClientscripts {
        #[arg(short, long, default_value = "expected")]
        expected: PathBuf,
        #[arg(short, long, default_value = "content_unpack")]
        output: PathBuf,
        #[arg(long, default_value = rs_pack::PACK_DIR)]
        pack: PathBuf,
    },
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
        #[arg(long, action = ArgAction::Set, default_value_t = true)]
        verify: bool,
        #[arg(long, default_value = "true")]
        members: bool,
    },
    /// Extract original cache archives into editable content files
    Unpack {
        /// Directory containing original cache files
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
    /// Print the cache name hash of each string (stdin lines if none are given)
    Namehash {
        #[arg(long)]
        jag: bool,
        names: Vec<String>,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info,runec=error")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        #[cfg(since_647)]
        Commands::RecoverNames {
            pack,
            kind,
            max_index,
            dictionaries,
        } => {
            let count = unpack::names::recover(&pack, &kind, &dictionaries, max_index)?;
            tracing::info!("Recovered {count} {kind} names");
        }
        #[cfg(since_647)]
        Commands::UnpackConfigs {
            expected,
            output,
            pack,
        } => unpack::unpack_configs_only(&expected, &output, &pack)?,
        #[cfg(since_647)]
        Commands::UnpackClientscripts {
            expected,
            output,
            pack,
        } => {
            let count = unpack::cs2::unpack_cs2_scripts(&expected, &output, &pack)?;
            tracing::info!("Decompiled and byte-verified {count} clientscripts");
        }
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
                "JS5 cache packed in memory: {} npcs, {} objs, {} locs, {} seqs, {} enums, {} structs, {} invs, {} varbits",
                result.store.npcs.count(),
                result.store.objs.count(),
                result.store.locs.count(),
                result.store.seqs.count(),
                result.store.enums.count(),
                result.store.structs.count(),
                result.store.invs.count(),
                result.store.varbits.count(),
            );
        }
        #[cfg(before_647)]
        Commands::ToPyxel { source } => pyxel::content_to_pyxel(&source)?,
        #[cfg(before_647)]
        Commands::FromPyxel { source } => pyxel::content_from_pyxel(&source)?,
        Commands::Unpack {
            expected,
            output,
            pack,
        } => {
            let pack = pack.unwrap_or_else(|| output.join("pack"));
            unpack::unpack_all(&expected, &output, &pack)?;
        }
        Commands::Namehash { jag, names } => {
            let hash = if jag { JagFile::hash } else { java_string_hash };
            let print = |name: &str| println!("{}	{name}", hash(name));
            if names.is_empty() {
                for line in stdin().lock().lines() {
                    print(&line?);
                }
            } else {
                names.iter().for_each(|name| print(name));
            }
        }
    }

    Ok(())
}
