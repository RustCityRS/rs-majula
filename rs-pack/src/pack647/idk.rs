//! Idk (identity kit) packer — all.idk text → idk.dat/.idx + idk.client.dat/.idx.
//!
//! Binary: op1=BodyType(p1), op2=Models(p1 count, p2×N), op3=Disable flag,
//! op40=Recol(p1 count, (p2,p2)×N), op41=Retex same, op60-69=Head(p2).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::parse_config_sections;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

use crate::cache647::IdkType;

fn parse_bodytype(value: &str) -> u8 {
    match value {
        "man_hair" => 0,
        "man_jaw" => 1,
        "man_torso" => 2,
        "man_arms" => 3,
        "man_hands" => 4,
        "man_legs" => 5,
        "man_feet" => 6,
        "woman_hair" => 7,
        "woman_jaw" => 8,
        "woman_torso" => 9,
        "woman_arms" => 10,
        "woman_hands" => 11,
        "woman_legs" => 12,
        "woman_feet" => 13,
        _ => value.parse().unwrap_or(0),
    }
}

fn resolve_model(value: &str, registry: &PackRegistry) -> u16 {
    registry
        .model
        .get_by_name(value)
        .map(|id| id as u16)
        .or_else(|| value.strip_prefix("model_").and_then(|s| s.parse().ok()))
        .unwrap_or_else(|| value.parse().unwrap_or(0))
}

pub fn pack_idks(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let all_file = source_dir.join("_unpack").join("all.idk");
    if !all_file.exists() {
        info!("  No all.idk found, skipping");
        return Ok(());
    }

    let configs = parse_config_sections(&[all_file], constants);
    info!("  Parsed {} idk configs", configs.len());

    let mut server = PackedData::new(registry.idk.max);
    let mut client = PackedData::new(registry.idk.max);

    for id in 0..registry.idk.max {
        server.start_entry();
        client.start_entry();

        let Some(name) = registry.idk.get_by_id(id) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        let Some(props) = configs.get(name) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        let mut models: Vec<(usize, u16)> = Vec::new();
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();
        let mut retex_s: Vec<(usize, u16)> = Vec::new();
        let mut retex_d: Vec<(usize, u16)> = Vec::new();

        // Track first-appearance order of batched opcode types so we flush
        // in the same order as the source cache (text preserves that order).
        #[derive(Clone, Copy, PartialEq)]
        enum Batched {
            Models,
            Recol,
            Retex,
        }
        let mut flush_order: Vec<Batched> = Vec::new();

        for (key, value) in props {
            if let Some(rest) = key.strip_prefix("model") {
                if let Ok(idx) = rest.parse::<usize>() {
                    if !flush_order.contains(&Batched::Models) {
                        flush_order.push(Batched::Models);
                    }
                    models.push((idx, resolve_model(value, registry)));
                }
                continue;
            }
            if let Some(rest) = key.strip_prefix("recol") {
                if !flush_order.contains(&Batched::Recol) {
                    flush_order.push(Batched::Recol);
                }
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
                if !flush_order.contains(&Batched::Retex) {
                    flush_order.push(Batched::Retex);
                }
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
            if let Some(rest) = key.strip_prefix("head") {
                if let Ok(slot_1based) = rest.parse::<u8>() {
                    let slot = slot_1based.saturating_sub(1);
                    let model_id = resolve_model(value, registry);
                    server.p1(60 + slot);
                    server.p2(model_id);
                    client.p1(60 + slot);
                    client.p2(model_id);
                }
                continue;
            }

            match key.as_str() {
                "type" => {
                    let v = parse_bodytype(value);
                    server.p1(1);
                    server.p1(v);
                    client.p1(1);
                    client.p1(v);
                }
                "disable" => {
                    server.p1(3);
                    client.p1(3);
                }
                _ => {}
            }
        }

        // Flush batched opcodes in source-appearance order
        for batch in &flush_order {
            match batch {
                Batched::Models => {
                    if !models.is_empty() {
                        models.sort_by_key(|(idx, _)| *idx);
                        for stream in [&mut server, &mut client] {
                            stream.p1(2);
                            stream.p1(models.len() as u8);
                            for &(_, model_id) in &models {
                                stream.p2(model_id);
                            }
                        }
                    }
                }
                Batched::Recol => {
                    if !recol_s.is_empty() {
                        recol_s.sort_by_key(|(idx, _)| *idx);
                        recol_d.sort_by_key(|(idx, _)| *idx);
                        let count = recol_s.len() as u8;
                        for stream in [&mut server, &mut client] {
                            stream.p1(40);
                            stream.p1(count);
                            for i in 0..count as usize {
                                let s = recol_s.get(i).map(|(_, v)| *v).unwrap_or(0);
                                let d = recol_d.get(i).map(|(_, v)| *v).unwrap_or(0);
                                stream.p2(s);
                                stream.p2(d);
                            }
                        }
                    }
                }
                Batched::Retex => {
                    if !retex_s.is_empty() {
                        retex_s.sort_by_key(|(idx, _)| *idx);
                        retex_d.sort_by_key(|(idx, _)| *idx);
                        let count = retex_s.len() as u8;
                        for stream in [&mut server, &mut client] {
                            stream.p1(41);
                            stream.p1(count);
                            for i in 0..count as usize {
                                let s = retex_s.get(i).map(|(_, v)| *v).unwrap_or(0);
                                let d = retex_d.get(i).map(|(_, v)| *v).unwrap_or(0);
                                stream.p2(s);
                                stream.p2(d);
                            }
                        }
                    }
                }
            }
        }

        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "idk")?;
    config_blob.add_packed_data("idk", client.dat, client.idx);
    Ok(())
}

fn bodytype_name(v: u8) -> Option<&'static str> {
    Some(match v {
        0 => "man_hair",
        1 => "man_jaw",
        2 => "man_torso",
        3 => "man_arms",
        4 => "man_hands",
        5 => "man_legs",
        6 => "man_feet",
        7 => "woman_hair",
        8 => "woman_jaw",
        9 => "woman_torso",
        10 => "woman_arms",
        11 => "woman_hands",
        12 => "woman_legs",
        13 => "woman_feet",
        _ => return None,
    })
}

