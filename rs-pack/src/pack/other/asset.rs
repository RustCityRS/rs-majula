use crate::archive;
use crate::pack::names::AssetNames;
use crate::pack::other::{defaults, loading, worldmap_source};
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
        if aid == archive::WORLDMAP {
            worldmap_source::pack_squares(path, body, names)
        } else {
            pack(aid, gid, body, names)
        }
    })
}

pub fn entry_kind(aid: u8, gid: u32) -> Option<&'static str> {
    match (aid, gid) {
        (archive::LOADING_SCREENS, 1) => Some("screen"),
        (archive::WORLDMAP, _) => Some("file"),
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
    if !has_sections(aid) {
        return Ok(vec![(0, encode(text)?)]);
    }
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

/// Whether an asset's file holds a section for each of its group's files.
pub fn has_sections(aid: u8) -> bool {
    aid != archive::DEFAULTS
}

pub fn extension(aid: u8) -> Option<&'static str> {
    Some(match aid {
        archive::BASES => "base",
        archive::BINARY => "huffman",
        archive::FONTMETRICS => "fontmetrics",
        archive::MATERIALS => "material",
        archive::DEFAULTS => "defaults",
        archive::SYNTH => "synth",
        archive::INSTRUMENTS => "instrument",
        archive::LOADING_SCREENS => "screen",
        archive::WORLDMAP => "fm",
        _ => return None,
    })
}

pub fn unpack(aid: u8, gid: u32, data: &[u8], names: &AssetNames) -> Result<String> {
    if aid == archive::DEFAULTS {
        return defaults::unpack(gid, data, names);
    }
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
    if aid == archive::DEFAULTS {
        return defaults::pack(gid, text, names);
    }
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
        archive::SYNTH => synth(c),
        archive::INSTRUMENTS => instrument::transcode(c),
        archive::LOADING_SCREENS => loading::transcode(gid, c),
        archive::WORLDMAP => worldmap_source::transcode(c),
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
