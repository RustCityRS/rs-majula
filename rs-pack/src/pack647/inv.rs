//! Inv (inventory) packers — two flavours sharing one 2004scape-style text
//! format:
//!
//! 1. **Server cache (legacy, untouched binary)**: [`pack_invs`] reads .inv
//!    text and writes `data/pack/server/inv.dat`/`inv.idx` via `PackedData`.
//!    This is the SERVER cache the engine reads at startup; binary output is
//!    unchanged from prior revisions.
//! 2. **Client cache (Phase 1 rebuild)**: [`emit_inv_props`] +
//!    [`pack_inv_def`] roundtrip through `InvDefinition` for archive 2
//!    group 5 of the JS5 client cache. `pack_inv_def(...).encode()` produces
//!    byte-perfect cache bytes.
//!
//! Both paths read the SAME `.inv` text format, mirroring 2004scape's
//! `engine/tools/pack/config/InvConfig.ts`: `scope=temp|perm|shared`,
//! `size=N`, `stackall|restock|allstock|protect|runweight|dummyinv=yes|no`,
//! `stockN=obj_name,count[,rate]`. The debugname (op 250) is the
//! `[section]` header in pack_inv_def's case; for pack_invs the debugname
//! is the registry-mapped name.
//!
//! Object references resolve through `PackRegistry::obj` — human-readable
//! names in, numeric ids out.

use std::collections::HashMap;
use std::path::Path;

use crate::cache647::InvType;
use anyhow::Result;
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

/// Walk an inv's emissions and produce the (key, value) pairs matching
/// 2004scape's InvConfig text format. The `DebugName` emission is dropped
/// because the `[section]` header already carries it; the writer lifts the
/// section name from `def.debugname`.
pub fn emit_inv_props(def: &InvType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if def.has_scope {
        let s = match def.scope {
            crate::types647::InvScope::Temp => "temp".to_string(),
            crate::types647::InvScope::Perm => "perm".to_string(),
            crate::types647::InvScope::Shared => "shared".to_string(),
        };
        props.push(("scope".into(), s));
    }
    if def.size != 0 {
        props.push(("size".into(), def.size.to_string()));
    }
    if def.stackall {
        props.push(("stackall".into(), "yes".into()));
    }
    if !def.stock_obj.is_empty() {
        for i in 0..def.stock_obj.len() {
            let obj_name = if def.stock_obj[i] == u16::MAX {
                "null".to_string()
            } else {
                registry
                    .obj
                    .get_by_id(def.stock_obj[i] as u32)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("obj_{}", def.stock_obj[i]))
            };
            let value = if def.stock_rate[i] != 0 {
                format!("{},{},{}", obj_name, def.stock_count[i], def.stock_rate[i])
            } else {
                format!("{},{}", obj_name, def.stock_count[i])
            };
            props.push((format!("stock{}", i + 1), value));
        }
    }
    if def.restock {
        props.push(("restock".into(), "yes".into()));
    }
    if def.allstock {
        props.push(("allstock".into(), "yes".into()));
    }
    if !def.protect {
        props.push(("protect".into(), "no".into()));
    }
    if def.runweight {
        props.push(("runweight".into(), "yes".into()));
    }
    if def.dummyinv {
        props.push(("dummyinv".into(), "yes".into()));
    }
    props
}

// ─── Legacy server-cache packer (data/pack/server/inv.dat) ───────────────
//
// Same .inv text format as the cache packer above; binary output is the
// rs-server's startup-loadable PackedData blob. Server cache is a separate
// store from the JS5 client cache (data/pack/server/) and is never
// transmitted to the client.
pub fn pack_invs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "inv");
    info!("  Found {} .inv files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} inv configs", configs.len());

    // The client stream stops at the cache's last inv: content may add
    // server-only invs past it, and those are not in the client's archive.
    let all_file = source_dir.join("_unpack").join("all.inv");
    let client_count = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
            .keys()
            .filter_map(|name| registry.inv.get_by_name(name))
            .max()
            .map_or(registry.inv.max, |m| m + 1)
    } else {
        registry.inv.max
    };

    let mut packed = PackedData::new(registry.inv.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.inv.max {
        let to_client = id < client_count;
        packed.start_entry();
        if to_client {
            client.start_entry();
        }
        let Some(name) = registry.inv.get_by_id(id) else {
            packed.finish_entry();
            if to_client {
                client.finish_entry();
            }
            continue;
        };
        let Some(props) = configs.get(name) else {
            packed.finish_entry();
            if to_client {
                client.finish_entry();
            }
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                "scope" => {
                    let v: u8 = match value.as_str() {
                        "temp" => 0,
                        "perm" => 1,
                        "shared" => 2,
                        s => s.parse().unwrap_or(0),
                    };
                    packed.p1(1);
                    packed.p1(v);
                }
                "size" => {
                    if let Ok(v) = value.parse::<u16>() {
                        packed.p1(2);
                        packed.p2(v);
                        if to_client {
                            client.p1(2);
                            client.p2(v);
                        }
                    }
                }
                "stackall" => {
                    if value == "yes" {
                        packed.p1(3);
                    }
                }
                "restock" => {
                    if value == "yes" {
                        packed.p1(4);
                    }
                }
                "allstock" => {
                    if value == "yes" {
                        packed.p1(5);
                    }
                }
                "protect" => {
                    if value == "no" {
                        packed.p1(8);
                    }
                }
                "runweight" => {
                    if value == "yes" {
                        packed.p1(9);
                    }
                }
                "dummyinv" => {
                    if value == "yes" {
                        packed.p1(10);
                    }
                }
                _ if key.starts_with("stock") && key != "stackall" => {
                    // Each stockN=obj_name,count[,rate] becomes one opcode-6
                    // entry — server cache shape, NOT the client cache's
                    // batched opcode-4. Both formats coexist; pack_inv_def
                    // handles the client cache.
                }
                _ => {}
            }
        }

        // Stock items (server-cache opcode 6): stockN=obj_name,count[,rate]
        let stocks: Vec<(&str, &str)> = props
            .iter()
            .filter(|(k, _)| k.starts_with("stock") && k.as_str() != "stackall")
            .filter_map(|(_, v)| v.split_once(','))
            .collect();
        if !stocks.is_empty() {
            for (obj_name, rest) in &stocks {
                let parts: Vec<&str> = rest.split(',').collect();
                let Ok(count) = parts[0].trim().parse::<u16>() else {
                    continue;
                };
                let Some(obj_id) = registry.obj.get_by_name(obj_name.trim()) else {
                    continue;
                };
                let rate: u32 = parts
                    .get(1)
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);
                packed.p1(6);
                packed.p2(obj_id as u16);
                packed.p2(count);
                packed.p4(rate as i32);
            }
        }

        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
        if to_client {
            client.finish_entry();
        }
    }

    packed.save(output_dir, "inv")?;
    config_blob.add_packed_data("inv", client.dat, client.idx);
    Ok(())
}
