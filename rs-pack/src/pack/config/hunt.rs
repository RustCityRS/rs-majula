use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::*;
use crate::types::{
    HuntCheckAfk, HuntCheckNotBusy, HuntCheckNotTooStrong, HuntCheckVis, HuntFindKeepHunting,
    HuntModeType, HuntNobodyNear, NpcMode,
};

pub fn pack_hunts(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.hunt;

    let files = file_cache.collect("hunt");
    debug!("  Found {} .hunt files", files.len());

    let configs = parse_config_sections_cached(file_cache, "hunt", constants);
    debug!("  Parsed {} hunt configs", configs.len());

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown npc id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown hunt config: {debugname}");
        };

        let has_key = |k: &str| props.iter().any(|(pk, _)| pk == k);
        let has_type = |t: &str| props.iter().any(|(pk, pv)| pk == "type" && pv == t);

        let mut extracheck_var = 0;

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => {
                    let val = HuntModeType::from_config_str(value);
                    if val != HuntModeType::Off {
                        server.p1(1);
                        server.p1(val as u8);
                    }
                }

                // 2
                "check_vis" => {
                    let val = HuntCheckVis::from_config_str(value);
                    if val != HuntCheckVis::Off {
                        server.p1(2);
                        server.p1(val as u8);
                    }
                }

                // 3
                "check_nottoostrong" => {
                    let val = HuntCheckNotTooStrong::from_config_str(value);
                    if val != HuntCheckNotTooStrong::Off {
                        server.p1(3);
                        server.p1(val as u8);
                    }
                }

                // 4
                "check_notbusy" => {
                    if HuntCheckNotBusy::from_config_str(value) != HuntCheckNotBusy::Off {
                        server.p1(4);
                    }
                }

                // 5
                "find_keephunting" => {
                    if HuntFindKeepHunting::from_config_str(value) != HuntFindKeepHunting::Off {
                        server.p1(5);
                    }
                }

                // 6
                "find_newmode" => {
                    let v = NpcMode::from_config_str(value) as u8;
                    if v != 0 {
                        server.p1(6);
                        server.p1(v);
                    }
                }

                // 7
                "nobodynear" => {
                    let val = HuntNobodyNear::from_config_str(value);
                    if val != HuntNobodyNear::PauseHunt {
                        server.p1(7);
                        server.p1(val as u8);
                    }
                }

                // 8
                "check_notcombat" => parse_varp(registry, value.strip_prefix("%").unwrap(), |v| {
                    server.p1(8);
                    server.p2(v);
                }),

                // 9
                "check_notcombat_self" => {
                    parse_varn(registry, value.strip_prefix("%").unwrap(), |v| {
                        server.p1(9);
                        server.p2(v);
                    })
                }

                // 10
                "check_afk" => {
                    if HuntCheckAfk::from_config_str(value) != HuntCheckAfk::Off {
                        server.p1(10);
                    }
                }

                // 11
                "rate" => parse_number(value, |v| {
                    if v != 1 {
                        server.p1(11);
                        server.p2(v);
                    }
                }),

                // 12
                "check_category" => {
                    if has_key("check_npc")
                        || has_key("check_obj")
                        || has_key("check_loc")
                        || has_key("check_inv")
                        || has_key("check_invparam")
                    {
                        panic!("Invalid check_category value: {key}={value}");
                    }
                    if !has_type("npc") && !has_type("obj") && !has_type("scenery") {
                        panic!("Invalid check_category value: {key}={value}");
                    }
                    parse_category(registry, value, |v| {
                        server.p1(12);
                        server.p2(v);
                    });
                }

                // 13
                "check_npc" => {
                    if has_key("check_category")
                        || has_key("check_obj")
                        || has_key("check_loc")
                        || has_key("check_inv")
                        || has_key("check_invparam")
                    {
                        panic!("Invalid check_npc value: {key}={value}");
                    }
                    if !has_type("npc") {
                        panic!("Invalid check_npc value: {key}={value}");
                    }
                    parse_npc(registry, value, |v| {
                        server.p1(13);
                        server.p2(v);
                    });
                }

                // 14
                "check_obj" => {
                    if has_key("check_category")
                        || has_key("check_npc")
                        || has_key("check_loc")
                        || has_key("check_inv")
                        || has_key("check_invparam")
                    {
                        panic!("Invalid check_obj value: {key}={value}");
                    }
                    if !has_type("obj") {
                        panic!("Invalid check_obj value: {key}={value}");
                    }
                    parse_obj(registry, value, |v| {
                        server.p1(14);
                        server.p2(v);
                    });
                }

                // 15
                "check_loc" => {
                    if has_key("check_category")
                        || has_key("check_npc")
                        || has_key("check_obj")
                        || has_key("check_inv")
                        || has_key("check_invparam")
                    {
                        panic!("Invalid check_loc value: {key}={value}");
                    }
                    if !has_type("scenery") {
                        panic!("Invalid check_loc value: {key}={value}");
                    }
                    parse_loc(registry, value, |v| {
                        server.p1(15);
                        server.p2(v);
                    });
                }

                // 16
                "check_inv" => {
                    if has_key("check_category")
                        || has_key("check_npc")
                        || has_key("check_obj")
                        || has_key("check_loc")
                        || has_key("check_invparam")
                    {
                        panic!("Invalid check_inv value: {key}={value}");
                    }
                    if !has_type("player") {
                        panic!("Invalid check_inv value: {key}={value}");
                    }
                    let parts: Vec<&str> = value.splitn(3, ',').collect();
                    if parts.len() != 3 {
                        panic!("Invalid check_inv value: {key}={value}");
                    }
                    let last = parts[2];
                    let Some(condition) = last.chars().next() else {
                        panic!("Invalid check_inv value: {key}={value}");
                    };
                    if !matches!(condition, '=' | '>' | '<' | '!' | '&' | '|') {
                        panic!("Invalid check_inv value: {key}={value}");
                    }
                    server.p1(16);
                    parse_inv(registry, parts[0], |v| server.p2(v));
                    parse_obj(registry, parts[1], |v| server.p2(v));
                    server.pjstr(&condition.to_string());
                    parse_number(&last[1..], |v| server.p4(v));
                }

                // 17
                "check_invparam" => {
                    if has_key("check_category")
                        || has_key("check_npc")
                        || has_key("check_obj")
                        || has_key("check_loc")
                        || has_key("check_inv")
                    {
                        panic!("Invalid check_invparam value: {key}={value}");
                    }
                    if !has_type("player") {
                        panic!("Invalid check_invparam value: {key}={value}");
                    }
                    let parts: Vec<&str> = value.splitn(3, ',').collect();
                    if parts.len() != 3 {
                        panic!("Invalid check_invparam value: {key}={value}");
                    }
                    let last = parts[2];
                    let Some(condition) = last.chars().next() else {
                        panic!("Invalid check_invparam value: {key}={value}");
                    };
                    if !matches!(condition, '=' | '>' | '<' | '!' | '&' | '|') {
                        panic!("Invalid check_invparam value: {key}={value}");
                    }
                    server.p1(17);
                    parse_inv(registry, parts[0], |v| server.p2(v));
                    parse_param(registry, parts[1], |v| server.p2(v));
                    server.pjstr(&condition.to_string());
                    parse_number(&last[1..], |v| server.p4(v));
                }

                // 18-20
                "extracheck_var" => {
                    if extracheck_var > 2 {
                        panic!("Invalid extracheck_var value: {key}={value}");
                    }
                    let parts: Vec<&str> = value.splitn(2, ',').collect();
                    if parts.len() != 2 {
                        panic!("Invalid extracheck_var value: {key}={value}");
                    }
                    if !parts[0].starts_with('%') {
                        panic!("Invalid extracheck_var value: {key}={value}");
                    }
                    let last = parts[1];
                    let Some(condition) = last.chars().next() else {
                        panic!("Invalid extracheck_var value: {key}={value}");
                    };
                    if !matches!(condition, '=' | '>' | '<' | '!' | '&' | '|') {
                        panic!("Invalid extracheck_var value: {key}={value}");
                    }
                    server.p1(18 + extracheck_var);
                    parse_varp(registry, &parts[0][1..], |v| server.p2(v));
                    server.pjstr(&condition.to_string());
                    parse_number(&last[1..], |v| server.p4(v));
                    extracheck_var += 1;
                }

                // not found
                _ => panic!("Unrecognized hunt config key: {key}"),
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
