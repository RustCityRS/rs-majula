#![allow(clippy::needless_range_loop)]
use crate::archive;
use anyhow::Result;
use rs_io::js5::Js5Store;
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tracing::info;

pub fn unpack_materials(store: &mut Js5Store, content_dir: &Path) -> Result<u32> {
    let archive_dir = content_dir.join("materials").join("_unpack");
    fs::create_dir_all(&archive_dir)?;

    let Some(group) = store.read_group(archive::MATERIALS, 0)? else {
        info!("  materials: archive {} group 0 absent", archive::MATERIALS);
        return Ok(0);
    };
    let Some(entry) = group.entries.first() else {
        return Ok(0);
    };

    let data = entry.data();
    let mut pos = 0;

    let texture_count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    let mut has_material = vec![false; texture_count];
    for i in 0..texture_count {
        has_material[i] = data[pos] == 1;
        pos += 1;
    }

    macro_rules! read_pass_u8 {
        () => {{
            let mut vals = vec![0u8; texture_count];
            for i in 0..texture_count {
                if has_material[i] {
                    vals[i] = data[pos];
                    pos += 1;
                }
            }
            vals
        }};
    }

    macro_rules! read_pass_i8 {
        () => {{
            let mut vals = vec![0i8; texture_count];
            for i in 0..texture_count {
                if has_material[i] {
                    vals[i] = data[pos] as i8;
                    pos += 1;
                }
            }
            vals
        }};
    }

    let highdetail = read_pass_u8!(); // pass 1: highdetail = (val == 0)
    let is_lowmem = read_pass_u8!(); // pass 2: is_lowmem = (val == 1)
    let lowdetail = read_pass_u8!(); // pass 3: lowdetail = (val == 1)
    let brightness = read_pass_i8!(); // pass 4
    let intensity = read_pass_i8!(); // pass 5
    let effect = read_pass_i8!(); // pass 6
    let effect_param1 = read_pass_i8!(); // pass 7

    // pass 8: averageColor (u16)
    let mut average_color = vec![0u16; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            average_color[i] = u16::from_be_bytes([data[pos], data[pos + 1]]);
            pos += 2;
        }
    }

    let speed_u = read_pass_i8!(); // pass 9
    let speed_v = read_pass_i8!(); // pass 10
    let flag_94 = read_pass_u8!(); // pass 11
    let flag_95 = read_pass_u8!(); // pass 12
    let mipmapping = read_pass_i8!(); // pass 13
    let repeat_s = read_pass_u8!(); // pass 14
    let repeat_t = read_pass_u8!(); // pass 15
    let hdr = read_pass_u8!(); // pass 16
    let field_1177 = read_pass_u8!(); // pass 17

    // pass 18: anInt1173 (u32)
    let mut field_1173 = vec![0u32; texture_count];
    for i in 0..texture_count {
        if has_material[i] {
            field_1173[i] =
                u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
            pos += 4;
        }
    }

    let alpha_mode = read_pass_u8!(); // pass 19

    let out_path = archive_dir.join("all.material");
    let mut file = File::create(&out_path)?;
    let mut written = 0u32;

    for i in 0..texture_count {
        if !has_material[i] {
            continue;
        }
        writeln!(file, "[material_{}]", i)?;
        if highdetail[i] == 0 {
            writeln!(file, "highdetail=yes")?;
        }
        if is_lowmem[i] == 1 {
            writeln!(file, "is_lowmem=yes")?;
        }
        if lowdetail[i] == 1 {
            writeln!(file, "lowdetail=yes")?;
        }
        if brightness[i] != 0 {
            writeln!(file, "brightness={}", brightness[i])?;
        }
        if intensity[i] != 0 {
            writeln!(file, "intensity={}", intensity[i])?;
        }
        if effect[i] != 0 {
            writeln!(file, "effect={}", effect[i])?;
        }
        if effect_param1[i] != 0 {
            writeln!(file, "effect_param1={}", effect_param1[i])?;
        }
        if average_color[i] != 0 {
            writeln!(file, "averagecolor={}", average_color[i])?;
        }
        if speed_u[i] != 0 {
            writeln!(file, "speed_u={}", speed_u[i])?;
        }
        if speed_v[i] != 0 {
            writeln!(file, "speed_v={}", speed_v[i])?;
        }
        if flag_94[i] == 1 {
            writeln!(file, "combinealpha=yes")?;
        }
        if flag_95[i] == 1 {
            writeln!(file, "flag_95=yes")?;
        }
        if mipmapping[i] != 0 {
            writeln!(file, "mipmapping={}", mipmapping[i])?;
        }
        if repeat_s[i] == 1 {
            writeln!(file, "repeat_s=yes")?;
        }
        if repeat_t[i] == 1 {
            writeln!(file, "repeat_t=yes")?;
        }
        if hdr[i] == 1 {
            writeln!(file, "hdr=yes")?;
        }
        if field_1177[i] != 0 {
            writeln!(file, "field_1177={}", field_1177[i])?;
        }
        if field_1173[i] != 0 {
            writeln!(file, "field_1173={}", field_1173[i])?;
        }
        if alpha_mode[i] != 0 {
            writeln!(file, "alpha_mode={}", alpha_mode[i])?;
        }
        written += 1;
    }

    info!("  {} material configs", written);
    Ok(written)
}
