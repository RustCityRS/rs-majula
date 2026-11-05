//! Seq (animation sequence) packers — two flavours sharing one .seq text
//! format:
//!
//! 1. **Server cache (legacy)**: [`pack_seqs`] writes
//!    `data/pack/server/seq.dat`/`seq.idx` with debugnames only (op 250 in
//!    the rev-225 server format). Server-only; never transmitted.
//! 2. **Client cache (Phase 1 rebuild)**: [`emit_seq_props`] +
//!    [`pack_seq_def`] roundtrip through `SeqDefinition` for archive 20
//!    (strided u16, stride 128). `pack_seq_def(...).encode()` is byte-perfect
//!    against the JS5 cache bytes for every cache-resident seq.
//!
//! ## Seq has no op 250 in the cache format
//!
//! Unlike inv, seq's rev-647 cache decoder (`SeqTypeRaw.decode`) defines no
//! debugname opcode. The section `[name]` header is the content identifier
//! (used by registry lookups and by the unpacker for human-readable section
//! titles), but it's NOT emitted into the cache bytes.
//!
//! ## Text format (extends 2004scape's `SeqConfig.ts`)
//!
//! 2004scape's rev-225 seq config doesn't cover all 17 rev-647 opcodes.
//! Where 2004scape has a canonical key we use it; otherwise we use a literal
//! `opNN`-style key:
//!
//! | op | text key                                  | shape                              |
//! |----|-------------------------------------------|------------------------------------|
//! | 1  | `frame<N>=<duration>:<lo>:<hi>`           | one line per frame in the block    |
//! | 2  | `loopoffset=<i16>`                        | cache 0xFFFF = -1                  |
//! | 3  | `walkmerge=<u8>,<u8>,...`                | full mask as one comma-list        |
//! | 5  | `priority=<u8>`                           |                                    |
//! | 6  | `replaceheldleft=<u16>`                               | no canonical 2004scape name        |
//! | 7  | `replaceheldright=<u16>`                               | no canonical 2004scape name        |
//! | 8  | `loopcount=<u8>`                          |                                    |
//! | 9  | `interrupt=<i8>`                          |                                    |
//! | 10 | `walkmerge=<u8>`                          | postanim_move, not 2004scape's labels  |
//! | 11 | `looptype=<u8>`                           |                                    |
//! | 12 | `iframe<N>=<lo>:<hi>`                     | one line per interface-frame entry |
//! | 13 | `synth<N>=<head>:<t0>:<t1>:...` or `null` | one line per synth slot            |
//! | 14 | `op14_abool9=yes`                         | flag                               |
//! | 15 | `tween=yes`                               | flag                               |
//! | 16 | `op16_abool8=yes`                         | flag                               |
//! | 18 | `op18_abool10=yes`                        | flag                               |
//! | 19 | `op19=<idx>:<val>`                        | multi-emit allowed                 |
//! | 20 | `op20=<idx>:<a>:<b>`                      | multi-emit allowed                 |
//!
//! ### Source order is preserved
//!
//! The encoder walks `def.emissions` in original order, so text-config order
//! must match the cache order. Frames / SoundMask / InterfaceFrames / Synths
//! are each a SINGLE opcode emission containing N entries — the parser
//! buffers their numbered text keys (`frameN`, `iframeN`, `synthN`) and
//! flushes the buffer as ONE emission on the first non-batch key (or at end
//! of section). `walkmerge` is emitted in a single line and so doesn't need
//! buffering.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use crate::cache::seq::SeqTypeRaw;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

// ─── Cache-byte-perfect packer (archive 20, strided u16 ids) ─────────────

