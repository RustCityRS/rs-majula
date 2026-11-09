use rs_pack::cache::script::{GOSUB_WITH_PARAMS, ScriptProvider};
use rs_pack::compile_scripts;
use rs_pack::pack::incremental;
use std::env;
use std::fs::{self, File, FileTimes};
use std::path::Path;
use std::process::{self, Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const CHILD: &str = "RS_TEST_SCRIPT_WORLD";
const TEST: &str = "script_ids_survive_parallel_worlds_reload_and_cache_reuse";

struct Worlds(Vec<Child>);

impl Worlds {
    fn start(&mut self, root: &Path, mode: &str) {
        self.0.push(
            Command::new(env::current_exe().unwrap())
                .args(["--exact", TEST, "--nocapture"])
                .env(CHILD, mode)
                .env("RS_TEST_SCRIPT_ROOT", root)
                .stdout(Stdio::null())
                .spawn()
                .unwrap(),
        );
    }

    fn finish(&mut self) {
        for child in &mut self.0 {
            assert!(child.wait().unwrap().success());
        }
        self.0.clear();
    }
}

impl Drop for Worlds {
    fn drop(&mut self) {
        for child in &mut self.0 {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
    }
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let start = Instant::now();
    while !ready() {
        assert!(start.elapsed() < Duration::from_secs(30), "world timed out");
        thread::sleep(Duration::from_millis(10));
    }
}

fn write_source(path: &Path, text: &str) {
    fs::write(path, text).unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(1)))
        .unwrap();
}

fn compile(root: &Path, cached: bool) -> ScriptProvider {
    let session = cached.then(|| incremental::open(&root.join("cache")).unwrap());
    let scripts = compile_scripts(root, &root.join("pack")).unwrap();
    if let Some(session) = session {
        session.finish().unwrap();
    }
    assert_eq!(fs::read_dir(root.join("pack")).unwrap().count(), 0);
    scripts
}

fn id(scripts: &ScriptProvider, name: &str) -> i32 {
    *scripts.names.get(name).unwrap()
}

fn world(root: &Path, mode: &str) {
    if mode == "cached" {
        let cached = compile(root, true);
        assert_eq!(id(&cached, "[proc,beta]"), 0);
        assert_eq!(id(&cached, "[proc,alpha]"), 3);
        let rebuilt = compile(root, false);
        assert_eq!(cached.names, rebuilt.names);
        return;
    }
    if mode == "conflicting" {
        let fresh = compile(root, false);
        assert_eq!(id(&fresh, "[proc,alpha]"), 0);
        assert_eq!(id(&fresh, "[proc,beta]"), 1);
        let reused = compile(root, true);
        assert_eq!(fresh.names, reused.names);
        return;
    }

    let initial = compile(root, false);
    assert_eq!(initial.count(), 3);
    assert_eq!(id(&initial, "[proc,beta]"), 0);
    assert_eq!(id(&initial, "[proc,gamma]"), 1);
    assert_eq!(id(&initial, "[proc,retired]"), 2);
    let suspended = initial.get_by_name("[proc,beta]").unwrap().clone();
    let pc = suspended
        .opcodes
        .iter()
        .position(|&op| op == GOSUB_WITH_PARAMS)
        .unwrap();
    assert_eq!(suspended.int_operands[pc], 1);
    fs::write(root.join(format!("ready-{mode}")), "").unwrap();
    wait_until(|| root.join("reload").exists());

    let reloaded = compile(root, true);
    assert_eq!(id(&reloaded, "[proc,beta]"), 0);
    assert_eq!(id(&reloaded, "[proc,gamma]"), 1);
    assert_eq!(id(&reloaded, "[proc,alpha]"), 3);
    assert!(reloaded.get_by_id(2).is_none());
    assert_eq!(
        reloaded
            .get_by_id(suspended.int_operands[pc])
            .unwrap()
            .info
            .name
            .as_ref(),
        "[proc,gamma]"
    );
}

#[test]
fn script_ids_survive_parallel_worlds_reload_and_cache_reuse() {
    if let Ok(mode) = env::var(CHILD) {
        world(Path::new(&env::var("RS_TEST_SCRIPT_ROOT").unwrap()), &mode);
        return;
    }
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("runtime-script-ids-{}", process::id()));
    let scripts = root.join("scripts/runescript");
    fs::create_dir_all(&scripts).unwrap();
    fs::create_dir_all(root.join("pack")).unwrap();
    let path = scripts.join("scripts.rs2");
    write_source(
        &path,
        "[proc,retired]()(int)\nreturn(9);\n[proc,gamma]()(int)\nreturn(7);\n[proc,beta]()(int)\nreturn(~gamma);\n",
    );
    let mut worlds = Worlds(Vec::new());
    for n in 0..4 {
        worlds.start(&root, &n.to_string());
    }
    wait_until(|| {
        assert!(
            worlds
                .0
                .iter_mut()
                .all(|child| child.try_wait().unwrap().is_none())
        );
        (0..4).all(|n| root.join(format!("ready-{n}")).exists())
    });
    write_source(
        &path,
        "[proc,alpha]()(int)\nreturn(1);\n[proc,beta]()(int)\nreturn(~gamma);\n[proc,gamma]()(int)\nreturn(8);\n",
    );
    fs::write(root.join("reload"), "").unwrap();
    worlds.finish();
    worlds.start(&root, "cached");
    worlds.finish();
    worlds.start(&root, "conflicting");
    worlds.finish();
    fs::remove_dir_all(root).unwrap();
}
