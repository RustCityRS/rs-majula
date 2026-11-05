use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::provider::CacheType;
use crate::cache::seq::SeqTypeRaw;
use crate::pack::pack_registry::PackRegistry;

const ARCHIVE_SEQS: u8 = 20;
const STRIDE_SEQS: u32 = 128;

pub fn unpack_seqs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_SEQS).unwrap_or(0);
    if group_count == 0 {
        info!("  Archive {ARCHIVE_SEQS} (seq) absent — skipping");
        return Ok(());
    }

    let out_path = output_dir.join("all.seq");
    let mut file = std::fs::File::create(&out_path)?;

    // Collect every populated entry with its global id so we can emit sorted
    // for deterministic output.
    let mut entries: Vec<(u32, rs_dat2::Entry)> = Vec::new();
    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_SEQS, group_id)? else {
            continue;
        };
        for entry in group.entries {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE_SEQS + entry.id;
            entries.push((global_id, entry));
        }
    }
    entries.sort_by_key(|(gid, _)| *gid);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for (global_id, entry) in &entries {
        let mut def = SeqTypeRaw::new(*global_id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .seq
            .get_by_id(*global_id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{global_id}"));
        let section = if seen.contains_key(&base) {
            format!("{base}__{global_id}")
        } else {
            base
        };
        seen.insert(section.clone(), *global_id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_seq_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} seq configs to {}", out_path.display());
    Ok(())
}

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