/// Walk a seq's emissions and produce the (key, value) pairs matching the
/// text format documented at the top of this module.
///
/// `registry` is unused for seq (no cross-type references in the cache
/// format) but kept for signature parity with other config packers.
pub fn emit_seq_props(def: &SeqTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if let (Some(frames), Some(delays)) = (&def.frames, &def.delays) {
        for i in 0..frames.len() {
            p.push((
                1,
                format!("frame{}", i + 1),
                format!("{}:{}:{}", delays[i], frames[i] as u16, frames[i] >> 16),
            ));
        }
    }
    if def.opcode_order.contains(&2) || def.loops.is_some() {
        p.push((
            2,
            "loopoffset".into(),
            (def.loops.unwrap_or(u16::MAX) as i16).to_string(),
        ));
    }
    if let Some(mask) = &def.walkmerge {
        let csv = mask
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(",");
        p.push((3, "sound_mask".into(), csv));
    }
    if def.opcode_order.contains(&5) || def.priority != 5 {
        p.push((5, "priority".into(), def.priority.to_string()));
    }
    if def.opcode_order.contains(&6) || def.replaceheldleft.is_some() {
        p.push((
            6,
            "op6".into(),
            def.replaceheldleft.unwrap_or(u16::MAX).to_string(),
        ));
    }
    if def.opcode_order.contains(&7) || def.replaceheldright.is_some() {
        p.push((
            7,
            "op7".into(),
            def.replaceheldright.unwrap_or(u16::MAX).to_string(),
        ));
    }
    if def.opcode_order.contains(&8) || def.maxloops != 99 {
        p.push((8, "loopcount".into(), def.maxloops.to_string()));
    }
    if def.opcode_order.contains(&9) {
        p.push((
            9,
            "interrupt".into(),
            (def.preanim_move.unwrap_or(u8::MAX) as i8).to_string(),
        ));
    }
    if def.opcode_order.contains(&10) {
        p.push((
            10,
            "walkmerge".into(),
            (def.postanim_move.unwrap_or(u8::MAX) as i8).to_string(),
        ));
    }
    if def.opcode_order.contains(&11) || def.duplicatebehaviour.unwrap_or(2) != 2 {
        p.push((
            11,
            "looptype".into(),
            def.duplicatebehaviour.unwrap_or(2).to_string(),
        ));
    }
    if let Some(iframes) = &def.iframes {
        for i in 0..iframes.len() {
            p.push((
                12,
                format!("iframe{}", i + 1),
                format!(
                    "{}:{}",
                    iframes[i].unwrap_or(u32::MAX) as u16,
                    iframes[i].unwrap_or(u32::MAX) >> 16
                ),
            ));
        }
    }
    if let Some(synths) = &def.synths {
        for (i, slot) in synths.iter().enumerate() {
            let value = match slot {
                None => "null".to_string(),
                Some(s) => {
                    if s.tail.is_empty() {
                        format!("{}", s.head)
                    } else {
                        let tail = s
                            .tail
                            .iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(":");
                        format!("{}:{}", s.head, tail)
                    }
                }
            };
            p.push((13, format!("synth{}", i + 1), value));
        }
    }
    if def.a_bool9 {
        p.push((14, "op14_abool9".into(), "yes".into()));
    }
    if def.tween {
        p.push((15, "tween".into(), "yes".into()));
    }
    if def.a_bool8 {
        p.push((16, "op16_abool8".into(), "yes".into()));
    }
    if def.a_bool10 {
        p.push((18, "op18_abool10".into(), "yes".into()));
    }
    for (idx, val) in &def.op19 {
        p.push((19, "op19".into(), format!("{idx}:{val}")));
    }
    for (idx, a, b) in &def.op20 {
        p.push((20, "op20".into(), format!("{idx}:{a}:{b}")));
    }
    super::reorder_by_opcode_with_raw_fn(&def.opcode_order, p, &def.raw_opcodes, decode_seq_raw)
}