fn model_name(registry: &PackRegistry, id: u16) -> String {
    registry
        .model
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}"))
}

pub fn emit_idk_props(def: &IdkType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    // op 1 — body part type
    if let Some(bt) = def.body_type {
        let value = bodytype_name(bt)
            .map(str::to_string)
            .unwrap_or_else(|| bt.to_string());
        p.push((1, "type".into(), value));
    }
    // op 2 — models
    if !def.models.is_empty() {
        for (i, id) in def.models.iter().enumerate() {
            p.push((2, format!("model{}", i + 1), model_name(registry, *id)));
        }
    }
    // op 3 — disable
    if def.disable {
        p.push((3, "disable".into(), "yes".into()));
    }
    // op 40 — recolour pairs
    if !def.recol_s.is_empty() {
        for i in 0..def.recol_s.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}s"), def.recol_s[i].to_string()));
            p.push((40, format!("recol{n}d"), def.recol_d[i].to_string()));
        }
    }
    // op 41 — retexture pairs
    if !def.retex_s.is_empty() {
        for i in 0..def.retex_s.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}s"), def.retex_s[i].to_string()));
            p.push((41, format!("retex{n}d"), def.retex_d[i].to_string()));
        }
    }
    // ops 60-69 — head models (each slot tagged with its actual opcode)
    for (slot, &model_id) in def.heads.iter().enumerate() {
        if model_id != -1 {
            p.push((
                (60 + slot) as u8,
                format!("head{}", slot + 1),
                model_name(registry, model_id as u16),
            ));
        }
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
