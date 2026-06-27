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
        /// Strict verification mode
        #[arg(long, default_value = "true")]
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
    }

    Ok(())
}
