use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use std::io::Write;
use std::path::Path;
use tracing::info;

fn escape_csv_comma(s: &str) -> String {
    s.replace(',', "\\,")
}

pub const KNOWN_CONFIG_GROUPS: &[(u32, &str)] = &[
    (31, "light"),
    (32, "bas"),
    (33, "cursor"),
    (34, "msi"),
    (35, "quest"),
    (36, "mel"),
    (46, "hitmark"),
    (54, "varclansetting"),
];

pub fn unpack_config_group(
    store: &mut CacheStore,
    group_id: u32,
    type_name: &str,
    output_dir: &Path,
) -> Result<u32> {
    let Some(group) = store.read_group(2, group_id)? else {
        info!("  Archive 2 group {group_id} ({type_name}) absent -- skipping");
        return Ok(0);
    };

    let out_path = output_dir.join(format!("all.{type_name}"));
    let mut file = std::fs::File::create(&out_path)?;

    let mut sorted: Vec<_> = group.entries.iter().collect();
    sorted.sort_by_key(|e| e.id);

    let mut written = 0u32;

    for entry in sorted {
        if entry.data.is_empty() {
            continue;
        }

        writeln!(file, "[{type_name}_{}]", entry.id)?;

        let mut reader = Packet::from(entry.data.to_vec());
        loop {
            if reader.remaining() == 0 {
                break;
            }
            let opcode = reader.g1();
            if opcode == 0 {
                break;
            }
            let props = decode_known_opcode(&mut reader, group_id, opcode)?;
            for (key, value) in props {
                writeln!(file, "{key}={value}")?;
            }
        }

        writeln!(file)?;
        written += 1;
    }

    info!(
        "  Wrote {written} {type_name} entries to {}",
        out_path.display()
    );
    Ok(written)
}

pub fn unpack_raw_config_group(
    store: &mut CacheStore,
    group_id: u32,
    output_dir: &Path,
) -> Result<u32> {
    let type_name = format!("config{group_id}");
    unpack_config_group(store, group_id, &type_name, output_dir)
}

fn decode_known_opcode(r: &mut Packet, group_id: u32, opcode: u8) -> Result<Vec<(String, String)>> {
    let result = match group_id {
        31 => decode_light(group_id, r, opcode),
        32 => decode_bas(group_id, r, opcode),
        33 => decode_cursor(group_id, r, opcode),
        34 => decode_msi(group_id, r, opcode),
        35 => decode_quest(group_id, r, opcode),
        36 => decode_mel(group_id, r, opcode),
        46 => decode_hitmark(group_id, r, opcode),
        54 => decode_varclansetting(group_id, r, opcode),
        // Group 19: op1 = g1, op2 = flag
        19 => Ok(match opcode {
            1 => {
                let v = r.g1();
                vec![("op1_g1".into(), v.to_string())]
            }
            2 => {
                vec![("op2_flag".into(), "yes".into())]
            }
            _ => vec![emit_unknown_opcode(group_id, r, opcode)],
        }),
        _ => Ok(vec![emit_unknown_opcode(group_id, r, opcode)]),
    };
    if let Err(ref e) = result {
        tracing::error!("group {group_id} opcode {opcode}: {e}");
    }
    result
}

