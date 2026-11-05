use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::parse_config_sections;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

use crate::cache::flo::FloTypeRaw;

pub fn pack_flos(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let all_file = source_dir.join("_unpack").join("all.flo");
    if !all_file.exists() {
        info!("  No all.flo found, skipping flo pack");
        return Ok(());
    }

    let configs = parse_config_sections(&[all_file], constants);
    info!("  Parsed {} flo configs", configs.len());

    let mut server = PackedData::new(registry.flo.max);
    let mut client = PackedData::new(registry.flo.max);

    for id in 0..registry.flo.max {
        server.start_entry();
        client.start_entry();

        let Some(name) = registry.flo.get_by_id(id) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        let Some(props) = configs.get(name) else {
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
                "texture_u8" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(2);
                    server.p1(v);
                    client.p1(2);
                    client.p1(v);
                }
                "texture" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(3);
                    server.p2(v);
                    client.p1(3);
                    client.p2(v);
                }
                "op5_occlude_false" => {
                    server.p1(5);
                    client.p1(5);
                }
                "water_color" => {
                    let v: u32 = value.parse().unwrap_or(0);
                    server.p1(7);
                    server.p3(v);
                    client.p1(7);
                    client.p3(v);
                }
                "op8_flomark" => {
                    server.p1(8);
                    client.p1(8);
                }
                "op9_u16" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(9);
                    server.p2(v);
                    client.p1(9);
                    client.p2(v);
                }
                "op10_bool34_false" => {
                    server.p1(10);
                    client.p1(10);
                }
                "op11_u8" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(11);
                    server.p1(v);
                    client.p1(11);
                    client.p1(v);
                }
                "op12_bool33_true" => {
                    server.p1(12);
                    client.p1(12);
                }
                "op13_u24" => {
                    let v: u32 = value.parse().unwrap_or(0);
                    server.p1(13);
                    server.p3(v);
                    client.p1(13);
                    client.p3(v);
                }
                "op14_u8" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(14);
                    server.p1(v);
                    client.p1(14);
                    client.p1(v);
                }
                "op16_u8" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(16);
                    server.p1(v);
                    client.p1(16);
                    client.p1(v);
                }
                _ => {}
            }
        }

        server.p1(250);
        server.pjstr(name);
        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "flo")?;
    config_blob.add_packed_data("flo", client.dat, client.idx);
    Ok(())
}

pub fn emit_flo_props(def: &FloTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "color".into(), def.colour.to_string()));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "texture_u8".into(), def.texture_u8_raw.to_string()));
    }
    if def.opcode_order.contains(&3) {
        p.push((3, "texture".into(), def.texture_u16_raw.to_string()));
    }
    if !def.occlude {
        p.push((5, "op5_occlude_false".into(), "yes".into()));
    }
    if def.water_color >= 0 {
        p.push((
            7,
            "water_color".into(),
            (def.water_color as u32).to_string(),
        ));
    }
    if def.op8_flomark {
        p.push((8, "op8_flomark".into(), "yes".into()));
    }
    if let Some(v) = def.op9_u16 {
        p.push((9, "op9_u16".into(), v.to_string()));
    }
    if def.op10_bool34_false {
        p.push((10, "op10_bool34_false".into(), "yes".into()));
    }
    for v in &def.op11_values {
        p.push((11, "op11_u8".into(), v.to_string()));
    }
    if def.op12_bool33_true {
        p.push((12, "op12_bool33_true".into(), "yes".into()));
    }
    if let Some(v) = def.op13_u24 {
        p.push((13, "op13_u24".into(), v.to_string()));
    }
    if let Some(v) = def.op14_u8 {
        p.push((14, "op14_u8".into(), v.to_string()));
    }
    if let Some(v) = def.op16_u8 {
        p.push((16, "op16_u8".into(), v.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
