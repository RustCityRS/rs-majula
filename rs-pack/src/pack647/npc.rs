//! NPC config packer — .npc text → separate client + server .dat/.idx files.
//! Matches 2004scape's dual-output architecture: `packNpcConfigs()` returns
//! `{ client, server }` with client opcodes (models, name, anims) going to
//! `npc.client.dat/.idx` and server opcodes (stats, patrol, AI) going to
//! `npc.dat/.idx`.

use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

fn parse_coord(s: &str) -> Option<i32> {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() != 5 {
        return None;
    }
    let level: u32 = parts[0].parse().ok()?;
    let mx: u32 = parts[1].parse().ok()?;
    let mz: u32 = parts[2].parse().ok()?;
    let lx: u32 = parts[3].parse().ok()?;
    let lz: u32 = parts[4].parse().ok()?;
    let x = mx * 64 + lx;
    let z = mz * 64 + lz;
    Some(((level & 0x3) << 28 | (x & 0x3FFF) << 14 | (z & 0x3FFF)) as i32)
}

fn resolve_model_id(value: &str, registry: &PackRegistry) -> u16 {
    if let Some(id) = registry.model.get_by_name(value) {
        return id as u16;
    }
    if let Some(rest) = value.strip_prefix("model_") {
        if let Some(num_str) = rest.split('_').next() {
            if let Ok(id) = num_str.parse::<u16>() {
                return id;
            }
        }
    }
    value.parse::<u16>().unwrap_or(0)
}

fn resolve_seq_id(value: &str, registry: &PackRegistry) -> u16 {
    if let Some(id) = registry.seq.get_by_name(value) {
        return id as u16;
    }
    if let Some(rest) = value.strip_prefix("seq_") {
        if let Ok(id) = rest.parse::<u16>() {
            return id;
        }
    }
    value.parse::<u16>().unwrap_or(0)
}

