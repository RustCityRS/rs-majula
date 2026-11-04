//! Loads one engine for the whole run and drives each test label through it.

use rs_engine::{Engine, TickStats};
use rs_pack::cache::CacheStore;
use rs_pack::cache::script::ScriptProvider;
use rs_vm::ops::test::{self, AssertionResult};
use rs_vm::state::ScriptState;
use rsmod::rsmod::collision::collision_strategy::CollisionType;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, watch};
use tracing::info;

/// A test that has not finished after this many ticks has hung.
const MAX_TICKS: u32 = 1000;

/// The outcome of one test label.
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub duration: Duration,
    pub assertions: Vec<AssertionResult>,
    pub error: Option<String>,
}

/// The revision's content directory, e.g. `content/647`.
pub fn content_dir() -> PathBuf {
    PathBuf::from(rs_pack::CONTENT_DIR)
}

/// Packs the content, compiles its scripts *with* their `#testscript`
/// sections, and starts an engine on the result with the test commands
/// registered.
///
/// Paths are relative to the workspace root, as they are for the server.
pub fn load_engine() -> anyhow::Result<Engine> {
    let content = content_dir();
    let pack = PathBuf::from(rs_pack::PACK_DIR);

    // A test binary's tests load their engines on parallel threads, and the
    // pack is files on disk: one thread repacking a stale tree while another
    // read it read a map square without its locs. The first thread brings the
    // pack up to date while the rest wait; then every thread finds the stamps
    // fresh and only reads.
    static FRESH: std::sync::Once = std::sync::Once::new();
    FRESH.call_once(|| {
        let _ = rs_pack::pack_all(&content, &pack, false, true);
    });

    // Stamp-checked: a no-op when the server has already packed this tree.
    let rs_pack::startup::PackResult {
        cache_dir,
        config,
        layout,
        mut js5_store,
    } = rs_pack::pack_all(&content, &pack, false, true)?;

    let keys = content.join("keys.json");
    rs_dat2::xtea::XteaKeyStore::load(&keys)?.install_global();

    let server_dir = cache_dir.join("server");
    let store = CacheStore::from_packed_config(config, &mut js5_store, &server_dir)?;
    let cache_ptr = Box::into_raw(store);
    // The engine and every script borrow the cache for the process lifetime,
    // exactly as the server's does.
    let cache: &'static CacheStore = unsafe { &*cache_ptr };

    let (dat, idx) =
        runec::compile_memory_with_tests(&content.join("scripts"), Some(&pack), false, true)
            .map_err(|e| anyhow::anyhow!("compile with tests: {e}"))?;
    let scripts = ScriptProvider::from_bytes(&dat, &idx);
    info!("compiled {} scripts with tests", scripts.count());

    let (_login_tx, login_rx) = mpsc::unbounded_channel();
    let (stats_tx, _stats_rx) = watch::channel(TickStats::default());
    let (reload_tx, _reload_rx) = mpsc::unbounded_channel();
    let (_ether_tx, ether_rx) = mpsc::unbounded_channel();
    let (_db_tx, db_rx) = mpsc::unbounded_channel();
    let (mut engine, _clock) = Engine::new(
        true, 1, false, login_rx, scripts, cache, cache_ptr, layout, stats_tx, reload_tx, 10, None,
        ether_rx, None, db_rx,
    );
    engine.harness_enable_tests();
    Ok(engine)
}

/// Loads an engine from the workspace's content and runs `f` on it.
///
/// For tests: the working directory moves to the workspace root first, since
/// the content, pack and cache paths are relative to it, and `f` runs on a
/// thread with room for the engine's map and player arrays.
pub fn on_loaded_engine<R: Send + 'static>(f: impl FnOnce(&mut Engine) -> R + Send + 'static) -> R {
    std::env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
        .expect("workspace root");
    // rs-pathfinder's collision map and pathfinder are process statics whose
    // first mutable touch is unsynchronized (once_cell `Lazy::force_mut`): two
    // tests loading engines at once raced it, and the loser panicked "Lazy
    // instance has previously been poisoned". One pathfind here, once, before
    // any engine loads, initializes both.
    static PATHFINDER: std::sync::Once = std::sync::Once::new();
    PATHFINDER.call_once(|| {
        rsmod::find_path(
            0,
            0,
            0,
            0,
            0,
            1,
            1,
            1,
            0,
            -1,
            false,
            0,
            1,
            CollisionType::Normal,
        );
    });
    // Those statics are shared by every engine in the process, and the
    // initializing pathfind above does not stop two engines touching them at
    // once: CI still poisoned them with two tests of one binary running
    // (equip, doors). Engines take turns. A test that panicked leaves the
    // lock poisoned; the next one carries on.
    static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _turn = TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    std::thread::Builder::new()
        .stack_size(16 * 1024 * 1024)
        .spawn(move || {
            let mut engine = load_engine().expect("load engine");
            f(&mut engine)
        })
        .expect("spawn engine thread")
        .join()
        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
}

