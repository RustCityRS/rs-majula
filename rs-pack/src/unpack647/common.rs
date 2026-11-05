//! Shared scaffolding for per-type unpackers.
//!
//! Every unpack module (npc, obj, loc, ...) follows the same shape: open the
//! cache, load every config of one type, and emit a `[name]` block for each
//! one into `_unpack/all.<type>`.
//!
//! The output is a function of the cache and the name registries alone. It
//! holds every cache definition, including those an authored file also
//! defines, and nothing the cache does not carry. Authored content (a
//! barrier's `category`, an obj's `wearpos`, a definition restored from a
//! donor cache) lives in its own files and is laid over this output at pack
//! time by `pack::overlay_section`, so a regeneration can never lose it.

use crate::cache::provider::CacheType;
use std::path::Path;

/// The key of a `key=value` config line, or `None` if it has no `=`.
pub fn line_key(line: &str) -> Option<&str> {
    line.trim().split_once('=').map(|(k, _)| k.trim())
}

/// Writes one `[section]` of a regenerated `all.*` file: exactly the
/// properties decoded from the cache.
pub fn write_config_section(
    out: &mut impl std::io::Write,
    section: &str,
    emitted: impl IntoIterator<Item = (String, String)>,
) -> std::io::Result<()> {
    writeln!(out, "[{section}]")?;
    for (k, v) in emitted {
        writeln!(out, "{k}={v}")?;
    }
    writeln!(out)
}

/// A section parsed from a specific config file, separating client-emittable
/// lines from server-only lines that must be preserved verbatim.
pub struct ParsedSection {
    pub name: String,
    /// Server-only key=value lines (e.g. `param=woodcutting_anim,...`).
    pub server_only_lines: Vec<String>,
    /// ALL original lines (excluding the header), for donor sections not in cache.
    pub all_lines: Vec<String>,
}

/// Parse a specific config file into sections, extracting server-only lines.
///
/// For each `[section_name]`, lines whose key is in `server_only_keys` are
/// collected into `server_only_lines`. ALL non-empty lines are also kept in
/// `all_lines` so donor sections can be written back verbatim.
pub fn parse_specific_file(file_path: &Path, server_only_keys: &[&str]) -> Vec<ParsedSection> {
    let Ok(text) = std::fs::read_to_string(file_path) else {
        return Vec::new();
    };
    let mut sections = Vec::new();
    let mut current: Option<ParsedSection> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(sec) = current.take() {
                sections.push(sec);
            }
            current = Some(ParsedSection {
                name: name.to_string(),
                server_only_lines: Vec::new(),
                all_lines: Vec::new(),
            });
            continue;
        }
        if let Some(ref mut sec) = current {
            sec.all_lines.push(raw_line.to_string());
            if let Some((key, _)) = line.split_once('=') {
                let key = key.trim();
                if server_only_keys.contains(&key) {
                    sec.server_only_lines.push(raw_line.to_string());
                }
            }
        }
    }
    if let Some(sec) = current {
        sections.push(sec);
    }
    sections
}

/// Update a specific config file in-place, regenerating client fields from
/// the original cache while preserving server-only additions.
///
/// For each `[section]` in the file:
///   1. Look up the pack ID via the registry callback.
///   2. Read the raw entry from the cache.
///   3. Decode into a struct and emit text via the provided callback.
///   4. Write back: emitter output (correct opcode ordering/defaults) +
///      server-only lines from the existing file.
///
/// Sections whose pack ID doesn't exist in the cache are left unchanged
/// (they are donor content or server-only definitions).
pub fn update_specific_file<T, F, L>(
    cache: &mut rs_dat2::CacheStore,
    file_path: &Path,
    archive_id: u8,
    stride: u32,
    server_only_keys: &[&str],
    lookup_id: L,
    emit_fn: F,
) -> anyhow::Result<()>
where
    T: CacheType,
    F: Fn(&T) -> Vec<(String, String)>,
    L: Fn(&str) -> Option<u32>,
{
    use std::io::Write;

    let sections = parse_specific_file(file_path, server_only_keys);
    if sections.is_empty() {
        return Ok(());
    }

    let mut output = Vec::new();
    let mut updated = 0u32;
    let mut kept = 0u32;

    for sec in &sections {
        let Some(global_id) = lookup_id(&sec.name) else {
            // No pack ID — write section as-is (shouldn't happen for known configs)
            writeln!(output, "[{}]", sec.name)?;
            for line in &sec.all_lines {
                writeln!(output, "{line}")?;
            }
            writeln!(output)?;
            kept += 1;
            continue;
        };

        let group_id = global_id / stride;
        let entry_id = global_id % stride;

        // Try to read from cache
        let cache_entry = cache.read_group(archive_id, group_id)?;
        let entry_data = cache_entry
            .as_ref()
            .and_then(|g| g.entries.iter().find(|e| e.id == entry_id))
            .filter(|e| !e.data.is_empty());

        let Some(entry) = entry_data else {
            // Not in cache — this is donor content; write section as-is
            writeln!(output, "[{}]", sec.name)?;
            for line in &sec.all_lines {
                writeln!(output, "{line}")?;
            }
            writeln!(output)?;
            kept += 1;
            continue;
        };

        // Decode from cache and emit
        let mut def = T::new(global_id as u16);
        def.decode(&mut rs_io::Packet::from(entry.data.to_vec()));

        let emitted = emit_fn(&def);

        writeln!(output, "[{}]", sec.name)?;
        for (k, v) in &emitted {
            writeln!(output, "{k}={v}")?;
        }
        // Append server-only lines
        for line in &sec.server_only_lines {
            writeln!(output, "{line}")?;
        }
        writeln!(output)?;
        updated += 1;
    }

    std::fs::write(file_path, output)?;
    tracing::info!(
        "  Updated {} sections in {} ({} kept as-is)",
        updated,
        file_path.display(),
        kept
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A section is exactly what the cache decode produced.
    #[test]
    fn writes_the_cache_props_and_nothing_else() {
        let mut out = Vec::new();
        let pairs = vec![
            ("model".to_string(), "sword".to_string()),
            ("cost".to_string(), "26".to_string()),
        ];
        write_config_section(&mut out, "bronze_sword", pairs).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "[bronze_sword]\nmodel=sword\ncost=26\n\n"
        );
    }
}
