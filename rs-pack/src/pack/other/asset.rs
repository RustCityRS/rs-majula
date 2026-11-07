use crate::archive;
use crate::pack::names::AssetNames;
use crate::pack::other::{loading, worldmap_source};
use crate::pack::sound::instrument;
use crate::pack::text::{Number, TextCodec};
use anyhow::{Context, Result, bail, ensure};
use rs_io::js5::encode_group_entries;
use std::fs;
use std::path::Path;

pub fn pack_file(
    aid: u8,
    gid: u32,
    path: &Path,
    names: &AssetNames,
) -> Result<Vec<(u32, Vec<u8>)>> {
    let text = fs::read_to_string(path).with_context(|| path.display().to_string())?;
    pack_sections(aid, gid, &text, names, |body| {
        if aid == archive::WORLDMAP_DATA {
            worldmap_source::pack_squares(path, gid, body, names)
        } else {
            pack(aid, gid, body, names)
        }
    })
}

pub fn entry_kind(aid: u8, gid: u32) -> Option<&'static str> {
    match (aid, gid) {
        (archive::QUICKCHAT, 0) => Some("quickchat_category"),
        (archive::QUICKCHAT, 1) => Some("quickchat_phrase"),
        (archive::QUICKCHAT_GLOBAL, 0) => Some("quickchat_global_category"),
        (archive::QUICKCHAT_GLOBAL, 1) => Some("quickchat_global_phrase"),
        (archive::PARTICLES, 0) => Some("emitter"),
        (archive::PARTICLES, 1) => Some("effector"),
        (archive::BILLBOARDS, _) => Some("billboard"),
        (archive::LOADING_SCREENS, 1) => Some("screen"),
        (archive::WORLDMAP_DATA, _) => Some("file"),
        _ => None,
    }
}

pub fn pack_group(aid: u8, gid: u32, text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let entries = pack_entries(aid, gid, text, names)?;
    let refs: Vec<_> = entries
        .iter()
        .map(|(id, data)| (*id, data.as_slice()))
        .collect();
    Ok(encode_group_entries(&refs))
}

pub fn pack_entries(
    aid: u8,
    gid: u32,
    text: &str,
    names: &AssetNames,
) -> Result<Vec<(u32, Vec<u8>)>> {
    pack_sections(aid, gid, text, names, |body| pack(aid, gid, body, names))
}

fn pack_sections(
    aid: u8,
    gid: u32,
    text: &str,
    names: &AssetNames,
    mut encode: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<(u32, Vec<u8>)>> {
    let mut entries = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    let mut current = None;
    for line in text.split_inclusive('\n') {
        let end = offset + line.len();
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(header) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Some(id) = current {
                entries.push((id, encode(&text[start..offset])?));
            }
            start = end;
            current = Some(if let Some(kind) = entry_kind(aid, gid) {
                names.id(kind, header)?
            } else {
                0
            });
        }
        offset = end;
    }
    if let Some(id) = current {
        entries.push((id, encode(&text[start..])?));
    }
    ensure!(!entries.is_empty(), "Asset has no sections");
    entries.sort_by_key(|(id, _)| *id);
    Ok(entries)
}

pub fn extension(aid: u8) -> Option<&'static str> {
    Some(match aid {
        archive::BASES => "base",
        archive::BINARY => "huffman",
        archive::FONTMETRICS => "fontmetrics",
        archive::MATERIALS => "material",
        archive::DEFAULTS => "defaults",
        archive::BILLBOARDS => "billboard",
        archive::PARTICLES => "particle",
        archive::QUICKCHAT | archive::QUICKCHAT_GLOBAL => "quickchat",
        archive::SYNTH => "synth",
        archive::MUSIC_SAMPLES => "instrument",
        archive::LOADING_SCREENS => "screen",
        archive::WORLDMAP_DATA => "fm",
        _ => return None,
    })
}

pub fn unpack(aid: u8, gid: u32, data: &[u8], names: &AssetNames) -> Result<String> {
    let mut codec = TextCodec::unpack(data, names);
    transcode(aid, gid, &mut codec)?;
    let text = codec.finish()?.0;
    if aid == archive::MATERIALS {
        let mut lines: Vec<_> = text.lines().collect();
        lines[2..].sort_by_key(|line| line.rsplit_once('.').map(|(name, _)| name));
        Ok(lines.join("\n") + "\n")
    } else {
        Ok(text)
    }
}

pub fn pack(aid: u8, gid: u32, text: &str, names: &AssetNames) -> Result<Vec<u8>> {
    let mut codec = TextCodec::pack(text, names)?;
    transcode(aid, gid, &mut codec)?;
    Ok(codec.finish()?.1)
}

