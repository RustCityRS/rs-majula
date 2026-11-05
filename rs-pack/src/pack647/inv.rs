use std::collections::HashMap;
use std::path::Path;

use crate::cache::inv::InvTypeRaw;
use crate::types::InvScope;
use anyhow::{Context, Result, ensure};
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

pub fn emit_inv_props(def: &InvTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if def.has_scope {
        let s = match def.scope {
            InvScope::Temp => "temp".to_string(),
            InvScope::Perm => "perm".to_string(),
            InvScope::Shared => "shared".to_string(),
        };
        props.push(("scope".into(), s));
    }
    if def.size != 0 {
        props.push(("size".into(), def.size.to_string()));
    }
    if def.stackall {
        props.push(("stackall".into(), "yes".into()));
    }
    let stockobj = def.stockobj.as_deref().unwrap_or_default();
    let stockcount = def.stockcount.as_deref().unwrap_or_default();
    let stockrate = def.stockrate.as_deref().unwrap_or_default();
    if !stockobj.is_empty() {
        for i in 0..stockobj.len() {
            let obj_name = if stockobj[i] == u16::MAX {
                "null".to_string()
            } else {
                registry
                    .obj
                    .get_by_id(stockobj[i] as u32)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("obj_{}", stockobj[i]))
            };
            let value = if stockrate[i] != 0 {
                format!("{},{},{}", obj_name, stockcount[i], stockrate[i])
            } else {
                format!("{},{}", obj_name, stockcount[i])
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
                "stackall" if value == "yes" => packed.p1(3),
                "restock" if value == "yes" => packed.p1(5),
                "allstock" if value == "yes" => packed.p1(6),
                "protect" if value == "no" => packed.p1(7),
                "runweight" if value == "yes" => packed.p1(8),
                "dummyinv" if value == "yes" => packed.p1(9),
                _ => {}
            }
        }

        let stocks = props
            .iter()
            .filter(|(key, _)| key.starts_with("stock"))
            .map(|(_, value)| value.split(',').map(str::trim).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        if !stocks.is_empty() {
            let count = u8::try_from(stocks.len())
                .with_context(|| format!("{name}: too many stock entries"))?;
            packed.p1(4);
            packed.p1(count);
            for stock in stocks {
                ensure!(
                    (2..=3).contains(&stock.len()),
                    "{name}: invalid stock entry"
                );
                let obj =
                    if stock[0] == "null" {
                        u16::MAX
                    } else {
                        registry.obj.get_by_name(stock[0]).with_context(|| {
                            format!("{name}: unknown stock object '{}'", stock[0])
                        })? as u16
                    };
                let count = stock[1].parse::<u16>()?;
                let rate = stock
                    .get(2)
                    .map(|rate| rate.parse::<i32>())
                    .transpose()?
                    .unwrap_or(0);
                packed.p2(obj);
                packed.p2(count);
                packed.p4(rate);
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