pub fn pack_npcs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &std::collections::HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let all_files = collect_config_files(source_dir, "npc");
    let (specific, unpack): (Vec<_>, Vec<_>) = all_files
        .into_iter()
        .partition(|p| !p.components().any(|c| c.as_os_str() == "_unpack"));
    let mut files = specific;
    files.extend(unpack);
    info!("  Found {} .npc files", files.len());

    let configs = parse_config_sections(&files, constants);
    let param_types = super::param::param_types(source_dir, registry, constants);
    info!("  Parsed {} npc configs", configs.len());

    let mut server = PackedData::new(registry.npc.max);
    let mut client = PackedData::new(registry.npc.max);

    for id in 0..registry.npc.max {
        let Some(name) = registry.npc.get_by_id(id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        let props = configs.get(name);

        if props.is_none() {
            server.skip_entry();
            client.skip_entry();
            continue;
        }

        let has_client_fields = props.is_some_and(|p| {
            p.iter().any(|(k, _)| {
                !matches!(
                    k.as_str(),
                    "category"
                        | "desc"
                        | "attack"
                        | "defence"
                        | "strength"
                        | "hitpoints"
                        | "ranged"
                        | "magic"
                        | "wanderrange"
                        | "maxrange"
                        | "huntrange"
                        | "timer"
                        | "respawnrate"
                        | "moverestrict"
                        | "attackrange"
                        | "blockwalk"
                        | "defaultmode"
                        | "regenrate"
                        | "huntmode"
                        | "members"
                        | "givechase"
                        | "param"
                ) && !k.starts_with("patrol")
            })
        });

        server.start_entry();
        if has_client_fields {
            client.start_entry();
        } else {
            client.skip_entry();
        }

        // Collect server-only multi-value fields (client uses emission-order pass below)
        let mut patrols: Vec<(i32, u8)> = Vec::new();
        if let Some(props) = props {
            for (key, value) in props {
                if key.starts_with("patrol") {
                    if let Some((coord_str, delay_str)) = value.split_once(',') {
                        if let (Some(coord), Ok(delay)) =
                            (parse_coord(coord_str), delay_str.parse::<u8>())
                        {
                            patrols.push((coord, delay));
                        }
                    }
                }
            }
        } // if let Some(props) = props

        // =====================================================================
        // CLIENT opcodes → client PackedData (emission-order)
        //
        // Walks props in text-file order (which mirrors original cache emission
        // order). Multi-key families (model, head, recol, retex, colour,
        // translate, campaign, multi/transform, sound, param) are buffered and
        // flushed as a single opcode when the family changes or the props end.
        // =====================================================================

        // Batch family classifiers
        fn is_model_key(k: &str) -> bool {
            k.starts_with("model") && k[5..].chars().all(|c| c.is_ascii_digit()) && k.len() > 5
        }
        fn is_head_key(k: &str) -> bool {
            k.starts_with("head") && k[4..].chars().all(|c| c.is_ascii_digit()) && k.len() > 4
        }
        fn is_recol_key(k: &str) -> bool {
            k.starts_with("recol") && (k.ends_with('s') || k.ends_with('d'))
        }
        fn is_retex_key(k: &str) -> bool {
            k.starts_with("retex") && (k.ends_with('s') || k.ends_with('d'))
        }
        fn is_colour_key(k: &str) -> bool {
            k.starts_with("colour") && k[6..].chars().all(|c| c.is_ascii_digit()) && k.len() > 6
        }
        fn is_translate_key(k: &str) -> bool {
            k.starts_with("translate") && k[9..].chars().all(|c| c.is_ascii_digit()) && k.len() > 9
        }
        fn is_campaign_key(k: &str) -> bool {
            k.starts_with("campaign") && k[8..].chars().all(|c| c.is_ascii_digit()) && k.len() > 8
        }
        fn is_multi_key(k: &str) -> bool {
            k == "multivar"
                || k == "multivarp"
                || k == "multidefault"
                || (k.starts_with("multi")
                    && k[5..].chars().all(|c| c.is_ascii_digit())
                    && k.len() > 5)
        }
        fn is_sound_key(k: &str) -> bool {
            matches!(
                k,
                "idlesound" | "crawlsound" | "walksound" | "runsound" | "sounddistance"
            )
        }
        fn is_param_key(k: &str) -> bool {
            k == "paramint" || k == "paramstr"
        }

        // Batch buffers
        let mut models_buf: Vec<u16> = Vec::new();
        let mut heads_buf: Vec<u16> = Vec::new();
        let mut recol_s_buf: Vec<u16> = Vec::new();
        let mut recol_d_buf: Vec<u16> = Vec::new();
        let mut retex_s_buf: Vec<u16> = Vec::new();
        let mut retex_d_buf: Vec<u16> = Vec::new();
        let mut colours_buf: Vec<u8> = Vec::new();
        let mut translate_buf: Vec<(u8, i8, i8, i8)> = Vec::new();
        let mut campaign_buf: Vec<i16> = Vec::new();
        let mut multi_var_buf: i32 = -1;
        let mut multi_varp_buf: i32 = -1;
        let mut multi_default_buf: Option<i32> = None;
        let mut multi_transforms_buf: Vec<i32> = Vec::new();
        let mut multi_active = false;
        let mut sound_idle: i32 = -1;
        let mut sound_crawl: i32 = -1;
        let mut sound_walk: i32 = -1;
        let mut sound_run: i32 = -1;
        let mut sound_dist: u8 = 0;
        let mut sound_active = false;
        let mut params_buf: Vec<(u32, bool, String)> = Vec::new(); // (key, is_string, value)

        // Flush helpers
        fn flush_models(client: &mut PackedData, buf: &mut Vec<u16>) {
            if buf.is_empty() {
                return;
            }
            client.p1(1);
            client.p1(buf.len() as u8);
            for &v in buf.iter() {
                client.p2(v);
            }
            buf.clear();
        }
        fn flush_heads(client: &mut PackedData, buf: &mut Vec<u16>) {
            if buf.is_empty() {
                return;
            }
            client.p1(60);
            client.p1(buf.len() as u8);
            for &v in buf.iter() {
                client.p2(v);
            }
            buf.clear();
        }
        fn flush_recol(client: &mut PackedData, s: &mut Vec<u16>, d: &mut Vec<u16>) {
            if s.is_empty() {
                return;
            }
            let count = s.len().min(d.len());
            client.p1(40);
            client.p1(count as u8);
            for i in 0..count {
                client.p2(s[i]);
                client.p2(d[i]);
            }
            s.clear();
            d.clear();
        }
        fn flush_retex(client: &mut PackedData, s: &mut Vec<u16>, d: &mut Vec<u16>) {
            if s.is_empty() {
                return;
            }
            let count = s.len().min(d.len());
            client.p1(41);
            client.p1(count as u8);
            for i in 0..count {
                client.p2(s[i]);
                client.p2(d[i]);
            }
            s.clear();
            d.clear();
        }
        fn flush_colours(client: &mut PackedData, buf: &mut Vec<u8>) {
            if buf.is_empty() {
                return;
            }
            client.p1(42);
            client.p1(buf.len() as u8);
            for &v in buf.iter() {
                client.p1(v);
            }
            buf.clear();
        }
        fn flush_translations(client: &mut PackedData, buf: &mut Vec<(u8, i8, i8, i8)>) {
            if buf.is_empty() {
                return;
            }
            client.p1(121);
            client.p1(buf.len() as u8);
            for &(idx, x, y, z) in buf.iter() {
                client.p1(idx);
                client.p1(x as u8);
                client.p1(y as u8);
                client.p1(z as u8);
            }
            buf.clear();
        }
        fn flush_campaigns(client: &mut PackedData, buf: &mut Vec<i16>) {
            if buf.is_empty() {
                return;
            }
            client.p1(160);
            client.p1(buf.len() as u8);
            for &c in buf.iter() {
                client.p2(c as u16);
            }
            buf.clear();
        }
        #[allow(clippy::too_many_arguments)]
        fn flush_multi(
            client: &mut PackedData,
            active: &mut bool,
            var: &mut i32,
            varp: &mut i32,
            default: &mut Option<i32>,
            transforms: &mut Vec<i32>,
        ) {
            if !*active {
                return;
            }
            let has_default = default.is_some();
            client.p1(if has_default { 118 } else { 106 });
            client.p2(if *var == -1 { 65535u16 } else { *var as u16 });
            client.p2(if *varp == -1 { 65535u16 } else { *varp as u16 });
            if let Some(def) = *default {
                client.p2(if def == -1 { 65535u16 } else { def as u16 });
            }
            let count = transforms.len().saturating_sub(1);
            client.p1(count as u8);
            for &t in transforms.iter() {
                client.p2(if t == -1 { 65535u16 } else { t as u16 });
            }
            *active = false;
            *var = -1;
            *varp = -1;
            *default = None;
            transforms.clear();
        }
        fn flush_sound(
            client: &mut PackedData,
            active: &mut bool,
            idle: &mut i32,
            crawl: &mut i32,
            walk: &mut i32,
            run: &mut i32,
            dist: &mut u8,
        ) {
            if !*active {
                return;
            }
            client.p1(134);
            client.p2(if *idle == -1 { 65535u16 } else { *idle as u16 });
            client.p2(if *crawl == -1 {
                65535u16
            } else {
                *crawl as u16
            });
            client.p2(if *walk == -1 { 65535u16 } else { *walk as u16 });
            client.p2(if *run == -1 { 65535u16 } else { *run as u16 });
            client.p1(*dist);
            *active = false;
            *idle = -1;
            *crawl = -1;
            *walk = -1;
            *run = -1;
            *dist = 0;
        }
        fn flush_npc_params(client: &mut PackedData, buf: &mut Vec<(u32, bool, String)>) {
            if buf.is_empty() {
                return;
            }
            client.p1(249);
            client.p1(buf.len() as u8);
            for (key, is_string, val) in buf.iter() {
                if *is_string {
                    client.p1(1);
                    client.p3(*key);
                    client.pjstr(val);
                } else {
                    client.p1(0);
                    client.p3(*key);
                    client.p4(val.parse::<i32>().unwrap_or(0));
                }
            }
            buf.clear();
        }

        // Main emission-order walk (uses client configs from all.npc only)
        let prop_list = props.map(|v| v.as_slice()).unwrap_or(&[]);
        for (key, value) in prop_list {
            let k = key.as_str();

            // Determine which batch families this key belongs to
            let im = is_model_key(k);
            let ih = is_head_key(k);
            let ir = is_recol_key(k);
            let it = is_retex_key(k);
            let ic = is_colour_key(k);
            let itr = is_translate_key(k);
            let ica = is_campaign_key(k);
            let imu = is_multi_key(k);
            let ism = is_sound_key(k);
            let ip = is_param_key(k);

            // Flush batch buffers when switching away from their family
            if !im {
                flush_models(&mut client, &mut models_buf);
            }
            if !ih {
                flush_heads(&mut client, &mut heads_buf);
            }
            if !ir {
                flush_recol(&mut client, &mut recol_s_buf, &mut recol_d_buf);
            }
            if !it {
                flush_retex(&mut client, &mut retex_s_buf, &mut retex_d_buf);
            }
            if !ic {
                flush_colours(&mut client, &mut colours_buf);
            }
            if !itr {
                flush_translations(&mut client, &mut translate_buf);
            }
            if !ica {
                flush_campaigns(&mut client, &mut campaign_buf);
            }
            if !imu {
                flush_multi(
                    &mut client,
                    &mut multi_active,
                    &mut multi_var_buf,
                    &mut multi_varp_buf,
                    &mut multi_default_buf,
                    &mut multi_transforms_buf,
                );
            }
            if !ism {
                flush_sound(
                    &mut client,
                    &mut sound_active,
                    &mut sound_idle,
                    &mut sound_crawl,
                    &mut sound_walk,
                    &mut sound_run,
                    &mut sound_dist,
                );
            }
            if !ip {
                flush_npc_params(&mut client, &mut params_buf);
            }

            // Server-only keys: skip in client pass
            if matches!(
                k,
                "category"
                    | "attack"
                    | "defence"
                    | "strength"
                    | "hitpoints"
                    | "ranged"
                    | "magic"
                    | "wanderrange"
                    | "maxrange"
                    | "huntrange"
                    | "timer"
                    | "respawnrate"
                    | "moverestrict"
                    | "attackrange"
                    | "blockwalk"
                    | "defaultmode"
                    | "regenrate"
                    | "huntmode"
                    | "members"
                    | "givechase"
                    | "param"
            ) || k.starts_with("patrol")
            {
                continue;
            }

            // Batch accumulation and single-key emission
            if im {
                // model1, model2, ...
                models_buf.push(resolve_model_id(value, registry));
            } else if ih {
                // head1, head2, ...
                heads_buf.push(resolve_model_id(value, registry));
            } else if ir {
                // recol<N>s / recol<N>d — interleaved pairs from emitter
                if let Some(rest) = k.strip_prefix("recol") {
                    if rest.ends_with('s') {
                        if let Ok(v) = value.parse::<u16>() {
                            recol_s_buf.push(v);
                        }
                    } else if rest.ends_with('d') {
                        if let Ok(v) = value.parse::<u16>() {
                            recol_d_buf.push(v);
                        }
                    }
                }
            } else if it {
                // retex<N>s / retex<N>d
                if let Some(rest) = k.strip_prefix("retex") {
                    if rest.ends_with('s') {
                        if let Ok(v) = value.parse::<u16>() {
                            retex_s_buf.push(v);
                        }
                    } else if rest.ends_with('d') {
                        if let Ok(v) = value.parse::<u16>() {
                            retex_d_buf.push(v);
                        }
                    }
                }
            } else if ic {
                // colour1, colour2, ...
                if let Ok(v) = value.parse::<u8>() {
                    colours_buf.push(v);
                }
            } else if itr {
                // translate1, translate2, ...
                let parts: Vec<&str> = value.split(',').collect();
                if parts.len() == 4 {
                    if let (Ok(i), Ok(x), Ok(y), Ok(z)) = (
                        parts[0].trim().parse::<u8>(),
                        parts[1].trim().parse::<i8>(),
                        parts[2].trim().parse::<i8>(),
                        parts[3].trim().parse::<i8>(),
                    ) {
                        translate_buf.push((i, x, y, z));
                    }
                }
            } else if ica {
                // campaign1, campaign2, ...
                if let Ok(v) = value.parse::<i16>() {
                    campaign_buf.push(v);
                }
            } else if imu {
                // multivar, multivarp, multidefault, multi1..N
                multi_active = true;
                if k == "multivar" {
                    // Resolve varbit name → id
                    if value == "null" || value == "-1" {
                        multi_var_buf = -1;
                    } else if let Some(id) = registry.varbit.get_by_name(value) {
                        multi_var_buf = id as i32;
                    } else if let Ok(id) = value.parse::<i32>() {
                        multi_var_buf = id;
                    }
                } else if k == "multivarp" {
                    if value == "null" || value == "-1" {
                        multi_varp_buf = -1;
                    } else if let Some(id) = registry.varp.get_by_name(value) {
                        multi_varp_buf = id as i32;
                    } else if let Ok(id) = value.parse::<i32>() {
                        multi_varp_buf = id;
                    }
                } else if k == "multidefault" {
                    if value == "null" || value == "-1" {
                        multi_default_buf = Some(-1);
                    } else if let Some(id) = registry.npc.get_by_name(value) {
                        multi_default_buf = Some(id as i32);
                    } else if let Ok(id) = value.parse::<i32>() {
                        multi_default_buf = Some(id);
                    }
                } else {
                    // multi1, multi2, ... — transform entries in order
                    if value == "null" || value == "-1" {
                        multi_transforms_buf.push(-1);
                    } else if let Some(id) = registry.npc.get_by_name(value) {
                        multi_transforms_buf.push(id as i32);
                    } else if let Ok(id) = value.parse::<i32>() {
                        multi_transforms_buf.push(id);
                    }
                }
            } else if ism {
                // idlesound, crawlsound, walksound, runsound, sounddistance
                sound_active = true;
                match k {
                    "idlesound" => sound_idle = value.parse().unwrap_or(-1),
                    "crawlsound" => sound_crawl = value.parse().unwrap_or(-1),
                    "walksound" => sound_walk = value.parse().unwrap_or(-1),
                    "runsound" => sound_run = value.parse().unwrap_or(-1),
                    "sounddistance" => sound_dist = value.parse().unwrap_or(0),
                    _ => {}
                }
            } else if ip {
                // paramint=name,val / paramstr=name,val
                if let Some((pname, pval)) = value.split_once(',') {
                    let pname = pname.trim();
                    let pval = pval.trim();
                    if let Some(pid) = registry.param.get_by_name(pname) {
                        params_buf.push((pid, k == "paramstr", pval.to_string()));
                    } else if let Ok(pid) = pname.parse::<u32>() {
                        params_buf.push((pid, k == "paramstr", pval.to_string()));
                    } else {
                        warn!("Unknown param '{}' for [{}]", pname, name);
                    }
                }
            } else {
                // Single-key opcodes — emit inline
                match k {
                    "name" => {
                        // Name (opcode 2) goes to the client dat (display) AND
                        // the server dat: the server runtime reads it for
                        // `npc_name`, examine text, etc. Without the server
                        // copy, NpcType.name is None and ops fall back to the
                        // lowercase debug symbol.
                        client.p1(2);
                        client.pjstr(value);
                        server.p1(2);
                        server.pjstr(value);
                    }
                    // Size (12) and combat level (95) go to both dats, as
                    // Jordan's packer has them: the server paths and collides
                    // by size (without it every npc was one tile, and a
                    // player could stand in a cow's other three), and hunts
                    // by combat level (`check_nottoostrong`).
                    "size" => {
                        if let Ok(s) = value.parse::<u8>() {
                            client.p1(12);
                            client.p1(s);
                            server.p1(12);
                            server.p1(s);
                        }
                    }
                    // `hidden` is the server's: an op the client never
                    // shows but p_opnpc can run (289's fishing spots loop
                    // their catch on op4/op5). The server stream below keeps
                    // it; the cache has no such op.
                    "op1" | "op2" | "op3" | "op4" | "op5" if value != "hidden" => {
                        client.p1(29 + (k.as_bytes()[2] - b'0'));
                        client.pjstr(value);
                    }
                    "minimap" => {
                        if value == "no" || value == "false" {
                            client.p1(93);
                        }
                    }
                    "vislevel" => {
                        let vl = if value == "hide" {
                            Some(0)
                        } else {
                            value.parse::<u16>().ok()
                        };
                        if let Some(vl) = vl {
                            client.p1(95);
                            client.p2(vl);
                            server.p1(95);
                            server.p2(vl);
                        }
                    }
                    "resizeh" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(97);
                            client.p2(val);
                        }
                    }
                    "resizev" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(98);
                            client.p2(val);
                        }
                    }
                    "alwaysontop" => {
                        if value == "yes" || value == "true" {
                            client.p1(99);
                        }
                    }
                    "ambient" => {
                        if let Ok(val) = value.parse::<i8>() {
                            client.p1(100);
                            client.p1(val as u8);
                        }
                    }
                    "contrast" => {
                        // Text stores the raw byte (not * 5); write directly.
                        if let Ok(val) = value.parse::<i8>() {
                            client.p1(101);
                            client.p1(val as u8);
                        }
                    }
                    "headicon" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(102);
                            client.p2(val);
                        }
                    }
                    "turnspeed" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(103);
                            client.p2(val);
                        }
                    }
                    "clickable" => {
                        if value == "no" || value == "false" {
                            client.p1(107);
                        }
                    }
                    "slowwalk" => {
                        if value == "no" || value == "false" {
                            client.p1(109);
                        }
                    }
                    "animateidle" => {
                        if value == "no" || value == "false" {
                            client.p1(111);
                        }
                    }
                    "shadowcolour" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(a), Ok(b)) = (
                                parts[0].trim().parse::<i16>(),
                                parts[1].trim().parse::<i16>(),
                            ) {
                                client.p1(113);
                                client.p2(a as u16);
                                client.p2(b as u16);
                            }
                        }
                    }
                    "shadowmod" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(a), Ok(b)) =
                                (parts[0].trim().parse::<i8>(), parts[1].trim().parse::<i8>())
                            {
                                client.p1(114);
                                client.p1(a as u8);
                                client.p1(b as u8);
                            }
                        }
                    }
                    "walkmask" => {
                        if let Ok(val) = value.parse::<i8>() {
                            client.p1(119);
                            client.p1(val as u8);
                        }
                    }
                    "hitbar" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(122);
                            client.p2(val);
                        }
                    }
                    "height" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(123);
                            client.p2(val);
                        }
                    }
                    "respawndir" => {
                        if let Ok(val) = value.parse::<i8>() {
                            client.p1(125);
                            client.p1(val as u8);
                        }
                    }
                    "readyanim" => {
                        client.p1(127);
                        client.p2(resolve_seq_id(value, registry));
                    }
                    "unknown128" => {
                        if let Ok(val) = value.parse::<u8>() {
                            client.p1(128);
                            client.p1(val);
                        }
                    }
                    "cursor1" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(op), Ok(id)) = (
                                parts[0].trim().parse::<u8>(),
                                parts[1].trim().parse::<u16>(),
                            ) {
                                client.p1(135);
                                client.p1(op);
                                client.p2(id);
                            }
                        }
                    }
                    "cursor2" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(op), Ok(id)) = (
                                parts[0].trim().parse::<u8>(),
                                parts[1].trim().parse::<u16>(),
                            ) {
                                client.p1(136);
                                client.p1(op);
                                client.p2(id);
                            }
                        }
                    }
                    "attackcursor" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(137);
                            client.p2(val);
                        }
                    }
                    "armyicon" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(138);
                            client.p2(val);
                        }
                    }
                    "spriteid" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(139);
                            client.p2(val);
                        }
                    }
                    "soundvolume" => {
                        if let Ok(val) = value.parse::<u8>() {
                            client.p1(140);
                            client.p1(val);
                        }
                    }
                    "visiblepriority" => {
                        if value == "yes" || value == "true" {
                            client.p1(141);
                        }
                    }
                    "mapfunction" => {
                        if let Ok(val) = value.parse::<u16>() {
                            client.p1(142);
                            client.p2(val);
                        }
                    }
                    "invisiblepriority" => {
                        if value == "yes" || value == "true" {
                            client.p1(143);
                        }
                    }
                    "memberop1" => {
                        client.p1(150);
                        client.pjstr(value);
                    }
                    "memberop2" => {
                        client.p1(151);
                        client.pjstr(value);
                    }
                    "memberop3" => {
                        client.p1(152);
                        client.pjstr(value);
                    }
                    "memberop4" => {
                        client.p1(153);
                        client.pjstr(value);
                    }
                    "memberop5" => {
                        client.p1(154);
                        client.pjstr(value);
                    }
                    "tint" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 4 {
                            if let (Ok(h), Ok(s), Ok(l), Ok(o)) = (
                                parts[0].trim().parse::<i8>(),
                                parts[1].trim().parse::<i8>(),
                                parts[2].trim().parse::<i8>(),
                                parts[3].trim().parse::<i8>(),
                            ) {
                                client.p1(155);
                                client.p1(h as u8);
                                client.p1(s as u8);
                                client.p1(l as u8);
                                client.p1(o as u8);
                            }
                        }
                    }
                    "mainoptionindex" => match value.as_str() {
                        "1" => client.p1(158),
                        "0" => client.p1(159),
                        _ => {}
                    },
                    "vorbis" => {
                        if value == "yes" || value == "true" {
                            client.p1(162);
                        }
                    }
                    "slayertype" => {
                        if let Ok(val) = value.parse::<u8>() {
                            client.p1(163);
                            client.p1(val);
                        }
                    }
                    "soundrate" => {
                        let parts: Vec<&str> = value.split(',').collect();
                        if parts.len() == 2 {
                            if let (Ok(a), Ok(b)) = (
                                parts[0].trim().parse::<u16>(),
                                parts[1].trim().parse::<u16>(),
                            ) {
                                client.p1(164);
                                client.p2(a);
                                client.p2(b);
                            }
                        }
                    }
                    "picksizeshift" => {
                        if let Ok(val) = value.parse::<u8>() {
                            client.p1(165);
                            client.p1(val);
                        }
                    }
                    "soundrangemin" => {
                        if let Ok(val) = value.parse::<u8>() {
                            client.p1(168);
                            client.p1(val);
                        }
                    }
                    _ => {
                        // Unknown client key — skip silently (server-only or unrecognised)
                    }
                }
            }
        }
        // Flush any remaining batch buffers
        flush_models(&mut client, &mut models_buf);
        flush_heads(&mut client, &mut heads_buf);
        flush_recol(&mut client, &mut recol_s_buf, &mut recol_d_buf);
        flush_retex(&mut client, &mut retex_s_buf, &mut retex_d_buf);
        flush_colours(&mut client, &mut colours_buf);
        flush_translations(&mut client, &mut translate_buf);
        flush_campaigns(&mut client, &mut campaign_buf);
        flush_multi(
            &mut client,
            &mut multi_active,
            &mut multi_var_buf,
            &mut multi_varp_buf,
            &mut multi_default_buf,
            &mut multi_transforms_buf,
        );
        flush_sound(
            &mut client,
            &mut sound_active,
            &mut sound_idle,
            &mut sound_crawl,
            &mut sound_walk,
            &mut sound_run,
            &mut sound_dist,
        );
        flush_npc_params(&mut client, &mut params_buf);

        // =====================================================================
        // SERVER opcodes → server PackedData
        // =====================================================================

        // Opcode 3: desc, the examine text. 647's client has none; the server
        // answers OPNPC6 with it.
        if let Some((_, v)) = prop_list.iter().find(|(k, _)| k == "desc") {
            server.p1(3);
            server.pjstr(v);
        }

        // Opcodes 30-34: op1-op5. 2011's cache gives many ops as memberops
        // (client 150-154: a members world shows them in that slot, a free
        // one hides them), 115 of them Pickpocket; without them here the
        // server refused the op the client offered. 647's server runs a
        // members world, as its objs load (`ObjContext { members: true }`),
        // so a memberop takes its slot.
        for (slot, opcode) in (1..=5).zip(30u8..) {
            let op = format!("op{slot}");
            let memberop = format!("memberop{slot}");
            let value = prop_list
                .iter()
                .find(|(k, _)| *k == memberop)
                .or_else(|| prop_list.iter().find(|(k, _)| *k == op));
            if let Some((_, v)) = value {
                server.p1(opcode);
                server.pjstr(v);
            }
        }

        // Opcode 18: category
        if let Some((_, v)) = prop_list.iter().find(|(k, _)| k == "category") {
            if let Some(cat_id) = registry.category.get_by_name(v) {
                server.p1(18);
                server.p2(cat_id as u16);
            }
        }
        // Stats: 74-79
        for (key, opcode) in [
            ("attack", 74u8),
            ("defence", 75),
            ("strength", 76),
            ("hitpoints", 77),
            ("ranged", 78),
            ("magic", 79),
        ] {
            if let Some((_, v)) = prop_list.iter().find(|(k, _)| k == key) {
                if let Ok(val) = v.parse::<u16>() {
                    server.p1(opcode);
                    server.p2(val);
                }
            }
        }
        // Server behavior: 200-214
        for (key, opcode, size) in [
            ("wanderrange", 200u8, 2u8),
            ("maxrange", 201, 2),
            ("huntrange", 202, 1),
            ("timer", 203, 2),
            ("respawnrate", 204, 2),
            ("moverestrict", 206, 1),
            ("attackrange", 207, 2),
            ("blockwalk", 208, 1),
            ("defaultmode", 210, 1),
            ("regenrate", 214, 2),
        ] {
            if let Some((_, v)) = prop_list.iter().find(|(k, _)| k == key) {
                let val = match key {
                    "moverestrict" => match v.as_str() {
                        "normal" => 0i32,
                        "blocked" => 1,
                        "blocked+normal" => 2,
                        "indoors" => 3,
                        "outdoors" => 4,
                        "nomove" => 5,
                        "passthru" => 6,
                        _ => v.parse().unwrap_or(0),
                    },
                    "blockwalk" => match v.as_str() {
                        "none" => 0,
                        "all" => 1,
                        "npc" => 2,
                        _ => v.parse().unwrap_or(2),
                    },
                    "defaultmode" => match v.as_str() {
                        "none" => 0,
                        "wander" => 1,
                        "patrol" => 2,
                        _ => v.parse().unwrap_or(0),
                    },
                    _ => v.parse().unwrap_or(0),
                };
                server.p1(opcode);
                if size == 1 {
                    server.p1(val as u8);
                } else {
                    server.p2(val as u16);
                }
            }
        }
        // Opcode 209: huntmode
        if let Some((_, v)) = prop_list.iter().find(|(k, _)| k == "huntmode") {
            if let Some(hunt_id) = registry.hunt.get_by_name(v) {
                server.p1(209);
                server.p1(hunt_id as u8);
            } else if let Ok(val) = v.parse::<u8>() {
                server.p1(209);
                server.p1(val);
            } else {
                warn!("Unknown huntmode '{}' for [{}]", v, name);
            }
        }
        // Opcode 211: members
        if prop_list
            .iter()
            .any(|(k, v)| k == "members" && (v == "yes" || v == "true"))
        {
            server.p1(211);
        }
        // Opcode 212: patrol
        if !patrols.is_empty() {
            server.p1(212);
            server.p1(patrols.len() as u8);
            for (coord, delay) in &patrols {
                server.p4(*coord);
                server.p1(*delay);
            }
        }
        // Opcode 213: givechase=no
        if prop_list
            .iter()
            .any(|(k, v)| k == "givechase" && (v == "no" || v == "false"))
        {
            server.p1(213);
        }
        // Opcode 249: params
        let params: Vec<(&str, &str)> = prop_list
            .iter()
            .filter(|(k, _)| k == "param")
            .filter_map(|(_, v)| v.split_once(','))
            .collect();
        // Each entry as `NpcType` reads it back: `g1(is_string) g3(key)` and
        // the value. The value takes the param's declared type, so
        // `fishing_movement_enum,fishing_movement_draynor_village_enum` packs
        // the enum's id, not its name as a string.
        if !params.is_empty() {
            server.p1(249);
            server.p1(params.len() as u8);
            for (param_name, param_value) in &params {
                let param_name = param_name.trim();
                let param_value = param_value.trim();
                let Some(param_id) = registry.param.get_by_name(param_name) else {
                    warn!("Unknown param '{}' for [{}]", param_name, name);
                    server.pbool(false);
                    server.p3(0);
                    server.p4(0);
                    continue;
                };
                let type_char = param_types.get(&param_id).copied().unwrap_or('i');
                if type_char == 's' {
                    server.pbool(true);
                    server.p3(param_id);
                    server.pjstr(param_value);
                } else {
                    server.pbool(false);
                    server.p3(param_id);
                    server.p4(super::param::param_int(param_value, type_char, registry));
                }
            }
        }
        // Opcode 250: debugname (server-only)
        server.p1(250);
        server.pjstr(name);
        server.finish_entry();
        if has_client_fields {
            client.finish_entry();
        }
    }

    server.save(server_dir, "npc")?;
    config_blob.add_packed_data("npc", client.dat, client.idx);

    Ok(())
}

