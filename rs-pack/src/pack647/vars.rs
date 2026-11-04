//! Vars config packer — .vars text → vars.dat/vars.idx.
//! Server-only. Matches 2004scape `VarsConfig.ts`.

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

pub fn pack_vars_configs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
) -> Result<()> {
    let pack = &registry.vars;
    let files = collect_config_files(source_dir, "vars");
    info!("  Found {} .vars files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} vars configs", configs.len());

    let mut packed = PackedData::new(pack.max);
    for id in 0..pack.max {
        packed.start_entry();
        let Some(name) = pack.get_by_id(id) else {
            packed.finish_entry();
            continue;
        };
        let Some(props) = configs.get(name) else {
            packed.finish_entry();
            continue;
        };

        for (key, value) in props {
            if key == "type" {
                packed.p1(1);
                packed.p1(crate::pack::param::type_name_to_char(value) as u8);
            }
        }
        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
    }
    packed.save(output_dir, "vars")
}