fn decode_light(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g1();
            vec![("op1_g1".into(), v.to_string())]
        }
        2 => {
            let v = r.g2();
            vec![("op2_g2".into(), v.to_string())]
        }
        3 => {
            let v = r.g2();
            vec![("op3_g2".into(), v.to_string())]
        }
        4 => {
            let v = r.g2s();
            vec![("op4_g2b".into(), v.to_string())]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_bas(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let idle = r.g2();
            let walk = r.g2();
            vec![("op1_g2_g2".into(), format!("{idle},{walk}"))]
        }
        2 => {
            let v = r.g2();
            vec![("op2_g2".into(), v.to_string())]
        }
        3 => {
            let v = r.g2();
            vec![("op3_g2".into(), v.to_string())]
        }
        4 => {
            let v = r.g2();
            vec![("op4_g2".into(), v.to_string())]
        }
        5 => {
            let v = r.g2();
            vec![("op5_g2".into(), v.to_string())]
        }
        6 => {
            let v = r.g2();
            vec![("op6_g2".into(), v.to_string())]
        }
        7 => {
            let v = r.g2();
            vec![("op7_g2".into(), v.to_string())]
        }
        8 => {
            let v = r.g2();
            vec![("op8_g2".into(), v.to_string())]
        }
        9 => {
            let v = r.g2();
            vec![("op9_g2".into(), v.to_string())]
        }
        26 => {
            let a = r.g1();
            let b = r.g1();
            vec![("op26_g1_g1".into(), format!("{a},{b}"))]
        }
        27 => {
            let body = r.g1();
            let mut vals = Vec::new();
            vals.push(body.to_string());
            for _ in 0..6 {
                vals.push(r.g2s().to_string());
            }
            vec![("op27_g1_6xg2b".into(), vals.join(","))]
        }
        28 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g1().to_string());
            }
            vec![("op28_counted_g1".into(), vals.join(","))]
        }
        29 => {
            let v = r.g1();
            vec![("op29_g1".into(), v.to_string())]
        }
        30 => {
            let v = r.g2();
            vec![("op30_g2".into(), v.to_string())]
        }
        31 => {
            let v = r.g1();
            vec![("op31_g1".into(), v.to_string())]
        }
        32 => {
            let v = r.g2();
            vec![("op32_g2".into(), v.to_string())]
        }
        33 => {
            let v = r.g2s();
            vec![("op33_g2b".into(), v.to_string())]
        }
        34 => {
            let v = r.g1();
            vec![("op34_g1".into(), v.to_string())]
        }
        35 => {
            let v = r.g2();
            vec![("op35_g2".into(), v.to_string())]
        }
        36 => {
            let v = r.g2s();
            vec![("op36_g2b".into(), v.to_string())]
        }
        37 => {
            let v = r.g1();
            vec![("op37_g1".into(), v.to_string())]
        }
        38 => {
            let v = r.g2();
            vec![("op38_g2".into(), v.to_string())]
        }
        39 => {
            let v = r.g2();
            vec![("op39_g2".into(), v.to_string())]
        }
        40 => {
            let v = r.g2();
            vec![("op40_g2".into(), v.to_string())]
        }
        41 => {
            let v = r.g2();
            vec![("op41_g2".into(), v.to_string())]
        }
        42 => {
            let v = r.g2();
            vec![("op42_g2".into(), v.to_string())]
        }
        43 => {
            let v = r.g2();
            vec![("op43_g2".into(), v.to_string())]
        }
        44 => {
            let v = r.g2();
            vec![("op44_g2".into(), v.to_string())]
        }
        45 => {
            let v = r.g2();
            vec![("op45_g2".into(), v.to_string())]
        }
        46 => {
            let v = r.g2();
            vec![("op46_g2".into(), v.to_string())]
        }
        47 => {
            let v = r.g2();
            vec![("op47_g2".into(), v.to_string())]
        }
        48 => {
            let v = r.g2();
            vec![("op48_g2".into(), v.to_string())]
        }
        49 => {
            let v = r.g2();
            vec![("op49_g2".into(), v.to_string())]
        }
        50 => {
            let v = r.g2();
            vec![("op50_g2".into(), v.to_string())]
        }
        51 => {
            let v = r.g2();
            vec![("op51_g2".into(), v.to_string())]
        }
        52 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g2().to_string());
                vals.push(r.g1().to_string());
            }
            vec![("op52_counted_g2_g1".into(), vals.join(","))]
        }
        53 => {
            vec![("op53_flag".into(), "yes".into())]
        }
        54 => {
            let a = r.g1();
            let b = r.g1();
            vec![("op54_g1_g1".into(), format!("{a},{b}"))]
        }
        55 => {
            let body = r.g1();
            let v = r.g2();
            vec![("op55_g1_g2".into(), format!("{body},{v}"))]
        }
        56 => {
            let body = r.g1();
            let mut vals = Vec::new();
            vals.push(body.to_string());
            for _ in 0..3 {
                vals.push(r.g2s().to_string());
            }
            vec![("op56_g1_3xg2b".into(), vals.join(","))]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_cursor(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g2();
            vec![("op1_g2".into(), v.to_string())]
        }
        2 => {
            let x = r.g1();
            let y = r.g1();
            vec![("op2_g1_g1".into(), format!("{x},{y}"))]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_msi(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g2();
            vec![("op1_g2".into(), v.to_string())]
        }
        2 => {
            let v = r.g3();
            vec![("op2_g3".into(), v.to_string())]
        }
        3 => {
            vec![("op3_flag".into(), "yes".into())]
        }
        4 => {
            // sprite = -1 (flag that sets sprite to -1)
            vec![("op4_flag".into(), "yes".into())]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_quest(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            if r.remaining() > 0 && r.data[r.pos] == 0 {
                r.pos += 1;
            }
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op1_str2".into(), v)]
        }
        2 => {
            if r.remaining() > 0 && r.data[r.pos] == 0 {
                r.pos += 1;
            }
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op2_str2".into(), v)]
        }
        3 => {
            // g1 count, count * (g2, g4, g4)
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g2().to_string());
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
            }
            vec![("op3_counted_g2_g4_g4".into(), vals.join(","))]
        }
        4 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g2().to_string());
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
            }
            vec![("op4_counted_g2_g4_g4".into(), vals.join(","))]
        }
        5 => {
            let v = r.g2();
            vec![("op5_g2".into(), v.to_string())]
        }
        6 => {
            let v = r.g1();
            vec![("op6_g1".into(), v.to_string())]
        }
        7 => {
            let v = r.g1();
            vec![("op7_g1".into(), v.to_string())]
        }
        8 => {
            // Flag only — no data bytes
            vec![("op8_flag".into(), "yes".into())]
        }
        9 => {
            let v = r.g1();
            vec![("op9_g1".into(), v.to_string())]
        }
        10 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g4s().to_string());
            }
            vec![("op10_counted_g4".into(), vals.join(","))]
        }
        12 => {
            let v = r.g4s();
            vec![("op12_g4".into(), v.to_string())]
        }
        13 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g2().to_string());
            }
            vec![("op13_counted_g2".into(), vals.join(","))]
        }
        14 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g1().to_string());
                vals.push(r.g1().to_string());
            }
            vec![("op14_counted_g1_g1".into(), vals.join(","))]
        }
        15 => {
            let v = r.g2();
            vec![("op15_g2".into(), v.to_string())]
        }
        17 => {
            let v = r.g2();
            vec![("op17_g2".into(), v.to_string())]
        }
        18 => {
            // g1 count, count * (g4, g4, g4, gjstr)
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
                vals.push(escape_csv_comma(&r.gjstr(STRING_TERMINATOR)));
            }
            vec![("op18_counted_g4_g4_g4_str".into(), vals.join(","))]
        }
        19 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
                vals.push(r.g4s().to_string());
                vals.push(escape_csv_comma(&r.gjstr(STRING_TERMINATOR)));
            }
            vec![("op19_counted_g4_g4_g4_str".into(), vals.join(","))]
        }
        249 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                let is_str = r.g1();
                let key = r.g3();
                vals.push(is_str.to_string());
                vals.push(key.to_string());
                if is_str == 1 {
                    vals.push(escape_csv_comma(&r.gjstr(STRING_TERMINATOR)));
                } else {
                    vals.push(r.g4s().to_string());
                }
            }
            vec![("op249_params".into(), vals.join(","))]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_mel(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g2();
            vec![("op1_g2".into(), v.to_string())]
        }
        2 => {
            let v = r.g2();
            vec![("op2_g2".into(), v.to_string())]
        }
        3 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op3_str".into(), v)]
        }
        4 => {
            let v = r.g3();
            vec![("op4_g3".into(), v.to_string())]
        }
        5 => {
            let v = r.g3();
            vec![("op5_g3".into(), v.to_string())]
        }
        6 => {
            let v = r.g1();
            vec![("op6_g1".into(), v.to_string())]
        }
        7 => {
            let v = r.g1();
            vec![("op7_g1".into(), v.to_string())]
        }
        8 => {
            let v = r.g1();
            vec![("op8_g1".into(), v.to_string())]
        }
        9 => {
            let a = r.g2();
            let b = r.g2();
            let c = r.g4s();
            let d = r.g4s();
            vec![("op9_g2_g2_g4_g4".into(), format!("{a},{b},{c},{d}"))]
        }
        10 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op10_str".into(), v)]
        }
        11 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op11_str".into(), v)]
        }
        12 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op12_str".into(), v)]
        }
        13 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op13_str".into(), v)]
        }
        14 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op14_str".into(), v)]
        }
        15 => {
            // Complex: g1 count, count*2 x g2b, g4, g1 count2, count2 x g4, count x g1b
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..(count as u16 * 2) {
                vals.push(r.g2s().to_string());
            }
            vals.push(r.g4s().to_string());
            let count2 = r.g1();
            vals.push(count2.to_string());
            for _ in 0..count2 {
                vals.push(r.g4s().to_string());
            }
            for _ in 0..count {
                vals.push(r.g1s().to_string());
            }
            vec![("op15_complex".into(), vals.join(","))]
        }
        16 => {
            vec![("op16_flag".into(), "yes".into())]
        }
        17 => {
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op17_str".into(), v)]
        }
        18 => {
            let v = r.g2();
            vec![("op18_g2".into(), v.to_string())]
        }
        19 => {
            let v = r.g2();
            vec![("op19_g2".into(), v.to_string())]
        }
        20 => {
            let a = r.g2();
            let b = r.g2();
            let c = r.g4s();
            let d = r.g4s();
            vec![("op20_g2_g2_g4_g4".into(), format!("{a},{b},{c},{d}"))]
        }
        21 => {
            let v = r.g4s();
            vec![("op21_g4".into(), v.to_string())]
        }
        22 => {
            let v = r.g4s();
            vec![("op22_g4".into(), v.to_string())]
        }
        23 => {
            let a = r.g1();
            let b = r.g1();
            let c = r.g1();
            vec![("op23_g1_g1_g1".into(), format!("{a},{b},{c}"))]
        }
        24 => {
            let a = r.g2s();
            let b = r.g2s();
            vec![("op24_g2b_g2b".into(), format!("{a},{b}"))]
        }
        249 => {
            let count = r.g1();
            let mut vals = Vec::new();
            vals.push(count.to_string());
            for _ in 0..count {
                let is_str = r.g1();
                let key = r.g3();
                vals.push(is_str.to_string());
                vals.push(key.to_string());
                if is_str == 1 {
                    vals.push(escape_csv_comma(&r.gjstr(STRING_TERMINATOR)));
                } else {
                    vals.push(r.g4s().to_string());
                }
            }
            vec![("op249_params".into(), vals.join(","))]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_hitmark(group_id: u32, r: &mut Packet, opcode: u8) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g2();
            vec![("op1_g2".into(), v.to_string())]
        }
        2 => {
            let v = r.g3();
            vec![("op2_g3".into(), v.to_string())]
        }
        3 => {
            let v = r.g2();
            vec![("op3_g2".into(), v.to_string())]
        }
        4 => {
            let v = r.g2();
            vec![("op4_g2".into(), v.to_string())]
        }
        5 => {
            let v = r.g2();
            vec![("op5_g2".into(), v.to_string())]
        }
        6 => {
            let v = r.g2();
            vec![("op6_g2".into(), v.to_string())]
        }
        7 => {
            let v = r.g2s();
            vec![("op7_g2b".into(), v.to_string())]
        }
        8 => {
            if r.remaining() > 0 && r.data[r.pos] == 0 {
                r.pos += 1;
            }
            let v = r.gjstr(STRING_TERMINATOR);
            vec![("op8_str2".into(), v)]
        }
        9 => {
            let v = r.g2();
            vec![("op9_g2".into(), v.to_string())]
        }
        10 => {
            let v = r.g2s();
            vec![("op10_g2b".into(), v.to_string())]
        }
        11 => {
            // Flag — sets anInt50 = 0
            vec![("op11_flag".into(), "yes".into())]
        }
        12 => {
            let v = r.g1();
            vec![("op12_g1".into(), v.to_string())]
        }
        13 => {
            let v = r.g2s();
            vec![("op13_g2b".into(), v.to_string())]
        }
        14 => {
            let v = r.g2();
            vec![("op14_g2".into(), v.to_string())]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn decode_varclansetting(
    group_id: u32,
    r: &mut Packet,
    opcode: u8,
) -> Result<Vec<(String, String)>> {
    Ok(match opcode {
        1 => {
            let v = r.g1s();
            vec![("op1_g1b".into(), v.to_string())]
        }
        2 => {
            let var = r.g2();
            let start = r.g1();
            let end = r.g1();
            vec![("op2_g2_g1_g1".into(), format!("{var},{start},{end}"))]
        }
        _ => {
            vec![emit_unknown_opcode(group_id, r, opcode)]
        }
    })
}

fn emit_unknown_opcode(group_id: u32, _r: &mut Packet, opcode: u8) -> (String, String) {
    panic!("group {group_id} unknown opcode {opcode} — add a decoder arm");
}
