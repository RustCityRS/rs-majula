use crate::pack::incremental::{Cached, Registry, Stage, bundle, dir_key, files_under, unbundle};
use crate::pack::pack_dir::PrivatePack;
use anyhow::Result;
use runec::ScriptIds;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::time::Instant;
use tracing::debug;

static IDS: LazyLock<Mutex<HashMap<PathBuf, Arc<Mutex<ScriptIds>>>>> =
    LazyLock::new(Mutex::default);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

struct ScriptRegistry(Mutex<ScriptIds>);

impl Registry for ScriptRegistry {
    fn recording(&self) -> Self {
        Self(Mutex::new(lock(&self.0).clone()))
    }

    fn recorded(&self) -> Vec<u8> {
        serde_json::to_vec(&*lock(&self.0)).unwrap()
    }

    fn unchanged(&self, recorded: &[u8]) -> bool {
        let Ok(recorded) = serde_json::from_slice::<ScriptIds>(recorded) else {
            return false;
        };
        let ids = lock(&self.0);
        let occupied: HashMap<_, _> = ids.iter().map(|(name, id)| (*id, name)).collect();
        recorded.iter().all(|(name, id)| {
            ids.get(name).is_none_or(|current| current == id)
                && occupied.get(id).is_none_or(|current| *current == name)
        })
    }
}

struct CompiledScripts {
    dat: Vec<u8>,
    idx: Vec<u8>,
    ids: ScriptIds,
}

impl Cached for CompiledScripts {
    fn encode(&self) -> Vec<u8> {
        bundle(&[
            &self.dat,
            &self.idx,
            &serde_json::to_vec(&self.ids).unwrap(),
        ])
    }

    fn decode(bytes: Vec<u8>) -> Option<Self> {
        let [dat, idx, ids] = <[Vec<u8>; 3]>::try_from(unbundle(&bytes)?).ok()?;
        Some(Self {
            dat,
            idx,
            ids: serde_json::from_slice(&ids).ok()?,
        })
    }
}

pub(crate) fn compile(source: &Path, pack: &Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let project = fs::canonicalize(source)?;
    let ids = lock(&IDS).entry(project).or_default().clone();
    let mut ids = lock(&ids);
    let registry = ScriptRegistry(Mutex::new(ids.clone()));
    let private = PrivatePack::copy(pack)?;
    let scripts = source.join("scripts");
    let stage = Stage::new(
        "scripts",
        vec![("pack".to_owned(), dir_key(private.path(), &[]))],
    );
    let start = Instant::now();
    let compiled = stage.unit_with(
        &registry,
        0,
        &files_under(&runec::sources::roots(&scripts)),
        |registry| {
            let mut ids = lock(&registry.0);
            let (dat, idx) =
                runec::compile_memory_with_ids(&scripts, Some(private.path()), &mut ids)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok(CompiledScripts {
                dat,
                idx,
                ids: ids.clone(),
            })
        },
    )?;
    ids.extend(compiled.ids);
    stage.finish(Vec::new());
    debug!("Compiled scripts in {:.2}s", start.elapsed().as_secs_f64());
    Ok((compiled.dat, compiled.idx))
}
