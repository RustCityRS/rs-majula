use rs_io::jag::JagFile;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::path::Path;
use std::rc::Rc;
use tracing::{info, warn};

use super::config::{UnpackedPacks, build_reverse_hsl_table, config_decoders, read_entries};
use crate::pack::pack_registry::PackedFile;

const MAX_ENTITIES_PER_TYPE: usize = 500;
const MAX_PROP_LINES: usize = 40;

enum DiffLine {
    Same(String),
    Add(String),
    Remove(String),
}

pub struct ConfigDiffReport {
    types: Vec<TypeDiff>,
}

struct TypeDiff {
    name: &'static str,
    compared: usize,
    identical: usize,
    byte_only: Vec<(u16, String)>,
    diffs: Vec<EntityDiff>,
}

struct EntityDiff {
    id: u16,
    name: String,
    lines: Vec<DiffLine>,
}

impl ConfigDiffReport {
    pub fn compare(
        jag: &JagFile,
        packed: &HashMap<String, PackedFile>,
        pack_dir: &Path,
        model_textures: Rc<HashMap<u16, HashSet<u16>>>,
    ) -> Self {
        let reverse_hsl = build_reverse_hsl_table();
        let mut types = Vec::new();

        let mut names_seed = UnpackedPacks::new();
        names_seed.load_existing_names(pack_dir);

        for (name, decoder) in config_decoders() {
            let (Some(cache_dat), Some(cache_idx)) = (
                jag.read(&format!("{name}.dat")),
                jag.read(&format!("{name}.idx")),
            ) else {
                continue;
            };
            let Some(client) = packed.get(name).and_then(|p| p.client.as_ref()) else {
                continue;
            };

            let cache_bytes = read_entries(&cache_dat.data, &cache_idx.data);
            let content_bytes = read_entries(&client.dat, &client.idx);

            let mut cache_packs = UnpackedPacks::new();
            cache_packs.existing_model_names = names_seed.existing_model_names.clone();
            cache_packs.existing_config_names = names_seed.existing_config_names.clone();
            cache_packs.model_textures = model_textures.clone();
            let cache_props = into_map(decoder(
                &cache_dat.data,
                &cache_idx.data,
                &reverse_hsl,
                &mut cache_packs,
            ));
            let mut content_packs = UnpackedPacks::new();
            content_packs.existing_model_names = names_seed.existing_model_names.clone();
            content_packs.existing_config_names = names_seed.existing_config_names.clone();
            content_packs.model_textures = model_textures.clone();
            let content_props = into_map(decoder(
                &client.dat,
                &client.idx,
                &reverse_hsl,
                &mut content_packs,
            ));

            let names = names_seed
                .existing_config_names
                .get(name)
                .cloned()
                .unwrap_or_default();
            let cache_b: HashMap<u16, &Vec<u8>> =
                cache_bytes.iter().map(|(i, b)| (*i, b)).collect();
            let content_b: HashMap<u16, &Vec<u8>> =
                content_bytes.iter().map(|(i, b)| (*i, b)).collect();
            let count = cache_bytes.len().max(content_bytes.len());

            let empty: Vec<u8> = Vec::new();
            let no_props: Vec<(String, String)> = Vec::new();
            let mut td = TypeDiff {
                name,
                compared: count,
                identical: 0,
                byte_only: Vec::new(),
                diffs: Vec::new(),
            };

            for id in 0..count as u16 {
                let c = cache_b.get(&id).copied().unwrap_or(&empty);
                let p = content_b.get(&id).copied().unwrap_or(&empty);
                if c == p {
                    td.identical += 1;
                    continue;
                }
                let cp = cache_props.get(&id).unwrap_or(&no_props);
                let pp = content_props.get(&id).unwrap_or(&no_props);
                let content_lines: Vec<String> = pp.iter().map(fmt_prop).collect();
                let cache_lines: Vec<String> = cp.iter().map(fmt_prop).collect();
                let lines = lcs_diff(&content_lines, &cache_lines);
                let changed = lines
                    .iter()
                    .any(|l| matches!(l, DiffLine::Add(_) | DiffLine::Remove(_)));
                let entity = names
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("{name}_{id}"));
                if !changed {
                    td.byte_only.push((id, entity));
                } else {
                    td.diffs.push(EntityDiff {
                        id,
                        name: entity,
                        lines,
                    });
                }
            }

            types.push(td);
        }

