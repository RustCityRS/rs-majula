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
    /// Extract original JAG archives into content files for re-packing
    Unpack {
        /// Directory containing expected JAG files
        #[arg(short, long, default_value = "expected")]
        expected: PathBuf,
        /// Output directory for unpacked content
        #[arg(short, long, default_value = "content_unpack")]
        output: PathBuf,
    },
    /// Generate editable .pyxel sprite docs next to the content TGAs
    ToPyxel {
        /// Content directory holding the sprite TGAs
        #[arg(short, long, default_value = rs_pack::CONTENT_DIR)]
        source: PathBuf,
    },
    /// Rebuild content TGAs from edited .pyxel docs (run before `pack`)
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
        Commands::Unpack { expected, output } => {
            let pack = output.join("pack");
            rs_pack::unpack::unpack_all(&expected, &output, &pack)?;
        }
        Commands::ToPyxel { source } => rs_pack::pyxel::content_to_pyxel(&source)?,
        Commands::FromPyxel { source } => rs_pack::pyxel::content_from_pyxel(&source)?,
    }

    Ok(())
}
