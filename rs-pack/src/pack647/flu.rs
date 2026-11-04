//! Flu (floor underlay) packer — all.flu text → flu.dat/.idx + flu.client.dat/.idx.
//!
//! Binary format: op1=Color(p3), op2=Texture(p2), op3=Size(p2),
//! op4=flag, op5=flag, op250=DebugName(pjstr), terminator p1(0).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::parse_config_sections;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

use crate::cache647::FluType;

pub fn pack_flus(
    source_dir: &Path,
    _registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let all_file = source_dir.join("_unpack").join("all.flu");
    if !all_file.exists() {
        info!("  No all.flu found, skipping flu pack");
        return Ok(());
    }

    let configs = parse_config_sections(&[all_file], constants);
    info!("  Parsed {} flu configs", configs.len());

    let max_id = configs
        .keys()
        .filter_map(|name| {
            name.strip_prefix("flu_")
                .and_then(|s| s.parse::<u32>().ok())
        })
        .max()
        .map(|m| m + 1)
        .unwrap_or(0);

    let mut server = PackedData::new(max_id);
    let mut client = PackedData::new(max_id);

    for id in 0..max_id {
        server.start_entry();
        client.start_entry();

        let section_name = format!("flu_{}", id);
        let Some(props) = configs.get(&section_name) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                "color" => {
                    let v: u32 = value.parse().unwrap_or(0);
                    server.p1(1);
                    server.p3(v);
                    client.p1(1);
                    client.p3(v);
                }
                "texture" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(2);
                    server.p2(v);
                    client.p1(2);
                    client.p2(v);
                }
                "size" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(3);
                    server.p2(v);
                    client.p1(3);
                    client.p2(v);
                }
                "op4_clearbool201" => {
                    server.p1(4);
                    client.p1(4);
                }
                "op5_clearbool202" => {
                    server.p1(5);
                    client.p1(5);
                }
                _ => {}
            }
        }

        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "flu")?;
    config_blob.add_packed_data("flu", client.dat, client.idx);
    Ok(())
}

pub fn emit_flu_props(def: &FluType, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if let Some(c) = def.color {
        p.push((1, "color".into(), c.to_string()));
    }
    if def.has_texture {
        p.push((2, "texture".into(), def.texture.to_string()));
    }
    if def.has_size {
        p.push((3, "size".into(), def.size.to_string()));
    }
    if def.op4 {
        p.push((4, "op4_clearbool201".into(), "yes".into()));
    }
    if def.op5 {
        p.push((5, "op5_clearbool202".into(), "yes".into()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