fn transcode(aid: u8, gid: u32, c: &mut TextCodec<'_>) -> Result<()> {
    match aid {
        archive::BASES => base(c),
        archive::BINARY => {
            c.array("code_lengths", Number::U8, 256)?;
            Ok(())
        }
        archive::FONTMETRICS => font(c),
        archive::MATERIALS => material(c),
        archive::DEFAULTS => defaults(gid, c),
        archive::BILLBOARDS => billboard(c),
        archive::PARTICLES if gid == 0 => emitter(c),
        archive::PARTICLES if gid == 1 => effector(c),
        archive::QUICKCHAT | archive::QUICKCHAT_GLOBAL => quickchat(aid, gid, c),
        archive::SYNTH => synth(c),
        archive::MUSIC_SAMPLES => instrument::transcode(c),
        archive::LOADING_SCREENS => loading::transcode(gid, c),
        archive::WORLDMAP_DATA => worldmap_source::transcode(gid, c),
        _ => bail!("Unsupported text asset {aid}/{gid}"),
    }
}

fn base(c: &mut TextCodec<'_>) -> Result<()> {
    let count = c.number("bones", Number::U8)? as usize;
    c.array("transforms", Number::U8, count)?;
    c.array("active", Number::U8, count)?;
    c.array("masks", Number::U16, count)?;
    let lengths = c.array("label_counts", Number::U8, count)?;
    for (i, count) in lengths.into_iter().enumerate() {
        c.array(&format!("bone{i}"), Number::U8, count as usize)?;
    }
    Ok(())
}

fn font(c: &mut TextCodec<'_>) -> Result<()> {
    c.number("version", Number::U8)?;
    let spacing = c.number("has_spacing", Number::U8)?;
    c.array("glyph_widths", Number::U8, 256)?;
    if spacing == 1 {
        let heights = c.array("glyph_heights", Number::U8, 256)?;
        c.array("glyph_spacings", Number::U8, 256)?;
        for side in ["left", "right"] {
            for (i, &height) in heights.iter().enumerate() {
                if height != 0 {
                    c.array(&format!("{side}_bearing{i}"), Number::I8, height as usize)?;
                }
            }
        }
    } else {
        c.number("space_height", Number::U8)?;
    }
    c.array("padding", Number::U8, 2)?;
    c.number("baseline_adjust", Number::U8)?;
    c.number("ascent_adjust", Number::U8)?;
    Ok(())
}

fn material(c: &mut TextCodec<'_>) -> Result<()> {
    let count = c.number("count", Number::U16)? as usize;
    let present = c.array("present", Number::U8, count)?;
    for (name, kind) in [
        ("highdetail", Number::U8),
        ("lowmem", Number::U8),
        ("lowdetail", Number::U8),
        ("brightness", Number::I8),
        ("intensity", Number::I8),
        ("effect", Number::I8),
        ("effect_param", Number::I8),
        ("averagecolour", Number::U16),
        ("scroll_u", Number::I8),
        ("scroll_v", Number::I8),
        ("combinealpha", Number::U8),
        ("flag95", Number::U8),
        ("mipmapping", Number::I8),
        ("repeat_s", Number::U8),
        ("repeat_t", Number::U8),
        ("hdr", Number::U8),
        ("effect_param2", Number::U8),
        ("effect_param3", Number::U32),
        ("alphamode", Number::U8),
    ] {
        for (i, &flag) in present.iter().enumerate() {
            if flag == 1 {
                c.number(
                    &format!("{}.{name}", c.names.name("texture", i as u32)),
                    kind,
                )?;
            }
        }
    }
    Ok(())
}

fn defaults(gid: u32, c: &mut TextCodec<'_>) -> Result<()> {
    let mut hitmarks = 4;
    loop {
        let code = c.opcode(&[
            (0, "end"),
            (1, "positions"),
            (2, "field2"),
            (3, "count"),
            (4, "array4"),
            (5, "array5"),
        ])?;
        match (gid, code) {
            (_, 0) => break,
            (1, 1) => {
                for i in 0..6 {
                    c.reference(&format!("environment_face{i}"), Number::U16, "texture")?;
                }
            }
            (1, 4 | 5) => {
                let count = c.number("count", Number::U8)? as usize;
                for i in 0..count {
                    c.reference(
                        &format!(
                            "{}_{i}",
                            if code == 4 {
                                "male_titles"
                            } else {
                                "female_titles"
                            }
                        ),
                        Number::U16,
                        "enum",
                    )?;
                }
            }
            (3, 1) => {
                c.array("hitmark_offsets", Number::I16, hitmarks * 2)?;
            }
            (3, 2) => {
                c.number("field2", Number::U16)?;
            }
            (3, 3) => {
                hitmarks = c.number("hitmark_count", Number::U8)? as usize;
            }
            (6, 1) => {
                let count = c.number("count", Number::U8)? as usize;
                c.array("solid_flags", Number::U8, count)?;
            }
            _ => bail!("Unknown defaults {gid} opcode {code}"),
        }
    }
    Ok(())
}