/// Decode raw SEQ opcode bytes back to a (key, value) text pair.
fn decode_seq_raw(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    let r_u8 = |pos: usize| -> u8 { if pos < raw.len() { raw[pos] } else { 0 } };
    let r_i8 = |pos: usize| -> i8 { if pos < raw.len() { raw[pos] as i8 } else { 0 } };
    let r_u16 = |pos: usize| -> u16 {
        if pos + 1 < raw.len() {
            u16::from_be_bytes([raw[pos], raw[pos + 1]])
        } else {
            0
        }
    };

    match opcode {
        2 => Some(("loopoffset".into(), r_u16(0).to_string())),
        5 => Some(("priority".into(), r_u8(0).to_string())),
        6 => Some(("op6".into(), (r_u16(0) as i32).to_string())),
        7 => Some(("op7".into(), (r_u16(0) as i32).to_string())),
        8 => Some(("loopcount".into(), r_u8(0).to_string())),
        9 => Some(("interrupt".into(), r_i8(0).to_string())),
        10 => Some(("walkmerge".into(), (r_u8(0) as i8).to_string())),
        11 => Some(("looptype".into(), r_u8(0).to_string())),
        14 => Some(("op14_abool9".into(), "yes".into())),
        15 => Some(("tween".into(), "yes".into())),
        16 => Some(("op16_abool8".into(), "yes".into())),
        18 => Some(("op18_abool10".into(), "yes".into())),
        _ => super::decode_raw_to_text(opcode, raw),
    }
}
// ─── Legacy server-cache packer (data/pack/server/seq.dat) ───────────────
//
// Server-only, debugname-only (op 250) — same shape as 2004scape's rev-225
// seq server pack. The client cache packer above produces the actual seq
// bytecode; this packer is kept for engine-side resolution only.
pub fn pack_seqs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    let files = collect_config_files(source_dir, "seq");
    info!("  Found {} .seq files", files.len());
    let configs = parse_config_sections(&files, constants);

    let all_file = source_dir.join("_unpack").join("all.seq");
    let client_configs = if all_file.exists() {
        parse_config_sections(&[all_file], constants)
    } else {
        HashMap::new()
    };
    info!(
        "  Parsed {} seq configs ({} client)",
        configs.len(),
        client_configs.len()
    );

    let mut server = PackedData::new(registry.seq.max);
    let mut client = PackedData::new(registry.seq.max);

    for id in 0..registry.seq.max {
        let Some(name) = registry.seq.get_by_id(id) else {
            server.skip_entry();
            client.skip_entry();
            continue;
        };

        server.start_entry();
        client.start_entry();

        // Server: debugname only
        if configs.contains_key(name) {
            server.p1(250);
            server.pjstr(name);
        }

        // Client: full cache binary from all.seq
        //
        // Batch keys (frame*, iframe*, synth*) accumulate into buffers and
        // flush as a single opcode emission on the first non-batch key (or
        // at end of section), preserving the source-data opcode order.
        if let Some(props) = client_configs.get(name) {
            let mut frames: Vec<(usize, u16, u16, u16)> = Vec::new(); // (idx, duration, lo, hi)
            let mut iframes: Vec<(usize, u16, u16)> = Vec::new();
            let mut synths: Vec<(usize, String)> = Vec::new();

            for (key, value) in props {
                let k = key.as_str();

                // Flush batch buffers whose key doesn't match this line
                if !is_frame_key(k) {
                    flush_frames(&mut client, &mut frames);
                }
                if !is_iframe_key(k) {
                    flush_iframes(&mut client, &mut iframes);
                }
                if !is_synth_key(k) {
                    flush_synths(&mut client, &mut synths);
                }

                if let Some(rest) = key.strip_prefix("frame") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        let parts: Vec<&str> = value.split(':').collect();
                        if parts.len() == 3 {
                            let dur: u16 = parts[0].parse().unwrap_or(0);
                            let lo: u16 = parts[1].parse().unwrap_or(0);
                            let hi: u16 = parts[2].parse().unwrap_or(0);
                            frames.push((idx, dur, lo, hi));
                        }
                    }
                    continue;
                }
                if let Some(rest) = key.strip_prefix("iframe") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        let parts: Vec<&str> = value.split(':').collect();
                        if parts.len() == 2 {
                            let lo: u16 = parts[0].parse().unwrap_or(0);
                            let hi: u16 = parts[1].parse().unwrap_or(0);
                            iframes.push((idx, lo, hi));
                        }
                    }
                    continue;
                }
                if let Some(rest) = key.strip_prefix("synth") {
                    if let Ok(idx) = rest.parse::<usize>() {
                        synths.push((idx, value.clone()));
                    }
                    continue;
                }

                match key.as_str() {
                    "loopoffset" => {
                        let v: i16 = value.parse().unwrap_or(0);
                        client.p1(2);
                        client.p2(v as u16);
                    }
                    "sound_mask" => {
                        let vals: Vec<u8> = value
                            .split(',')
                            .filter_map(|s| s.trim().parse().ok())
                            .collect();
                        client.p1(3);
                        client.p1(vals.len() as u8);
                        for v in &vals {
                            client.p1(*v);
                        }
                    }
                    "priority" => {
                        let v: u8 = value.parse().unwrap_or(0);
                        client.p1(5);
                        client.p1(v);
                    }
                    "op6" => {
                        let v: u16 = value.parse().unwrap_or(0);
                        client.p1(6);
                        client.p2(v);
                    }
                    "op7" => {
                        let v: u16 = value.parse().unwrap_or(0);
                        client.p1(7);
                        client.p2(v);
                    }
                    "loopcount" => {
                        let v: u8 = value.parse().unwrap_or(0);
                        client.p1(8);
                        client.p1(v);
                    }
                    "interrupt" => {
                        let v: i8 = value.parse().unwrap_or(0);
                        client.p1(9);
                        client.p1(v as u8);
                    }
                    "walkmerge" => {
                        let v: u8 = value.parse().unwrap_or(0);
                        client.p1(10);
                        client.p1(v);
                    }
                    "looptype" => {
                        let v: u8 = value.parse().unwrap_or(0);
                        client.p1(11);
                        client.p1(v);
                    }
                    "op14_abool9" => {
                        client.p1(14);
                    }
                    "tween" => {
                        client.p1(15);
                    }
                    "op16_abool8" => {
                        client.p1(16);
                    }
                    "op18_abool10" => {
                        client.p1(18);
                    }
                    "op19" => {
                        let parts: Vec<&str> = value.split(':').collect();
                        if parts.len() == 2 {
                            let idx: u8 = parts[0].parse().unwrap_or(0);
                            let val: u8 = parts[1].parse().unwrap_or(0);
                            client.p1(19);
                            client.p1(idx);
                            client.p1(val);
                        }
                    }
                    "op20" => {
                        let parts: Vec<&str> = value.split(':').collect();
                        if parts.len() == 3 {
                            let idx: u8 = parts[0].parse().unwrap_or(0);
                            let a: u16 = parts[1].parse().unwrap_or(0);
                            let b: u16 = parts[2].parse().unwrap_or(0);
                            client.p1(20);
                            client.p1(idx);
                            client.p2(a);
                            client.p2(b);
                        }
                    }
                    _ => {}
                }
            }

            // Flush any remaining batch buffers at end of section
            flush_frames(&mut client, &mut frames);
            flush_iframes(&mut client, &mut iframes);
            flush_synths(&mut client, &mut synths);
        }

        server.finish_entry();
        client.finish_entry();
    }

    server.save(server_dir, "seq")?;
    config_blob.add_packed_data("seq", client.dat, client.idx);
    Ok(())
}

