use super::client_config_table;
use crate::archive::QUICKCHAT_GLOBAL_FIRST;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackFile, PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_bool, parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_quickchatcats(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "quickchatcat",
        &registry.chatcat,
        constants,
        |client, name, props| {
            let id = registry.chatcat.get_by_debugname(name).unwrap_or(0) as u16;
            let first = id & QUICKCHAT_GLOBAL_FIRST;
            for group in props.chunk_by(|a, b| a.0 == b.0) {
                match group[0].0.as_str() {
                    // 1
                    "desc" => {
                        for (_, value) in group {
                            client.p1(1);
                            client.pjstr(value);
                        }
                    }

                    // 2
                    "subcat" => {
                        pack_targets(client, 2, group, ("chatcat", &registry.chatcat), first)
                    }

                    // 3
                    "phrase" => pack_targets(
                        client,
                        3,
                        group,
                        ("chatphrase", &registry.chatphrase),
                        first,
                    ),

                    // 4
                    "unsearchable" => {
                        for (_, value) in group {
                            parse_bool(value, |v| {
                                if v {
                                    client.p1(4);
                                }
                            });
                        }
                    }

                    key => panic!("Unrecognized quickchatcat config key: {key}"),
                }
            }
        },
    )
}

fn pack_targets(
    client: &mut PackedData,
    code: u8,
    lines: &[(String, String)],
    (kind, pack): (&str, &PackFile),
    first: u16,
) {
    client.p1(code);
    client.p1(u8::try_from(lines.len()).expect("Too many quickchat targets"));
    for (_, value) in lines {
        let (target, shortcut) = value
            .rsplit_once(',')
            .unwrap_or_else(|| panic!("Invalid quickchat target: {value}"));
        parse_ref(pack, kind, target.trim(), |v| client.p2(v - first));
        parse_number(shortcut.trim(), |v: u8| client.p1(v));
    }
}
