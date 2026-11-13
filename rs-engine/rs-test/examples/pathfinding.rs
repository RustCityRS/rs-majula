#[cfg(since_647)]
use rs_engine::{CycleResult, Engine};
#[cfg(since_647)]
use rs_grid::CoordGrid;
#[cfg(since_647)]
use rs_protocol::network::game::client_prot::ClientProt;
#[cfg(since_647)]
use rs_test::runner::load_engine_cached;
#[cfg(since_647)]
use std::time::{Duration, Instant};
#[cfg(since_647)]
use std::{env, io, thread};

#[cfg(since_647)]
fn number(name: &str, default: usize) -> usize {
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(since_647)]
fn route_hash(engine: &Engine) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for pid in engine.player_list.pids() {
        let pathing = &engine.harness_player(pid).unwrap().pathing;
        for i in 0..=pathing.waypoints.index {
            hash ^= pathing.waypoints[i as usize].packed() as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash ^= pathing.waypoints.index as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(since_647)]
fn run() {
    env::set_current_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/../..")).unwrap();
    let mut engine = load_engine_cached().unwrap();
    let stats = engine.harness_tick_stats();
    let counts: Vec<usize> = env::var("PF_THREADS")
        .unwrap_or_else(|_| "1,2,4".into())
        .split(',')
        .map(|s| s.parse().unwrap())
        .collect();
    let ticks = number("PF_TICKS", 24);
    let warm = 6;
    let mode = env::var("PF_MODE").unwrap_or_else(|_| "input".into());
    let input_only = mode != "tick";
    let flood = mode == "flood";
    let interval = Duration::from_millis(number("PF_INTERVAL_MS", 0) as u64);
    let mut reference = Vec::new();
    let mut clients = Vec::new();
    for (round, threads) in counts.into_iter().enumerate() {
        if round == 0 || !input_only {
            engine.harness_reset();
            if flood {
                for i in 0..2000 {
                    clients.push(
                        engine
                            .harness_login_returning(&format!("pathload{i}"))
                            .unwrap(),
                    );
                }
                for _ in 0..3 {
                    assert!(matches!(engine.cycle(), CycleResult::Continue));
                    for client in &mut clients {
                        client.packets();
                    }
                }
                for (i, client) in clients.iter_mut().enumerate() {
                    engine.harness_telejump(
                        client.pid,
                        3215 + (i / 15 % 15) as u16,
                        0,
                        3215 + (i % 15) as u16,
                    );
                    client.packets();
                }
            } else {
                assert_eq!(
                    engine.harness_local_bots(CoordGrid::new(3222, 0, 3222), 2000),
                    2000
                );
            }
        }
        engine.set_pathfinder_threads(threads).unwrap();
        let mut offset = number("PF_OFFSET_MS", 0);
        if env::var_os("PF_WAIT").is_some() {
            println!("READY threads={threads}");
            let mut command = String::new();
            io::stdin().read_line(&mut command).unwrap();
            offset = command.trim().parse().unwrap_or(offset);
        }
        let start = Instant::now() + Duration::from_millis(offset as u64);
        let mut inputs = Vec::new();
        let mut totals = Vec::new();
        for i in 0..warm + ticks {
            if let Some(delay) =
                (start + interval * i as u32).checked_duration_since(Instant::now())
            {
                thread::sleep(delay);
            }
            engine.clock = 10 + i as u32;
            engine.random.set_seed(12345 + i as i64);
            for (j, client) in clients.iter_mut().enumerate() {
                for k in 0..5 {
                    let x = 3197 + ((i * 13 + j * 7 + k * 11) % 50) as u16;
                    let z = 3197 + ((i * 17 + j * 23 + k * 3) % 50) as u16;
                    client.send(
                        ClientProt::MoveGameClick,
                        &[x as u8, (x >> 8) as u8, 128, (z >> 8) as u8, z as u8],
                    );
                }
            }
            let before = Instant::now();
            let input_ms = if input_only {
                engine.harness_inputs();
                before.elapsed().as_secs_f64() * 1000.0
            } else {
                assert!(matches!(engine.cycle(), CycleResult::Continue));
                stats.borrow().input
            };
            let total_ms = before.elapsed().as_secs_f64() * 1000.0;
            let paths = engine.pathfinding_stats();
            assert_eq!(paths.searches, 2000);
            assert_eq!(paths.requests, if flood { 10000 } else { 2000 });
            if input_only {
                let hash = route_hash(&engine);
                if reference.len() <= i {
                    reference.push(hash);
                } else {
                    assert_eq!(hash, reference[i], "threads={threads}, tick={i}");
                }
            }
            if i >= warm {
                inputs.push(input_ms);
                totals.push(total_ms);
            }
        }
        inputs.sort_by(f64::total_cmp);
        totals.sort_by(f64::total_cmp);
        println!(
            "RESULT threads={} mode={} players=2000 ticks={} input_median_ms={:.3} input_p95_ms={:.3} tick_median_ms={:.3} tick_p95_ms={:.3} requests={} searches=2000 routes_equal={}",
            threads,
            mode,
            ticks,
            inputs[ticks / 2],
            inputs[(ticks * 95 / 100).min(ticks - 1)],
            totals[ticks / 2],
            totals[(ticks * 95 / 100).min(ticks - 1)],
            if flood { 10000 } else { 2000 },
            if input_only { "true" } else { "unchecked" },
        );
    }
}

#[cfg(since_647)]
fn main() {
    thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(run)
        .unwrap()
        .join()
        .unwrap();
}

#[cfg(before_647)]
fn main() {}
