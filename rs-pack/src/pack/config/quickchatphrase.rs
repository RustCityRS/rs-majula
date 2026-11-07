use super::client_config_table;
use crate::archive::QUICKCHAT_GLOBAL_FIRST;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::util::{parse_bool, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

/// The id and the parameters' tables of a command the text can name.
fn command(name: &str) -> Option<(u16, &'static [&'static str])> {
    Some(match name {
        "listdialog" => (0, &["enum"]),
        "objdialog" => (1, &[]),
        "countdialog" => (2, &[]),
        "stat_base" => (4, &["stat"]),
        "enum_string" => (6, &["enum", "varp"]),
        "enum_string_clan" => (7, &["enum"]),
        "tostring_varp" => (8, &["varp"]),
        "tostring_varbit" => (9, &["varbit"]),
        "objtradedialog" => (10, &[]),
        "enum_string_statbase" => (11, &["enum", "stat"]),
        "acc_getcount_world" => (12, &[]),
        "acc_getmeancombatlevel" => (13, &[]),
        "tostring_shared" => (14, &["vars"]),
        "activecombatlevel" => (15, &[]),
        _ => return None,
    })
}

fn parameter(registry: &PackRegistry, kind: &str, value: &str) -> u16 {
    let pack = match kind {
        "enum" => &registry.r#enum,
        "stat" => &registry.stat,
        "varp" => &registry.varp,
        "varbit" => &registry.varbit,
        "vars" => &registry.vars,
        _ => unreachable!(),
    };
    let mut id = 0;
    parse_ref(pack, kind, value.trim(), |v| id = v);
    id
}

/// The text with each `<command(params)>` taken back down to a `<`, and the
/// commands it named.
fn parse_phrase(registry: &PackRegistry, phrase: &str) -> (String, Vec<(u16, Vec<u16>)>) {
    let mut text = String::new();
    let mut commands = Vec::new();
    let mut rest = phrase;
    while let Some(start) = rest.find('<') {
        text.push_str(&rest[..start]);
        text.push('<');
        rest = &rest[start + 1..];
        let Some(end) = rest.find('>') else {
            continue;
        };
        let Some((name, params)) = rest[..end]
            .strip_suffix(')')
            .and_then(|call| call.split_once('('))
        else {
            continue;
        };
        let Some((id, kinds)) = command(name) else {
            continue;
        };
        let params = if kinds.is_empty() {
            Vec::new()
        } else {
            let values: Vec<_> = params.split(',').collect();
            assert_eq!(values.len(), kinds.len(), "Invalid quickchat command");
            kinds
                .iter()
                .zip(values)
                .map(|(kind, value)| parameter(registry, kind, value))
                .collect()
        };
        commands.push((id, params));
        rest = &rest[end + 1..];
    }
    text.push_str(rest);
    (text, commands)
}

pub fn pack_quickchatphrases(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "quickchatphrase",
        &registry.chatphrase,
        constants,
        |client, name, props| {
            let id = registry.chatphrase.get_by_debugname(name).unwrap_or(0) as u16;
            let first = id & QUICKCHAT_GLOBAL_FIRST;
            let mut commands = Vec::new();
            for group in props.chunk_by(|a, b| a.0 == b.0) {
                match group[0].0.as_str() {
                    // 1, and 3 after the responses
                    "phrase" => {
                        for (_, value) in group {
                            let (text, named) = parse_phrase(registry, value);
                            commands.extend(named);
                            client.p1(1);
                            client.pjstr(&text);
                        }
                    }

                    // 2
                    "autoresponse" => {
                        client.p1(2);
                        client.p1(u8::try_from(group.len()).expect("Too many quickchat responses"));
                        for (_, value) in group {
                            parse_ref(&registry.chatphrase, "chatphrase", value, |v| {
                                client.p2(v - first)
                            });
                        }
                    }

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

                    key => panic!("Unrecognized quickchatphrase config key: {key}"),
                }
            }
            if !commands.is_empty() {
                client.p1(3);
                client.p1(u8::try_from(commands.len()).expect("Too many quickchat commands"));
                for (id, params) in commands {
                    client.p2(id);
                    for param in params {
                        client.p2(param);
                    }
                }
            }
        },
    )
}
