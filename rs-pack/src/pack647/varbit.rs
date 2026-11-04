//! VarBit config packer — all.varbit text → varbit.dat/.idx + varbit.client.dat/.idx.
//!
//! Binary format: opcode 1 = Bits(varp_id: p2, lsb: p1, msb: p1), terminator p1(0).

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::cache647::VarBitType;
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

pub fn pack_varbits(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "varbit");
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} varbit configs", configs.len());

    // The client's varbits are the cache's (all.varbit), as pack_varps keeps
    // the client's varps: a varbit past them, on a server-only varp (289's
    // Horror from the Deep packs its state in one), is the server's alone.
    let all_file = source_dir.join("_unpack").join("all.varbit");
    let client_count = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
            .keys()
            .filter_map(|name| registry.varbit.get_by_name(name))
            .max()
            .map(|m| m + 1)
            .unwrap_or(registry.varbit.max)
    } else {
        registry.varbit.max
    };

    let mut server = PackedData::new(registry.varbit.max);
    let mut client = PackedData::new(client_count);

    for id in 0..registry.varbit.max {
        let in_client = id < client_count;
        let Some(name) = registry.varbit.get_by_id(id) else {
            server.skip_entry();
            if in_client {
                client.skip_entry();
            }
            continue;
        };

        let Some(props) = configs.get(name) else {
            server.skip_entry();
            if in_client {
                client.skip_entry();
            }
            continue;
        };

        server.start_entry();
        if in_client {
            client.start_entry();
        }

        let mut basevar: Option<u16> = None;
        let mut startbit: Option<u8> = None;
        let mut endbit: Option<u8> = None;

        for (key, value) in props {
            match key.as_str() {
                "basevar" => {
                    basevar = Some(
                        registry
                            .varp
                            .get_by_name(value)
                            .map(|id| id as u16)
                            .unwrap_or_else(|| value.parse::<u16>().unwrap_or(0)),
                    );
                }
                "startbit" => startbit = value.parse().ok(),
                "endbit" => endbit = value.parse().ok(),
                _ => {}
            }
        }

        if let (Some(varp_id), Some(lsb), Some(msb)) = (basevar, startbit, endbit) {
            // opcode 1: Bits
            server.p1(1);
            server.p2(varp_id);
            server.p1(lsb);
            server.p1(msb);

            if in_client {
                client.p1(1);
                client.p2(varp_id);
                client.p1(lsb);
                client.p1(msb);
            }
        }

        server.finish_entry();
        if in_client {
            client.finish_entry();
        }
    }

    server.save(server_dir, "varbit")?;
    config_blob.add_packed_data("varbit", client.dat, client.idx);
    Ok(())
}

pub fn emit_varbit_props(def: &VarBitType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if def.varp_id != 0 || def.lsb != 0 || def.msb != 0 {
        let varp_name = registry
            .varp
            .get_by_id(def.varp_id as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("varp_{}", def.varp_id));
        props.push(("basevar".into(), varp_name));
        props.push(("startbit".into(), def.lsb.to_string()));
        props.push(("endbit".into(), def.msb.to_string()));
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_blob::ConfigBlob;

    // A varbit past the cache's (289's horrorquest, on the server-only varp
    // deephorror) is packed for the server and kept out of the client's
    // config archive, whose varbits stay the cache's own count.
    #[test]
    fn server_only_varbits_stay_out_of_the_client() {
        // Unit tests run with cwd = rs-pack/, one level below the repo-root-relative dirs.
        let root = Path::new("..");
        let source = root.join(crate::CONTENT_DIR).join("scripts");
        let registry = PackRegistry::load(&root.join(crate::PACK_DIR)).expect("load registry");
        let constants = super::super::load_constants(&source);
        let server_dir = root.join("target").join("varbit-pack-test");
        std::fs::create_dir_all(&server_dir).unwrap();
        let mut blob = ConfigBlob::new();
        pack_varbits(&source, &registry, &constants, &server_dir, &mut blob).unwrap();

        let horror = registry
            .varbit
            .get_by_name("horrorquest")
            .expect("horrorquest");
        let client = blob.read_packed_data("varbit").unwrap();
        let cache_varbits =
            parse_config_sections(&[source.join("_unpack").join("all.varbit")], &constants)
                .keys()
                .filter_map(|name| registry.varbit.get_by_name(name))
                .max()
                .expect("run `cargo unpack` first");
        assert_eq!(
            client.keys().next_back().copied(),
            Some(cache_varbits),
            "the client's last varbit is the cache's"
        );
        assert!(horror > cache_varbits, "horrorquest is past the cache's");
        assert!(!client.contains_key(&horror), "and not the client's");

        let server = std::fs::read(server_dir.join("varbit.dat")).unwrap();
        assert_eq!(
            u16::from_be_bytes([server[0], server[1]]) as u32,
            registry.varbit.max,
            "the server's holds every varbit"
        );
    }
}
