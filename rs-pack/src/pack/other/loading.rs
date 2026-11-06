use crate::pack::text::{Number, TextCodec};
use anyhow::{Result, ensure};

fn alignment(c: &mut TextCodec<'_>) -> Result<()> {
    c.choice("align_x", Number::U8, &["left", "centre", "right"])?;
    c.choice("align_y", Number::U8, &["top", "centre", "bottom"])?;
    c.number("x", Number::I16)?;
    c.number("y", Number::I16)?;
    Ok(())
}

fn progress(c: &mut TextCodec<'_>) -> Result<()> {
    alignment(c)?;
    c.number("width", Number::U16)?;
    c.number("height", Number::U16)?;
    c.number("text_y", Number::I16)?;
    c.reference("font", Number::I32, "fontmetrics")?;
    c.number("text_colour", Number::U32)?;
    Ok(())
}

pub fn transcode(gid: u32, c: &mut TextCodec<'_>) -> Result<()> {
    if gid == 0 {
        let version = c.number("version", Number::U8)?;
        ensure!(version <= 3, "Unsupported loading screen version");
        let count = c.number("element_types", Number::U8)? as usize;
        c.array("element_versions", Number::U8, count)?;
        let count = c.number("sequences", Number::U8)?;
        c.number("max_sequence", Number::U8)?;
        if version > 2 {
            c.reference("default_screen", Number::I16, "screen")?;
        }
        for _ in 0..count {
            c.number("sequence", Number::U8)?;
            c.number("shuffle", Number::U8)?;
            let count = c.number("screens", Number::U16)?;
            for _ in 0..count {
                c.reference("screen", Number::U16, "screen")?;
            }
        }
        return Ok(());
    }
    c.number("duration", Number::U24)?;
    c.number("fade_duration", Number::U16)?;
    let count = c.number("elements", Number::U8)?;
    for _ in 0..count {
        let kind = c.choice(
            "element",
            Number::U8,
            &[
                "clear",
                "progress_solid",
                "progress_texture",
                "progress_frame",
                "news",
                "image",
                "rotating_image",
                "text",
                "background",
                "animated_progress",
            ],
        )?;
        match kind {
            0 => {
                c.number("colour", Number::U32)?;
            }
            1 | 2 => {
                progress(c)?;
                c.number("fill_colour", Number::U32)?;
                c.number("border_colour", Number::U32)?;
                if kind == 2 {
                    c.reference("sprite", Number::U16, "loading_sprites")?;
                }
            }
            3 | 9 => {
                progress(c)?;
                for key in ["fill", "empty", "left", "right", "top", "bottom"] {
                    c.reference(key, Number::U16, "loading_sprites")?;
                }
                if kind == 9 {
                    c.number("speed", Number::I16)?;
                }
            }
            4 => {
                c.number("item", Number::U8)?;
                alignment(c)?;
                c.number("width", Number::U16)?;
                c.number("height", Number::U16)?;
                for key in ["text_colour", "shadow_colour", "border_colour"] {
                    c.number(key, Number::U32)?;
                }
                c.number("border", Number::U8)?;
            }
            5 | 6 => {
                c.reference("sprite", Number::U16, "loading_sprites")?;
                alignment(c)?;
                if kind == 6 {
                    c.number("rotation", Number::I24)?;
                }
            }
            7 => {
                c.string("text")?;
                alignment(c)?;
                c.number("text_align_x", Number::U8)?;
                c.number("text_align_y", Number::U8)?;
                c.number("line_height", Number::U8)?;
                c.number("width", Number::U16)?;
                c.number("height", Number::U16)?;
                c.reference("font", Number::I32, "fontmetrics")?;
                c.number("colour", Number::U32)?;
                c.number("shadow", Number::U32)?;
            }
            8 => {
                c.reference("sprite", Number::U16, "loading_sprites")?;
            }
            _ => unreachable!(),
        }
    }
    Ok(())
}
