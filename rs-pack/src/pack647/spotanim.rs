//! Spotanim (graphic effect) packer — all.spotanim text → spotanim.dat/.idx + spotanim.client.dat/.idx.
//!
//! All opcodes go to both server and client (no split needed).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::parse_config_sections;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

use crate::cache::spotanim::SpotAnimTypeRaw;

fn resolve_model(value: &str, registry: &PackRegistry) -> u16 {
    registry
        .model
        .get_by_name(value)
        .map(|id| id as u16)
        .or_else(|| value.strip_prefix("model_").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| value.parse().unwrap_or(0))
}

fn resolve_seq(value: &str, registry: &PackRegistry) -> u16 {
    registry
        .seq
        .get_by_name(value)
        .map(|id| id as u16)
        .or_else(|| value.strip_prefix("seq_").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| value.parse().unwrap_or(0))
}

pub fn pack_spotanims(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let all_file = source_dir.join("_unpack").join("all.spotanim");
    if !all_file.exists() {
        info!("  No all.spotanim found, skipping");
        return Ok(());
    }

    let configs = parse_config_sections(&[all_file], constants);
    info!("  Parsed {} spotanim configs", configs.len());

    let mut server = PackedData::new(registry.spotanim.max);
    let mut client = PackedData::new(registry.spotanim.max);

    for id in 0..registry.spotanim.max {
        let Some(name) = registry.spotanim.get_by_id(id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        let Some(props) = configs.get(name) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        server.start_entry();
        client.start_entry();

        // Collect batched recol/retex before encoding
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut retex_s: Vec<(usize, u16)> = Vec::new();
        let mut retex_d: Vec<(usize, u16)> = Vec::new();

        for (key, value) in props {
            if let Some(rest) = key.strip_prefix("recol") {
                if let Some(num) = rest.strip_suffix('s') {
                    if let (Ok(idx), Ok(v)) = (num.parse::<usize>(), value.parse::<u16>()) {
                        recol_s.push((idx, v));
                    }
                } else if let Some(num) = rest.strip_suffix('d') {
                    if let (Ok(idx), Ok(v)) = (num.parse::<usize>(), value.parse::<u16>()) {
                        recol_d.push((idx, v));
                    }
                }
                continue;
            }
            if let Some(rest) = key.strip_prefix("retex") {
                if let Some(num) = rest.strip_suffix('s') {
                    if let (Ok(idx), Ok(v)) = (num.parse::<usize>(), value.parse::<u16>()) {
                        retex_s.push((idx, v));
                    }
                } else if let Some(num) = rest.strip_suffix('d') {
                    if let (Ok(idx), Ok(v)) = (num.parse::<usize>(), value.parse::<u16>()) {
                        retex_d.push((idx, v));
                    }
                }
                continue;
            }

            // Write simple opcodes to both streams
            match key.as_str() {
                "model" => {
                    let v = resolve_model(value, registry);
                    server.p1(1);
                    server.p2(v);
                    client.p1(1);
                    client.p2(v);
                }
                "anim" => {
                    let v = resolve_seq(value, registry);
                    server.p1(2);
                    server.p2(v);
                    client.p1(2);
                    client.p2(v);
                }
                "resizeh" => {
                    let v: u16 = value.parse().unwrap_or(128);
                    server.p1(4);
                    server.p2(v);
                    client.p1(4);
                    client.p2(v);
                }
                "resizev" => {
                    let v: u16 = value.parse().unwrap_or(128);
                    server.p1(5);
                    server.p2(v);
                    client.p1(5);
                    client.p2(v);
                }
                "angle" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(6);
                    server.p2(v);
                    client.p1(6);
                    client.p2(v);
                }
                "ambient" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(7);
                    server.p1(v);
                    client.p1(7);
                    client.p1(v);
                }
                "contrast" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(8);
                    server.p1(v);
                    client.p1(8);
                    client.p1(v);
                }
                "op9_animmode3_param8224" => {
                    server.p1(9);
                    client.p1(9);
                }
                "op10_flag" => {
                    server.p1(10);
                    client.p1(10);
                }
                "op11_animmode1" => {
                    server.p1(11);
                    client.p1(11);
                }
                "op12_animmode4" => {
                    server.p1(12);
                    client.p1(12);
                }
                "op13_animmode5" => {
                    server.p1(13);
                    client.p1(13);
                }
                "op14_animmode2_u8" => {
                    let v: u8 = value.parse().unwrap_or(0);
                    server.p1(14);
                    server.p1(v);
                    client.p1(14);
                    client.p1(v);
                }
                "op15_animmode3_u16" => {
                    let v: u16 = value.parse().unwrap_or(0);
                    server.p1(15);
                    server.p2(v);
                    client.p1(15);
                    client.p2(v);
                }
                "op16_animmode3_i32" => {
                    let v: i32 = value.parse().unwrap_or(0);
                    server.p1(16);
                    server.p4(v);
                    client.p1(16);
                    client.p4(v);
                }
                _ => {}
            }
        }

        // Write batched recol (opcode 40)
        if !recol_d.is_empty() {
            recol_d.sort_by_key(|(idx, _)| *idx);
            recol_s.sort_by_key(|(idx, _)| *idx);
            let count = recol_d.len().max(recol_s.len()) as u8;
            for stream in [&mut server, &mut client] {
                stream.p1(40);
                stream.p1(count);
                for i in 0..count as usize {
                    let d = recol_d.get(i).map(|(_, v)| *v).unwrap_or(0);
                    let s = recol_s.get(i).map(|(_, v)| *v).unwrap_or(0);
                    stream.p2(d);
                    stream.p2(s);
                }
            }
        }

        // Write batched retex (opcode 41)
        if !retex_d.is_empty() {
            retex_d.sort_by_key(|(idx, _)| *idx);
            retex_s.sort_by_key(|(idx, _)| *idx);
            let count = retex_d.len().max(retex_s.len()) as u8;
            for stream in [&mut server, &mut client] {
                stream.p1(41);
                stream.p1(count);
                for i in 0..count as usize {
                    let d = retex_d.get(i).map(|(_, v)| *v).unwrap_or(0);
                    let s = retex_s.get(i).map(|(_, v)| *v).unwrap_or(0);
                    stream.p2(d);
                    stream.p2(s);
                }
            }
        }

        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "spotanim")?;
    config_blob.add_packed_data("spotanim", client.dat, client.idx);
    Ok(())
}

