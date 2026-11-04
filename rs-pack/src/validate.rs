//! Pack↔content consistency checks.
//!
//! Catches the class of bug where a `.pack` registry entry is renamed but the
//! matching content section headers and by-name references aren't updated in
//! lockstep. Because the packers resolve by *name* and silently fall back
//! (skip the entry, or resolve a reference to `-1`), such a desync changes the
//! packed bytes with no error — surfacing only much later as an opaque CRC-gate
//! failure on a config archive.
//!
//! Concrete example this guards against: renaming `varp.pack` `139=varp_139`
//! → `139=legendsquest` without renaming the `[varp_139]` section in
//! `all.varp` (drops the client varp entry → `config` archive CRC moves) and
//! without updating `multivarp=varp_139` in `all.npc` (resolves to `-1` →
//! `config_npc` archive CRC moves).
//!
//! Two checks:
//!   1. **Orphan section** — a `[name]` header whose name has no entry in its
//!      type's `.pack` registry.
//!   2. **Unresolved reference** — a by-name cross-reference (e.g. npc
//!      `multivarp=`) whose value resolves to no entry in the target registry.

use std::path::{Path, PathBuf};

use crate::pack::collect_config_files;
use crate::pack_registry::{PackFile, PackRegistry};

/// Severity of a [`Diagnostic`].
///
/// `Error` is a definite desync that *will* change packed bytes — an orphaned
/// auto-name section (`varp_139`, which exists only to mirror the registry) or
/// an unresolved by-name reference. `Warning` is a hand-authored section that
/// isn't registered: a real latent issue, but not necessarily a rename mistake,
/// so it shouldn't hard-fail the gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// One pack↔content inconsistency, with the source location that triggered it.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warn",
        };
        write!(
            f,
            "[{tag}] {}:{}: {}",
            self.file.display(),
            self.line,
            self.message
        )
    }
}

/// If `name` is the auto-generated `<ext>_<id>` form (e.g. `varp_139`), return
/// the id. These names exist only to mirror the registry, so an orphan of this
/// form is always a registry desync.
fn autoname_id(ext: &str, name: &str) -> Option<u32> {
    name.strip_prefix(ext)?
        .strip_prefix('_')
        .filter(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()))
        .and_then(|r| r.parse().ok())
}

/// A value the packers resolve *without* the registry: the `null` sentinel or
/// a bare integer id. Anything else must resolve by name.
fn is_literal(value: &str) -> bool {
    value == "null" || value.parse::<i64>().is_ok()
}

/// Walk a config file line by line, invoking `on_section` for `[name]` headers
/// and `on_kv` for `key=value` lines (comments and blanks skipped). Line
/// numbers are 1-based.
fn for_each_line(
    file: &Path,
    mut on_section: impl FnMut(usize, &str),
    mut on_kv: impl FnMut(usize, &str, &str),
) {
    let Ok(text) = std::fs::read_to_string(file) else {
        return;
    };
    for (i, raw) in text.lines().enumerate() {
        // Strip `//` comments, then trim.
        let line = raw.split("//").next().unwrap_or(raw).trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            on_section(i + 1, name.trim());
        } else if let Some((k, v)) = line.split_once('=') {
            on_kv(i + 1, k.trim(), v.trim());
        }
    }
}

/// Check that every content config section and by-name reference resolves
/// against its `.pack` registry. Returns one [`Diagnostic`] per desync.
///
/// `content_dir` is the repo `content/` root (sections live under
/// `content/scripts/`).
pub fn check_pack_content_sync(content_dir: &Path, reg: &PackRegistry) -> Vec<Diagnostic> {
    let scripts_dir = content_dir.join("scripts");
    let mut diags = Vec::new();

    // ── Check 1: orphan section headers ──────────────────────────────────
    // (file extension, registry for that type, label for messages)
    let types: [(&str, &PackFile, &str); 16] = [
        ("npc", &reg.npc, "npc"),
        ("obj", &reg.obj, "obj"),
        ("loc", &reg.loc, "loc"),
        ("varp", &reg.varp, "varp"),
        ("varbit", &reg.varbit, "varbit"),
        ("seq", &reg.seq, "seq"),
        ("spotanim", &reg.spotanim, "spotanim"),
        ("inv", &reg.inv, "inv"),
        ("enum", &reg.r#enum, "enum"),
        ("struct", &reg.r#struct, "struct"),
        ("idk", &reg.idk, "idk"),
        ("flo", &reg.flo, "flo"),
        ("flu", &reg.flu, "flu"),
        ("hunt", &reg.hunt, "hunt"),
        ("param", &reg.param, "param"),
        ("mesanim", &reg.mesanim, "mesanim"),
    ];
    for (ext, pack, label) in types {
        for file in collect_config_files(&scripts_dir, ext) {
            for_each_line(
                &file,
                |line, name| {
                    if pack.get_by_name(name).is_some() {
                        return; // resolves — fine
                    }
                    // Orphan. Is it an auto-name (`varp_139`) whose id the
                    // registry now names something else? That's a rename desync.
                    let (severity, message) = match autoname_id(ext, name) {
                        Some(id) => match pack.get_by_id(id) {
                            Some(real) => (
                                Severity::Error,
                                format!(
                                    "section [{name}] but {label}.pack names id {id} '{real}' — \
                                     rename this header to [{real}] (or revert the {label}.pack rename)"
                                ),
                            ),
                            None => (
                                Severity::Error,
                                format!("section [{name}] but {label}.pack has no id {id}"),
                            ),
                        },
                        None => (
                            Severity::Warning,
                            format!(
                                "section [{name}] is not registered in {label}.pack (won't be packed)"
                            ),
                        ),
                    };
                    diags.push(Diagnostic {
                        severity,
                        file: file.clone(),
                        line,
                        message,
                    });
                },
                |_, _, _| {},
            );
        }
    }

    // ── Check 2: unresolved by-name references (npc multi* family) ────────
    // (key, target registry, target label). `multi1..N` is handled by prefix.
    let npc_refs: [(&str, &PackFile, &str); 3] = [
        ("multivar", &reg.varbit, "varbit"),
        ("multivarp", &reg.varp, "varp"),
        ("multidefault", &reg.npc, "npc"),
    ];
    for file in collect_config_files(&scripts_dir, "npc") {
        for_each_line(
            &file,
            |_, _| {},
            |line, key, value| {
                let target = npc_refs
                    .iter()
                    .find(|(k, _, _)| *k == key)
                    .map(|(_, pack, label)| (*pack, *label))
                    .or_else(|| {
                        // multi1, multi2, … → npc transforms
                        let is_multi_n = key.strip_prefix("multi").is_some_and(|rest| {
                            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit())
                        });
                        is_multi_n.then_some((&reg.npc, "npc"))
                    });
                if let Some((pack, label)) = target {
                    if !is_literal(value) && pack.get_by_name(value).is_none() {
                        diags.push(Diagnostic {
                            severity: Severity::Error,
                            file: file.clone(),
                            line,
                            message: format!(
                                "reference {key}={value} does not resolve to any {label} in \
                                 {label}.pack — renamed in {label}.pack but not here?"
                            ),
                        });
                    }
                }
            },
        );
    }

    diags.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    diags
}