// ─── Batch key predicates ───────────────────────────────────────────────

fn is_frame_key(k: &str) -> bool {
    k.starts_with("frame") && k["frame".len()..].bytes().all(|b| b.is_ascii_digit())
}

fn is_iframe_key(k: &str) -> bool {
    k.starts_with("iframe") && k["iframe".len()..].bytes().all(|b| b.is_ascii_digit())
}

fn is_synth_key(k: &str) -> bool {
    k.starts_with("synth") && k["synth".len()..].bytes().all(|b| b.is_ascii_digit())
}

// ─── Flush helpers — emit buffered batch as one opcode, then clear ──────

fn flush_frames(out: &mut PackedData, buf: &mut Vec<(usize, u16, u16, u16)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _, _, _)| *idx);
    out.p1(1);
    out.p2(buf.len() as u16);
    for &(_, dur, _, _) in buf.iter() {
        out.p2(dur);
    }
    for &(_, _, lo, _) in buf.iter() {
        out.p2(lo);
    }
    for &(_, _, _, hi) in buf.iter() {
        out.p2(hi);
    }
    buf.clear();
}

fn flush_iframes(out: &mut PackedData, buf: &mut Vec<(usize, u16, u16)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _, _)| *idx);
    out.p1(12);
    out.p1(buf.len() as u8);
    for &(_, lo, _) in buf.iter() {
        out.p2(lo);
    }
    for &(_, _, hi) in buf.iter() {
        out.p2(hi);
    }
    buf.clear();
}

fn flush_synths(out: &mut PackedData, buf: &mut Vec<(usize, String)>) {
    if buf.is_empty() {
        return;
    }
    buf.sort_by_key(|(idx, _)| *idx);
    out.p1(13);
    out.p2(buf.len() as u16);
    for (_, value) in buf.iter() {
        if value == "null" {
            out.p1(0);
        } else {
            let parts: Vec<&str> = value.split(':').collect();
            let head: u32 = parts[0].parse().unwrap_or(0);
            let tail_count = parts.len() - 1;
            out.p1((1 + tail_count) as u8);
            out.p1(((head >> 16) & 0xFF) as u8);
            out.p1(((head >> 8) & 0xFF) as u8);
            out.p1((head & 0xFF) as u8);
            for &t in &parts[1..] {
                let v: u16 = t.parse().unwrap_or(0);
                out.p2(v);
            }
        }
    }
    buf.clear();
}