        Self { types }
    }

    pub fn write(&self, output_dir: &Path) -> std::io::Result<()> {
        let dir = output_dir.join("_configdiff");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("report.txt");

        let total_prop: usize = self.types.iter().map(|t| t.diffs.len()).sum();
        let total_byte: usize = self.types.iter().map(|t| t.byte_only.len()).sum();
        let changed = self
            .types
            .iter()
            .filter(|t| !t.diffs.is_empty() || !t.byte_only.is_empty())
            .count();

        let mut out = String::new();
        out.push_str(
            "# Per-entity config diff: each entity packed from the committed content/<rev>/\n\
             # vs the same entity in the cache being unpacked. Both sides are decoded through\n\
             # the SAME decoder, so name aliases (obj_229 vs vial_empty), frame index base,\n\
             # and bool synonyms (true/yes) never show as false diffs.\n\
             #\n\
             # ORDER IS SIGNIFICANT: the packer emits opcodes top-down in line order, so the\n\
             # property sequence must match exactly. The diff below is ordered (content -> cache):\n\
             #   + <prop>  in the cache at this point, not content  (add it to match)\n\
             #   - <prop>  in content at this point, not the cache  (remove it to match)\n\
             #     <prop>  same line in both (shown for alignment); a MOVED line shows as - then +\n\
             #   byte-only = sequence identical yet bytes differ (packer encoding, not content)\n",
        );

        if changed == 0 {
            out.push_str("\nAll packed config entities match the cache.\n");
            std::fs::write(&path, &out)?;
            info!(
                "Config-diff report: all entities match the cache - {}",
                path.display()
            );
            return Ok(());
        }

        writeln!(
            out,
            "\nSummary: {total_prop} entity property diff(s), {total_byte} byte-only diff(s) across {changed} type(s)."
        )
        .unwrap();

        for t in &self.types {
            if t.diffs.is_empty() && t.byte_only.is_empty() {
                continue;
            }
            writeln!(
                out,
                "\n## {}: {} of {} entities differ ({} property, {} byte-only, {} identical)",
                t.name,
                t.diffs.len() + t.byte_only.len(),
                t.compared,
                t.diffs.len(),
                t.byte_only.len(),
                t.identical,
            )
            .unwrap();
            for (id, ename) in &t.byte_only {
                writeln!(out, "  = {id:<6} {ename}  (byte-only)").unwrap();
            }
            for d in t.diffs.iter().take(MAX_ENTITIES_PER_TYPE) {
                writeln!(out, "  ~ {:<6} {}", d.id, d.name).unwrap();
                for (shown, line) in d.lines.iter().enumerate() {
                    if shown >= MAX_PROP_LINES {
                        writeln!(out, "      ... ({} more line(s))", d.lines.len() - shown)
                            .unwrap();
                        break;
                    }
                    match line {
                        DiffLine::Add(s) => writeln!(out, "      + {s}").unwrap(),
                        DiffLine::Remove(s) => writeln!(out, "      - {s}").unwrap(),
                        DiffLine::Same(s) => writeln!(out, "        {s}").unwrap(),
                    }
                }
            }
            if t.diffs.len() > MAX_ENTITIES_PER_TYPE {
                writeln!(
                    out,
                    "  ... and {} more entity diff(s) not shown",
                    t.diffs.len() - MAX_ENTITIES_PER_TYPE
                )
                .unwrap();
            }
        }

        std::fs::write(&path, &out)?;
        warn!(
            "Config-diff report: {total_prop} property diff(s), {total_byte} byte-only across {changed} type(s) vs committed content - see {}",
            path.display()
        );
        Ok(())
    }
}

fn into_map(entries: Vec<(u16, Vec<(String, String)>)>) -> HashMap<u16, Vec<(String, String)>> {
    entries.into_iter().collect()
}

fn fmt_prop(p: &(String, String)) -> String {
    format!("{}={}", p.0, p.1)
}

fn lcs_diff(a: &[String], b: &[String]) -> Vec<DiffLine> {
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut out = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffLine::Same(a[i].clone()));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(DiffLine::Remove(a[i].clone()));
            i += 1;
        } else {
            out.push(DiffLine::Add(b[j].clone()));
            j += 1;
        }
    }
    while i < n {
        out.push(DiffLine::Remove(a[i].clone()));
        i += 1;
    }
    while j < m {
        out.push(DiffLine::Add(b[j].clone()));
        j += 1;
    }
    out
}