// ─── Cache text emitter (archive 18) ────────────────────────────────────

use crate::cache647::{NpcType, ParamValue};

fn name_for_model(id: u16, registry: &PackRegistry) -> String {
    registry
        .model
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc"))
}

fn name_for_head_model(id: u16, registry: &PackRegistry) -> String {
    registry
        .model
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc_head"))
}

fn name_for_seq(id: i32, registry: &PackRegistry) -> String {
    if id < 0 {
        return id.to_string();
    }
    registry
        .seq
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("seq_{id}"))
}

fn name_for_varbit(id: i32, registry: &PackRegistry) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .varbit
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("varbit_{id}"))
}

fn name_for_varp(id: i32, registry: &PackRegistry) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .varp
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("varp_{id}"))
}

fn name_for_npc(id: i32, registry: &PackRegistry) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .npc
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("npc_{id}"))
}

fn name_for_param(id: u32, registry: &PackRegistry) -> String {
    registry
        .param
        .get_by_id(id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.to_string())
}

/// Produce the (key, value) pairs for an NPC config's text representation.
/// Opcodes are emitted in canonical ascending order matching the binary
/// layout. Cross-references resolve through `registry`.
pub fn emit_npc_props(def: &NpcType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if let Some(models) = &def.models {
        for (i, id) in models.iter().enumerate() {
            p.push((1, format!("model{}", i + 1), name_for_model(*id, registry)));
        }
    }
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }
    if def.opcode_order.contains(&12) || def.size != 1 {
        p.push((12, "size".into(), def.size.to_string()));
    }
    for (slot, opt) in def.options.iter().enumerate() {
        if let Some(text) = opt {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            p.push((40, format!("recol{}s", i + 1), src[i].to_string()));
            p.push((40, format!("recol{}d", i + 1), dst[i].to_string()));
        }
    }
    if let (Some(src), Some(dst)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            p.push((41, format!("retex{}s", i + 1), src[i].to_string()));
            p.push((41, format!("retex{}d", i + 1), dst[i].to_string()));
        }
    }
    if let Some(palette) = &def.colour_palette {
        for (i, c) in palette.iter().enumerate() {
            p.push((42, format!("colour{}", i + 1), c.to_string()));
        }
    }
    if let Some(heads) = &def.head_models {
        for (i, id) in heads.iter().enumerate() {
            p.push((
                60,
                format!("head{}", i + 1),
                name_for_head_model(*id, registry),
            ));
        }
    }
    if !def.minimap {
        p.push((93, "minimap".into(), "no".into()));
    }
    if def.opcode_order.contains(&95) || def.combat != -1 {
        if def.combat == 0 {
            p.push((95, "vislevel".into(), "hide".into()));
        } else {
            p.push((95, "vislevel".into(), def.combat.to_string()));
        }
    }
    if def.opcode_order.contains(&97) || def.resizeh != -1 {
        p.push((97, "resizeh".into(), def.resizeh.to_string()));
    }
    if def.opcode_order.contains(&98) || def.resizev != -1 {
        p.push((98, "resizev".into(), def.resizev.to_string()));
    }
    if def.alwaysontop {
        p.push((99, "alwaysontop".into(), "yes".into()));
    }
    if def.opcode_order.contains(&100) || def.ambient != 0 {
        p.push((100, "ambient".into(), def.ambient.to_string()));
    }
    if def.opcode_order.contains(&101) || def.contrast != 0 {
        p.push((101, "contrast".into(), (def.contrast / 5).to_string()));
    }
    if def.opcode_order.contains(&102) || def.headicon != -1 {
        p.push((102, "headicon".into(), def.headicon.to_string()));
    }
    if def.opcode_order.contains(&103) || def.turnspeed != 32 {
        p.push((103, "turnspeed".into(), def.turnspeed.to_string()));
    }
    if let Some(transforms) = &def.transforms {
        let op = if transforms.last().copied().unwrap_or(-1) != -1 {
            118
        } else {
            106
        };
        p.push((
            op,
            "multivar".into(),
            name_for_varbit(def.transform_varbit, registry),
        ));
        p.push((
            op,
            "multivarp".into(),
            name_for_varp(def.transform_varp, registry),
        ));
        let default_transform = *transforms.last().unwrap_or(&-1);
        let entries = &transforms[..transforms.len().saturating_sub(1)];
        if default_transform != -1 {
            p.push((
                op,
                "multidefault".into(),
                name_for_npc(default_transform, registry),
            ));
        }
        for (i, t) in entries.iter().enumerate() {
            p.push((op, format!("multi{}", i + 1), name_for_npc(*t, registry)));
        }
    }
    if !def.clickable {
        p.push((107, "clickable".into(), "no".into()));
    }
    if !def.slowwalk {
        p.push((109, "slowwalk".into(), "no".into()));
    }
    if !def.animateidle {
        p.push((111, "animateidle".into(), "no".into()));
    }
    if def.opcode_order.contains(&113)
        || def.primary_shadow_colour != 0
        || def.secondary_shadow_colour != 0
    {
        p.push((
            113,
            "shadowcolour".into(),
            format!(
                "{},{}",
                def.primary_shadow_colour, def.secondary_shadow_colour
            ),
        ));
    }
    if def.opcode_order.contains(&114)
        || def.primary_shadow_modifier != 0
        || def.secondary_shadow_modifier != 0
    {
        p.push((
            114,
            "shadowmod".into(),
            format!(
                "{},{}",
                def.primary_shadow_modifier, def.secondary_shadow_modifier
            ),
        ));
    }
    if def.opcode_order.contains(&119) || def.walkmask != 0 {
        p.push((119, "walkmask".into(), def.walkmask.to_string()));
    }
    if let Some(translations) = &def.translations {
        for (i, (idx, [x, y, z])) in translations.iter().enumerate() {
            p.push((
                121,
                format!("translate{}", i + 1),
                format!("{idx},{x},{y},{z}"),
            ));
        }
    }
    if def.opcode_order.contains(&122) || def.hitbar_sprite != -1 {
        p.push((122, "hitbar".into(), def.hitbar_sprite.to_string()));
    }
    if def.opcode_order.contains(&123) || def.height != -1 {
        p.push((123, "height".into(), def.height.to_string()));
    }
    if def.opcode_order.contains(&125) || def.respawn_direction != 0 {
        p.push((125, "respawndir".into(), def.respawn_direction.to_string()));
    }
    if def.opcode_order.contains(&127) || def.render_emote != -1 {
        p.push((
            127,
            "readyanim".into(),
            name_for_seq(def.render_emote, registry),
        ));
    }
    if def.opcode_order.contains(&128) || def.op128 != 0 {
        p.push((128, "unknown128".into(), def.op128.to_string()));
    }
    if def.opcode_order.contains(&134)
        || def.idle_sound != -1
        || def.crawl_sound != -1
        || def.walk_sound != -1
        || def.run_sound != -1
    {
        p.push((134, "idlesound".into(), def.idle_sound.to_string()));
        p.push((134, "crawlsound".into(), def.crawl_sound.to_string()));
        p.push((134, "walksound".into(), def.walk_sound.to_string()));
        p.push((134, "runsound".into(), def.run_sound.to_string()));
        p.push((134, "sounddistance".into(), def.sound_distance.to_string()));
    }
    if def.primary_cursor != 0 || def.primary_cursor_op != 0 {
        p.push((
            135,
            "cursor1".into(),
            format!("{},{}", def.primary_cursor_op, def.primary_cursor),
        ));
    }
    if def.secondary_cursor != 0 || def.secondary_cursor_op != 0 {
        p.push((
            136,
            "cursor2".into(),
            format!("{},{}", def.secondary_cursor_op, def.secondary_cursor),
        ));
    }
    if def.opcode_order.contains(&137) || def.attack_cursor != -1 {
        p.push((137, "attackcursor".into(), def.attack_cursor.to_string()));
    }
    if def.opcode_order.contains(&138) || def.army_icon != -1 {
        p.push((138, "armyicon".into(), def.army_icon.to_string()));
    }
    if def.opcode_order.contains(&139) || def.sprite_id != -1 {
        p.push((139, "spriteid".into(), def.sprite_id.to_string()));
    }
    if def.opcode_order.contains(&140) || def.ambient_sound_volume != 0 {
        p.push((
            140,
            "soundvolume".into(),
            def.ambient_sound_volume.to_string(),
        ));
    }
    if def.visible_priority {
        p.push((141, "visiblepriority".into(), "yes".into()));
    }
    if def.opcode_order.contains(&142) || def.mapfunction != -1 {
        p.push((142, "mapfunction".into(), def.mapfunction.to_string()));
    }
    if def.invisible_priority {
        p.push((143, "invisiblepriority".into(), "yes".into()));
    }
    for (slot, opt) in def.members_options.iter().enumerate() {
        if let Some(text) = opt {
            p.push((
                150 + slot as u8,
                format!("memberop{}", slot + 1),
                text.to_string(),
            ));
        }
    }
    if def.opcode_order.contains(&155)
        || def.hue != 0
        || def.saturation != 0
        || def.lightness != 0
        || def.opacity != 0
    {
        p.push((
            155,
            "tint".into(),
            format!(
                "{},{},{},{}",
                def.hue, def.saturation, def.lightness, def.opacity
            ),
        ));
    }
    if def.main_option_index != -1 {
        let op = if def.main_option_index == 1 { 158 } else { 159 };
        p.push((
            op,
            "mainoptionindex".into(),
            def.main_option_index.to_string(),
        ));
    }
    if let Some(campaigns) = &def.campaigns {
        for (i, v) in campaigns.iter().enumerate() {
            p.push((160, format!("campaign{}", i + 1), v.to_string()));
        }
    }
    if def.vorbis {
        p.push((162, "vorbis".into(), "yes".into()));
    }
    if def.opcode_order.contains(&163) || def.slayer_type != 0 {
        p.push((163, "slayertype".into(), def.slayer_type.to_string()));
    }
    if def.opcode_order.contains(&164) || def.sound_rate_min != 0 || def.sound_rate_max != 0 {
        p.push((
            164,
            "soundrate".into(),
            format!("{},{}", def.sound_rate_min, def.sound_rate_max),
        ));
    }
    if def.opcode_order.contains(&165) || def.pick_size_shift != 0 {
        p.push((165, "picksizeshift".into(), def.pick_size_shift.to_string()));
    }
    if def.opcode_order.contains(&168) || def.sound_range_min != 0 {
        p.push((168, "soundrangemin".into(), def.sound_range_min.to_string()));
    }
    if let Some(params) = &def.params {
        // Decode all param entries from raw opcode data (preserves earlier
        // values for duplicate keys that were overwritten in the HashMap).
        let raw_param_values = decode_raw_params(&def.raw_opcodes);

        if !def.param_order.is_empty() && !raw_param_values.is_empty() {
            // Use raw values indexed positionally for byte-perfect output
            for (i, k) in def.param_order.iter().enumerate() {
                let pname = name_for_param(*k as u32, registry);
                if let Some(raw_val) = raw_param_values.get(i) {
                    match raw_val {
                        ParamValue::Int(v) => {
                            p.push((249, "paramint".into(), format!("{pname},{v}")));
                        }
                        ParamValue::String(s) => {
                            p.push((249, "paramstr".into(), format!("{pname},{s}")));
                        }
                    }
                } else if let Some(val) = params.get(k) {
                    match val {
                        ParamValue::Int(v) => {
                            p.push((249, "paramint".into(), format!("{pname},{v}")));
                        }
                        ParamValue::String(s) => {
                            p.push((249, "paramstr".into(), format!("{pname},{s}")));
                        }
                    }
                }
            }
        } else {
            let ordered: Vec<_> = if !def.param_order.is_empty() {
                def.param_order
                    .iter()
                    .filter_map(|k| params.get(k).map(|v| (*k, v)))
                    .collect()
            } else {
                let mut sorted: Vec<_> = params.iter().map(|(k, v)| (*k, v)).collect();
                sorted.sort_by_key(|(k, _)| *k);
                sorted
            };
            for (key, val) in ordered {
                let pname = name_for_param(key as u32, registry);
                match val {
                    ParamValue::Int(i) => {
                        p.push((249, "paramint".into(), format!("{pname},{i}")));
                    }
                    ParamValue::String(s) => {
                        p.push((249, "paramstr".into(), format!("{pname},{s}")));
                    }
                }
            }
        }
    }
    super::reorder_by_opcode_with_raw_fn(&def.opcode_order, p, &def.raw_opcodes, decode_npc_raw)
}

