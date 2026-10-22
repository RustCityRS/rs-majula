use crate::store::JsonFile;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// How long a fixed mark is kept: the worlds forget an error 30 days after
/// it was last seen.
const KEEP_MS: i64 = 31 * 24 * 3_600_000;

/// A script error staff marked fixed: it's set aside until it happens again.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fixed {
    pub script: String,
    pub line: i32,
    pub message: String,
    pub by: String,
    pub at_ms: i64,
}

impl Fixed {
    pub fn is(&self, script: &str, line: i32, message: &str) -> bool {
        self.script == script && self.line == line && self.message == message
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct FixedFile {
    fixed: Vec<Fixed>,
}

pub struct FixedErrors {
    file: JsonFile<FixedFile>,
}

impl FixedErrors {
    pub fn open(path: PathBuf) -> Self {
        Self {
            file: JsonFile::open(path),
        }
    }

    pub fn all(&self) -> Vec<Fixed> {
        self.file.get().fixed
    }

    /// Marks the error fixed (again) as `by` did `at_ms`, or takes the mark
    /// off.
    pub fn set(
        &self,
        script: &str,
        line: i32,
        message: &str,
        fixed: Option<(&str, i64)>,
    ) -> Result<(), String> {
        self.file.update(|file| {
            file.fixed.retain(|f| !f.is(script, line, message));
            if let Some((by, at_ms)) = fixed {
                file.fixed.retain(|f| at_ms - f.at_ms < KEEP_MS);
                file.fixed.push(Fixed {
                    script: script.to_owned(),
                    line,
                    message: message.to_owned(),
                    by: by.to_owned(),
                    at_ms,
                });
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, process};

    #[test]
    fn marks_replace_and_old_ones_go() {
        let path = env::temp_dir().join(format!("rs-admin-fixed-{}.json", process::id()));
        let _ = fs::remove_file(&path);
        let fixed = FixedErrors::open(path.clone());
        fixed.set("[proc,a]", 1, "boom", Some(("Ash", 0))).unwrap();
        fixed.set("[proc,b]", 2, "bang", Some(("Ash", 1))).unwrap();
        fixed.set("[proc,a]", 1, "boom", Some(("Kim", 2))).unwrap();
        let all = fixed.all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].by, "Kim");
        fixed.set("[proc,a]", 1, "boom", None).unwrap();
        assert_eq!(fixed.all().len(), 1);
        // A month on, the old mark is dropped with the next.
        fixed
            .set("[proc,c]", 3, "bonk", Some(("Ash", KEEP_MS + 1)))
            .unwrap();
        assert_eq!(fixed.all().len(), 1);
        // Read back from the file.
        assert_eq!(FixedErrors::open(path.clone()).all()[0].script, "[proc,c]");
        let _ = fs::remove_file(&path);
    }
}
