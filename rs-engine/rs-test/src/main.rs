//! `rs-test` -- run the revision's `#testscript` labels headlessly.
//!
//! ```text
//! REV=647 cargo run -p rs-test -- [--filter <substring>]
//! ```
//!
//! Run from the workspace root. Exits non-zero if any test fails.

#[cfg(since_647)]
fn main() {
    use clap::Parser;

    #[derive(Parser)]
    #[command(
        name = "rs-test",
        about = "Run #testscript labels against a headless engine"
    )]
    struct Cli {
        /// Only run labels whose name contains this.
        #[arg(long)]
        filter: Option<String>,
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn,rs_test=info".into()),
        )
        .init();
    let cli = Cli::parse();

    // The engine's map and player arrays are large; give it room.
    let passed = std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || -> anyhow::Result<bool> {
            use rs_test::runner;
            let labels = runner::discover_labels(
                &runner::content_dir().join("scripts/runescript"),
                cli.filter.as_deref(),
            );
            let mut engine = runner::load_engine()?;
            let results: Vec<_> = labels
                .iter()
                .map(|l| runner::run_label(&mut engine, l))
                .collect();
            Ok(runner::report(&results))
        })
        .expect("spawn runner thread")
        .join()
        .expect("runner thread panicked");

    match passed {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}

#[cfg(before_647)]
fn main() {
    eprintln!("rs-test runs rev-647 content only (REV={})", env!("REV"));
    std::process::exit(2);
}