/// Decode all param entries from the raw op249 data, preserving order
/// and individual values (even for duplicate keys that were overwritten
/// in the HashMap during normal decode).
fn decode_raw_params(raw_opcodes: &[(u8, Vec<u8>)]) -> Vec<crate::cache647::provider::ParamValue> {
    let Some((_, data)) = raw_opcodes.iter().find(|(op, _)| *op == 249) else {
        return Vec::new();
    };
    if data.is_empty() {
        return Vec::new();
    }
    let count = data[0] as usize;
    let mut pos = 1;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let is_string = data[pos] == 1;
        pos += 1;
        // Skip key (3 bytes)
        pos += 3;
        if is_string {
            // Read NUL-terminated string
            let start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]).to_string();
            pos += 1; // skip NUL
            result.push(crate::cache647::provider::ParamValue::String(
                s.into_boxed_str(),
            ));
        } else {
            if pos + 3 < data.len() {
                let v =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                result.push(crate::cache647::provider::ParamValue::Int(v));
            }
            pos += 4;
        }
    }
    result
}

/// Decode raw NPC opcode bytes back to a (key, value) text pair.
fn decode_npc_raw(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    let r_u16 = |pos: usize| -> u16 {
        if pos + 1 < raw.len() {
            u16::from_be_bytes([raw[pos], raw[pos + 1]])
        } else {
            0
        }
    };
    let r_i8 = |pos: usize| -> i8 { if pos < raw.len() { raw[pos] as i8 } else { 0 } };
    let r_u8 = |pos: usize| -> u8 { if pos < raw.len() { raw[pos] } else { 0 } };

    match opcode {
        12 => Some(("size".into(), r_u8(0).to_string())),
        95 => {
            let v = r_u16(0);
            if v == 0 {
                Some(("vislevel".into(), "hide".into()))
            } else {
                Some(("vislevel".into(), v.to_string()))
            }
        }
        97 => Some(("resizeh".into(), r_u16(0).to_string())),
        98 => Some(("resizev".into(), r_u16(0).to_string())),
        100 => Some(("ambient".into(), r_i8(0).to_string())),
        101 => Some(("contrast".into(), r_i8(0).to_string())),
        102 => Some(("headicon".into(), r_u16(0).to_string())),
        103 => Some(("turnspeed".into(), r_u16(0).to_string())),
        122 => Some(("hitbar".into(), r_u16(0).to_string())),
        123 => Some(("height".into(), r_u16(0).to_string())),
        125 => Some(("respawndir".into(), r_i8(0).to_string())),
        128 => Some(("unknown128".into(), r_u8(0).to_string())),
        140 => Some(("soundvolume".into(), r_u8(0).to_string())),
        163 => Some(("slayertype".into(), r_u8(0).to_string())),
        165 => Some(("picksizeshift".into(), r_u8(0).to_string())),
        168 => Some(("soundrangemin".into(), r_u8(0).to_string())),
        // Flag opcodes (no payload) — needed for duplicate-opcode roundtrip
        107 => Some(("clickable".into(), "no".into())),
        109 => Some(("slowwalk".into(), "no".into())),
        111 => Some(("animateidle".into(), "no".into())),
        119 => Some(("walkmask".into(), r_i8(0).to_string())),
        127 => Some(("readyanim".into(), r_u16(0).to_string())),
        _ => super::decode_raw_to_text(opcode, raw),
    }
}