/// Every test label under `scripts_dir`, sorted, optionally narrowed to
/// those whose name contains `filter`.
///
/// A test label is a `[label,...]` header below the file's `#testscript`
/// line; labels above it are production code.
pub fn discover_labels(scripts_dir: &Path, filter: Option<&str>) -> Vec<String> {
    let mut files = Vec::new();
    collect_rs2(scripts_dir, &mut files);
    let mut labels = Vec::new();
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let start = if source.starts_with("#testscript") {
            0
        } else if let Some(pos) = source.find("\n#testscript") {
            pos
        } else {
            continue;
        };
        for line in source[start..].lines() {
            let Some(rest) = line.trim().strip_prefix("[label,") else {
                continue;
            };
            let Some(end) = rest.find(']') else {
                continue;
            };
            let name = &rest[..end];
            if filter.is_none_or(|f| name.contains(f)) {
                labels.push(name.to_string());
            }
        }
    }
    labels.sort();
    labels
}

fn collect_rs2(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs2(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs2") {
            out.push(path);
        }
    }
}

/// Runs one test label on a freshly reset world.
///
/// The label enters the world queue with no delay, as a scheduled world
/// script does, and the engine ticks until it finishes. So `test_tick`,
/// `wait_for` and any `p_delay` a trigger takes see production timing.
///
/// A test fails when an assertion fails, when it runs no assertions at all
/// (that is almost always a script that errored out before reaching them),
/// or when it is still running after [`MAX_TICKS`]. `expect_fail` inverts
/// the assertion verdict but not the other two.
pub fn run_label(engine: &mut Engine, name: &str) -> TestResult {
    let start = Instant::now();
    let fail = |error: String| TestResult {
        name: name.to_string(),
        passed: false,
        duration: start.elapsed(),
        assertions: test::take_results(),
        error: Some(error),
    };

    engine.harness_reset();
    test::clear();

    let Some(script) = engine
        .scripts
        .get_by_name(&format!("[label,{name}]"))
        .cloned()
    else {
        return fail(format!("label {name} is not in the compiled scripts"));
    };
    engine.enqueue_world_script(ScriptState::init(script, None, None, None), 0);

    let mut ticks = 0;
    while !engine.world_queue.is_empty() {
        if ticks >= MAX_TICKS {
            return fail(format!("still running after {MAX_TICKS} ticks"));
        }
        if engine.cycle() == rs_engine::CycleResult::Fatal {
            return fail("an engine phase panicked".into());
        }
        ticks += 1;
    }

    let assertions = test::take_results();
    let failed = assertions.iter().any(|a| !a.passed);
    let error = assertions
        .is_empty()
        .then(|| "ran no assertions; the script likely errored before reaching them".to_string());
    let passed = error.is_none() && failed == test::is_expect_fail();
    TestResult {
        name: name.to_string(),
        passed,
        duration: start.elapsed(),
        assertions,
        error,
    }
}

/// Prints one line per test, the failing assertions under each failure, and
/// a summary. Returns whether every test passed.
pub fn report(results: &[TestResult]) -> bool {
    let mut failed = 0;
    for r in results {
        if r.passed {
            println!("  PASS  {} ({:?})", r.name, r.duration);
            continue;
        }
        failed += 1;
        println!("  FAIL  {} ({:?})", r.name, r.duration);
        if let Some(e) = &r.error {
            println!("        {e}");
        }
        for a in r.assertions.iter().filter(|a| !a.passed) {
            println!("        pc {}: {}", a.pc, a.message);
        }
    }
    println!(
        "\n{} passed, {failed} failed, {} total",
        results.len() - failed,
        results.len()
    );
    failed == 0
}