fn billboard(c: &mut TextCodec<'_>) -> Result<()> {
    loop {
        match c.opcode(&[
            (0, "end"),
            (1, "material"),
            (2, "size"),
            (3, "unused"),
            (4, "blend_src"),
            (5, "blend_dst"),
            (6, "bloom_disable"),
            (7, "skip_triangle"),
        ])? {
            0 => break,
            1 => {
                c.reference("material", Number::U16, "texture")?;
            }
            2 => {
                c.array("size_minus_one", Number::U16, 2)?;
            }
            3 => {
                c.number("unused", Number::U8)?;
            }
            4 => {
                c.number("blend_src", Number::U8)?;
            }
            5 => {
                c.number("blend_dst", Number::U8)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn emitter(c: &mut TextCodec<'_>) -> Result<()> {
    let names = [
        "end",
        "angles",
        "unused2",
        "speed",
        "speed_change",
        "size",
        "colour",
        "lifetime",
        "rate",
        "effectors",
        "effectors10",
        "unused11",
        "priority12",
        "priority13",
        "max_particles",
        "material",
        "cycle",
        "material17",
        "target_colour",
        "colour_percent",
        "alpha_percent",
        "speed_percent",
        "target_speed",
        "size_percent",
        "correlated_colour",
        "effectors25",
        "lighting",
        "target_size",
        "target_size_percent",
        "unused29",
        "flag30",
        "size_range",
        "flag32",
        "flag33",
        "flag34",
    ];
    let options: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(i, &name)| (i as u8, name))
        .collect();
    loop {
        let code = c.opcode(&options)?;
        match code {
            0 => break,
            1 => {
                c.array("angles", Number::U16, 4)?;
            }
            2 | 19..=21 | 23 | 28 => {
                c.number(names[code as usize], Number::U8)?;
            }
            3 => {
                c.array("speed", Number::I32, 2)?;
            }
            4 => {
                c.number("speed_change_type", Number::U8)?;
                c.number("speed_change", Number::I8)?;
            }
            5 | 14 | 27 => {
                c.number(names[code as usize], Number::U16)?;
            }
            6 => {
                c.array("colour", Number::U32, 2)?;
            }
            7 | 8 | 31 => {
                c.array(names[code as usize], Number::U16, 2)?;
            }
            9 | 10 | 25 => {
                let count = c.number("count", Number::U8)?;
                for _ in 0..count {
                    c.reference(names[code as usize], Number::U16, "effector")?;
                }
            }
            12 | 13 => {
                c.number(names[code as usize], Number::I8)?;
            }
            15 | 17 => {
                c.reference(names[code as usize], Number::U16, "texture")?;
            }
            16 => {
                c.number("cycle_start", Number::U8)?;
                c.number("cycle_offset", Number::U16)?;
                c.number("cycle_duration", Number::U16)?;
                c.number("cycle_repeat", Number::U8)?;
            }
            18 => {
                c.number("target_colour", Number::U32)?;
            }
            22 => {
                c.number("target_speed", Number::I32)?;
            }
            29 => {
                c.number("unused29", Number::I16)?;
            }
            24 | 26 | 30 | 32..=34 => {}
            _ => bail!("Unknown emitter opcode {code}"),
        }
    }
    Ok(())
}

fn effector(c: &mut TextCodec<'_>) -> Result<()> {
    loop {
        match c.opcode(&[
            (0, "end"),
            (1, "angle"),
            (2, "unused"),
            (3, "force"),
            (4, "falloff"),
            (6, "type"),
            (8, "flag8"),
            (9, "flag9"),
            (10, "reverse"),
        ])? {
            0 => break,
            1 => {
                c.number("angle", Number::U16)?;
            }
            2 => {
                c.number("unused", Number::U8)?;
            }
            3 => {
                c.array("force", Number::I32, 3)?;
            }
            4 => {
                c.number("falloff_type", Number::U8)?;
                c.number("falloff", Number::I32)?;
            }
            6 => {
                c.number("type", Number::U8)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn quickchat(aid: u8, gid: u32, c: &mut TextCodec<'_>) -> Result<()> {
    let category = entry_kind(aid, 0).unwrap();
    let phrase = entry_kind(aid, 1).unwrap();
    loop {
        let code = c.opcode(&[
            (0, "end"),
            (1, "text"),
            (2, "responses"),
            (3, "phrases"),
            (4, "unsearchable"),
        ])?;
        match code {
            0 => break,
            1 => {
                c.string("text")?;
            }
            2 | 3 if gid == 0 => {
                let count = c.number("count", Number::U8)?;
                for _ in 0..count {
                    c.reference(
                        "target",
                        Number::U16,
                        if code == 2 { category } else { phrase },
                    )?;
                    c.number("shortcut", Number::U8)?;
                }
            }
            2 if gid == 1 => {
                let count = c.number("count", Number::U8)?;
                for _ in 0..count {
                    c.reference("response", Number::U16, phrase)?;
                }
            }
            3 if gid == 1 => {
                let count = c.number("count", Number::U8)?;
                for _ in 0..count {
                    let command = c.number("command", Number::U16)?;
                    let params = match command {
                        0 | 4 | 7..=9 | 14 => 1,
                        6 | 11 => 2,
                        1 | 2 | 10 | 12 | 13 | 15 => 0,
                        _ => bail!("Unknown quickchat command {command}"),
                    };
                    if matches!(command, 0 | 6 | 7 | 11) {
                        c.reference("enum", Number::U16, "enum")?;
                        c.array("params", Number::U16, params - 1)?;
                    } else {
                        c.array("params", Number::U16, params)?;
                    }
                }
            }
            4 => {}
            _ => bail!("Unknown quickchat group {gid}"),
        }
    }
    Ok(())
}

fn envelope(c: &mut TextCodec<'_>, prefix: &str, optional: bool) -> Result<()> {
    let form = c.number(&format!("{prefix}.form"), Number::U8)?;
    if optional && form == 0 {
        return Ok(());
    }
    c.number(&format!("{prefix}.start"), Number::I32)?;
    c.number(&format!("{prefix}.end"), Number::I32)?;
    segments(c, prefix)
}

fn segments(c: &mut TextCodec<'_>, prefix: &str) -> Result<()> {
    let count = c.number(&format!("{prefix}.segments"), Number::U8)? as usize;
    c.array(&format!("{prefix}.points"), Number::U16, count * 2)?;
    Ok(())
}

fn synth(c: &mut TextCodec<'_>) -> Result<()> {
    for tone in 0..10 {
        let prefix = format!("tone{tone}");
        let form = c.number(&format!("{prefix}.frequency.form"), Number::U8)?;
        if form == 0 {
            continue;
        }
        c.number(&format!("{prefix}.frequency.start"), Number::I32)?;
        c.number(&format!("{prefix}.frequency.end"), Number::I32)?;
        segments(c, &format!("{prefix}.frequency"))?;
        envelope(c, &format!("{prefix}.amplitude"), false)?;
        for (rate, range) in [
            ("frequency_mod_rate", "frequency_mod_range"),
            ("amplitude_mod_rate", "amplitude_mod_range"),
            ("release", "attack"),
        ] {
            let rate = format!("{prefix}.{rate}");
            let form = c.number(&format!("{rate}.form"), Number::U8)?;
            if form != 0 {
                c.number(&format!("{rate}.start"), Number::I32)?;
                c.number(&format!("{rate}.end"), Number::I32)?;
                segments(c, &rate)?;
                envelope(c, &format!("{prefix}.{range}"), false)?;
            }
        }
        for i in 0..10 {
            let volume = c.number(&format!("{prefix}.harmonic{i}.volume"), Number::Smart)?;
            if volume == 0 {
                break;
            }
            c.number(
                &format!("{prefix}.harmonic{i}.semitone"),
                Number::SignedSmart,
            )?;
            c.number(&format!("{prefix}.harmonic{i}.delay"), Number::Smart)?;
        }
        for key in ["reverb_delay", "reverb_volume"] {
            c.number(&format!("{prefix}.{key}"), Number::Smart)?;
        }
        for key in ["length", "start"] {
            c.number(&format!("{prefix}.{key}"), Number::U16)?;
        }
        let filter = c.number(&format!("{prefix}.filter.pairs"), Number::U8)?;
        if filter != 0 {
            let start = c.number(&format!("{prefix}.filter.start"), Number::U16)?;
            let end = c.number(&format!("{prefix}.filter.end"), Number::U16)?;
            let migrated = c.number(&format!("{prefix}.filter.migrated"), Number::U8)?;
            let counts = [filter >> 4, filter & 15];
            ensure!(counts.iter().all(|&n| n <= 4), "Too many filter pairs");
            for phase in 0..2 {
                for (channel, &count) in counts.iter().enumerate() {
                    for i in 0..count {
                        if phase == 0 || migrated & (1 << (channel * 4 + i as usize)) != 0 {
                            c.array(
                                &format!("{prefix}.filter{channel}.{phase}.{i}"),
                                Number::U16,
                                2,
                            )?;
                        }
                    }
                }
            }
            if migrated != 0 || start != end {
                segments(c, &format!("{prefix}.filter"))?;
            }
        }
    }
    c.number("loop_start", Number::U16)?;
    c.number("loop_end", Number::U16)?;
    Ok(())
}
