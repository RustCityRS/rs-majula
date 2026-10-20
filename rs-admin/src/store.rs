use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{error, warn};

/// A value kept in a JSON file in the console's data directory, read once at
/// startup and written whole on every change.
pub struct JsonFile<T> {
    path: PathBuf,
    inner: Mutex<Loaded<T>>,
}

struct Loaded<T> {
    value: T,
    broken: bool,
}

impl<T: Serialize + DeserializeOwned + Default + Clone> JsonFile<T> {
    pub fn open(path: PathBuf) -> Self {
        let (value, broken) = match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str(&text) {
                Ok(value) => (value, false),
                Err(e) => {
                    error!(
                        "{} is unreadable, so changes to it are refused until it's fixed: {}",
                        path.display(),
                        e
                    );
                    (T::default(), true)
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (T::default(), false),
            Err(e) => {
                error!("Couldn't read {}: {}", path.display(), e);
                (T::default(), true)
            }
        };
        Self {
            path,
            inner: Mutex::new(Loaded { value, broken }),
        }
    }

    pub fn get(&self) -> T {
        self.lock().value.clone()
    }

    /// Changes the value with `f` and writes it out; nothing changes if `f`
    /// refuses or the write fails.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> Result<R, String>) -> Result<R, String> {
        let mut inner = self.lock();
        if inner.broken {
            return Err(format!(
                "{} is unreadable; fix or remove it first.",
                self.path.display()
            ));
        }
        let mut value = inner.value.clone();
        let result = f(&mut value)?;
        self.write(&value).map_err(|e| {
            warn!("Couldn't write {}: {}", self.path.display(), e);
            "The console couldn't save that.".to_owned()
        })?;
        inner.value = value;
        Ok(result)
    }

    /// Writes beside the file and renames over it, so a crash never leaves
    /// it half written.
    fn write(&self, value: &T) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
        std::fs::rename(&tmp, &self.path)
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Loaded<T>> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}
