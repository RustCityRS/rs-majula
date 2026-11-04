//! MesAnim config packer — .mesanim text → mesanim.dat/mesanim.idx.
//! Server-only. Matches 2004scape `MesAnimConfig.ts`.

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

pub fn pack_mesanims(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
) -> Result<()> {
    let files = collect_config_files(source_dir, "mesanim");
    info!("  Found {} .mesanim files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} mesanim configs", configs.len());

    let mut packed = PackedData::new(registry.mesanim.max);
    for id in 0..registry.mesanim.max {
        packed.start_entry();
        let Some(name) = registry.mesanim.get_by_id(id) else {
            packed.finish_entry();
            continue;
        };
        let Some(props) = configs.get(name) else {
            packed.finish_entry();
            continue;
        };

        // len1=seq .. len4=seq → opcodes 1..4
        for (key, value) in props {
            if let Some(idx_str) = key.strip_prefix("len") {
                if let Ok(idx) = idx_str.parse::<u8>() {
                    if (1..=4).contains(&idx) {
                        if let Some(seq_id) = registry.seq.get_by_name(value) {
                            packed.p1(idx);
                            packed.p2(seq_id as u16);
                        }
                    }
                }
            }
        }

        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
    }
    packed.save(output_dir, "mesanim")
}
