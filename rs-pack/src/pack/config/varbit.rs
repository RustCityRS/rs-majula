use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_varbits(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.varbit;

    let configs = parse_config_sections_cached(file_cache, "varbit", constants);

    let mut server = PackedData::new(pack.max);
    #[cfg(before_647)]
    let client_count = pack.max;
    #[cfg(since_647)]
    let client_count = file_cache
        .client_count("varbit", pack, constants)
        .unwrap_or(pack.max);
    let mut client = PackedData::new(client_count);

    for id in 0..pack.max {
        server.start_entry();
        if id < client_count {
            client.start_entry();
        }

        let Some((debugname, props)) = config_entry("varbit", pack, &configs, id) else {
            server.skip_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        let mut basevar = None;
        let mut startbit = None;
        let mut endbit = None;

        for (key, value) in props {
            match key.as_str() {
                // 1
                "basevar" => parse_ref(&registry.varp, "varp", value, |v| {
                    basevar = Some(v);
                }),
                "startbit" => parse_number(value, |v| startbit = Some(v)),
                "endbit" => parse_number(value, |v| endbit = Some(v)),

                // not found
                _ => panic!("Unrecognized varbit config key: {key}"),
            }
        }

        if let Some(basevar) = basevar
            && let Some(startbit) = startbit
            && let Some(endbit) = endbit
        {
            if id < client_count {
                client.p1(1);
                client.p2(basevar);
                client.p1(startbit);
                client.p1(endbit);
            }
            server.p1(1);
            server.p2(basevar);
            server.p1(startbit);
            server.p1(endbit);
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        if id < client_count {
            client.finish_entry();
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;
    use crate::pack::pack::{load_constants, parse_config_sections};
    use std::fs;
    use std::path::Path;

    // A varbit past the cache's (289's horrorquest, on the server-only varp
    // deephorror) is packed for the server and kept out of the client's
    // config archive, whose varbits stay the cache's own count.
    #[test]
    fn server_only_varbits_stay_out_of_the_client() {
        // Unit tests run with cwd = rs-pack/, one level below the repo-root-relative dirs.
        let root = Path::new("..");
        let source = root.join(crate::CONTENT_DIR).join("scripts");
        let registry = PackRegistry::load(&root.join(crate::PACK_DIR)).expect("load registry");
        let constants = load_constants(&source);
        let server_dir = root.join("target").join("varbit-pack-test");
        fs::create_dir_all(&server_dir).unwrap();
        let result = pack_varbits(&FileCache::new(&source), &registry, &constants).unwrap();
        result.server.save(&server_dir, "varbit").unwrap();
        let client = result.client.unwrap();

        let horror = registry
            .varbit
            .get_by_debugname("horrorquest")
            .expect("horrorquest");
        let client: Vec<_> = PackedData::entries(&client.dat, &client.idx)
            .unwrap()
            .filter(|(_, data)| !data.is_empty())
            .map(|(id, _)| id as u32)
            .collect();
        let cache_varbits =
            parse_config_sections(&[source.join("_unpack").join("all.varbit")], &constants)
                .keys()
                .filter_map(|name| registry.varbit.get_by_debugname(name))
                .max()
                .expect("run `cargo unpack` first");
        assert_eq!(
            client.last().copied(),
            Some(cache_varbits),
            "the client's last varbit is the cache's"
        );
        assert!(horror > cache_varbits, "horrorquest is past the cache's");
        assert!(!client.contains(&horror), "and not the client's");

        let server = fs::read(server_dir.join("varbit.dat")).unwrap();
        assert_eq!(
            u16::from_be_bytes([server[0], server[1]]) as u32,
            registry.varbit.max,
            "the server's holds every varbit"
        );
    }
}