pub fn emit_spotanim_props(
    def: &SpotAnimTypeRaw,
    registry: &PackRegistry,
) -> Vec<(String, String)> {
    let recol_s = def.recol_s.as_deref().unwrap_or_default();
    let recol_d = def.recol_d.as_deref().unwrap_or_default();
    let retex_s = def.retex_s.as_deref().unwrap_or_default();
    let retex_d = def.retex_d.as_deref().unwrap_or_default();
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        let name = registry
            .model
            .get_by_id(def.model as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("model_{}", def.model));
        p.push((1, "model".into(), name));
    }
    if let Some(anim) = def.anim {
        let name = registry
            .seq
            .get_by_id(anim as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", anim));
        p.push((2, "anim".into(), name));
    }
    if def.opcode_order.contains(&4) || def.resizeh != 128 {
        p.push((4, "resizeh".into(), def.resizeh.to_string()));
    }
    if def.opcode_order.contains(&5) || def.resizev != 128 {
        p.push((5, "resizev".into(), def.resizev.to_string()));
    }
    if def.opcode_order.contains(&6) || def.angle != 0 {
        p.push((6, "angle".into(), def.angle.to_string()));
    }
    if def.opcode_order.contains(&7) || def.ambient != 0 {
        p.push((7, "ambient".into(), def.ambient.to_string()));
    }
    if def.opcode_order.contains(&8) || def.contrast != 0 {
        p.push((8, "contrast".into(), def.contrast.to_string()));
    }
    if def.op9 {
        p.push((9, "op9_animmode3_param8224".into(), "yes".into()));
    }
    if def.op10 {
        p.push((10, "op10_flag".into(), "yes".into()));
    }
    if def.op11 {
        p.push((11, "op11_animmode1".into(), "yes".into()));
    }
    if def.op12 {
        p.push((12, "op12_animmode4".into(), "yes".into()));
    }
    if def.op13 {
        p.push((13, "op13_animmode5".into(), "yes".into()));
    }
    if def.opcode_order.contains(&14) {
        p.push((14, "op14_animmode2_u8".into(), def.op14.to_string()));
    }
    if def.opcode_order.contains(&15) {
        p.push((15, "op15_animmode3_u16".into(), def.op15.to_string()));
    }
    if def.opcode_order.contains(&16) {
        p.push((16, "op16_animmode3_i32".into(), def.op16.to_string()));
    }
    if !recol_d.is_empty() {
        for i in 0..recol_d.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}d"), recol_d[i].to_string()));
            p.push((40, format!("recol{n}s"), recol_s[i].to_string()));
        }
    }
    if !retex_d.is_empty() {
        for i in 0..retex_d.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}d"), retex_d[i].to_string()));
            p.push((41, format!("retex{n}s"), retex_s[i].to_string()));
        }
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
