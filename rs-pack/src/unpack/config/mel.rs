use super::{UnpackedPacks, format_colour};
use crate::cache::mel::{MelType, ShowVar};
use std::collections::HashMap;

fn condition(var: &ShowVar, packs: &UnpackedPacks) -> String {
    let name = if let Some(varbit) = var.varbit {
        packs.reference("varpbit", varbit)
    } else {
        packs.reference("varp", var.varp.unwrap_or(u16::MAX))
    };
    format!("{name},{},{}", var.min, var.max)
}

pub fn emit_mel_props(
    def: &MelType,
    opcode: u8,
    _data: &[u8],
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if opcode == 1 {
        p.push((
            1,
            "sprite".into(),
            packs.reference("sprites", def.sprite.unwrap()),
        ));
    }
    if opcode == 2 {
        let sprite = packs.reference("sprites", def.mouseovergraphic.unwrap());
        p.push((2, "mouseovergraphic".into(), sprite));
    }
    if opcode == 3 {
        p.push((3, "text".into(), def.text.clone()));
    }
    if opcode == 4 {
        p.push((4, "textcolour".into(), format_colour(def.textcolour)));
    }
    if opcode == 5 {
        let colour = format_colour(def.textmouseovercolour.unwrap());
        p.push((5, "textmouseovercolour".into(), colour));
    }
    if opcode == 6 {
        p.push((6, "textsize".into(), def.textsize.to_string()));
    }
    if opcode == 7 {
        let show = match def.show {
            0 => "none",
            1 => "map",
            2 => "minimap",
            3 => "both",
            show => panic!("Invalid mel show value: {show}"),
        };
        p.push((7, "show".into(), show.into()));
    }
    if opcode == 8 {
        let value = match def.mapfunction {
            0 => "no",
            1 => "yes",
            value => panic!("Invalid mel mapfunction value: {value}"),
        };
        p.push((8, "mapfunction".into(), value.into()));
    }
    if let Some(var) = &def.showvar1 {
        p.push((9, "condition".into(), condition(var, packs)));
    }
    for (index, op) in def.ops.iter().enumerate() {
        if let Some(op) = op {
            p.push((10 + index as u8, format!("op{}", index + 1), op.clone()));
        }
    }
    if let Some(polygon) = &def.polygon {
        for (index, (x, y)) in polygon.points.iter().enumerate() {
            p.push((15, format!("polygonpoint{index}"), format!("{x},{y}")));
        }
        p.push((15, "polygonfill".into(), format_colour(polygon.fill)));
        if polygon.palette.len() == 1 && polygon.outline.iter().all(|slot| *slot == 0) {
            p.push((
                15,
                "polygonoutline".into(),
                format_colour(polygon.palette[0]),
            ));
        } else {
            for (index, slot) in polygon.outline.iter().enumerate() {
                let colour = format_colour(polygon.palette[*slot as usize]);
                p.push((15, format!("polygonoutline{index}"), colour));
            }
        }
    }
    if def.notlistable {
        p.push((16, "listable".into(), "no".into()));
    }
    if opcode == 17 {
        p.push((17, "opbase".into(), def.opbase.clone()));
    }
    if opcode == 18 {
        let sprite = packs.reference("sprites", def.worldmaparrow.unwrap());
        p.push((18, "worldmaparrow".into(), sprite));
    }
    if opcode == 19 {
        p.push((
            19,
            "category".into(),
            format!("category_{}", def.category.unwrap()),
        ));
    }
    if let Some(var) = &def.showvar2 {
        p.push((20, "condition2".into(), condition(var, packs)));
    }
    if opcode == 21 {
        let colour = format_colour(def.textbackgroundoutline);
        p.push((21, "textbackgroundoutline".into(), colour));
    }
    if let Some(colour) = def.textbackgroundfill.last() {
        p.push((22, "textbackgroundfill".into(), format_colour(*colour)));
    }
    if opcode == 23 {
        let (a, b, c) = def.polygonoutlinedash.unwrap();
        p.push((23, "polygonoutlinedash".into(), format!("{a},{b},{c}")));
    }
    if opcode == 24 {
        let (x, y) = def.textoffset;
        p.push((24, "textoffset".into(), format!("{x},{y}")));
    }
    if opcode == 249 {
        p.extend(super::emit_params(_data, packs));
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
