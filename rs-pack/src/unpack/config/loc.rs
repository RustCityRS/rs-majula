use super::UnpackedPacks;
#[cfg(since_647)]
use super::order::emit_params;
use crate::cache::loc::LocTypeRaw;
#[cfg(since_647)]
use crate::types::loc_shape_key;
use std::collections::HashMap;

pub fn emit_loc_props(
    def: &LocTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if def.raw_opcodes.len() > 1
        && (cfg!(since_647) || super::has_repeated_opcodes(&def.opcode_order))
    {
        #[cfg(since_647)]
        let (bgsound_vorbis, randomsound_vorbis) = (def.bgsound_vorbis, def.randomsound_vorbis);
        return super::emit_repeated::<LocTypeRaw>(
            def.id,
            &def.raw_opcodes,
            registry,
            _reverse_hsl,
            emit_loc_props,
            cfg!(before_647),
            |value| {
                value.opcode_order.clear();
                value.raw_opcodes.clear();
                #[cfg(since_647)]
                {
                    value.bgsound_vorbis = bgsound_vorbis;
                    value.randomsound_vorbis = randomsound_vorbis;
                }
            },
        );
    }
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if let Some(models) = &def.models {
            let pairs = models
                .iter()
                .map(|model| (model.model, model.shape))
                .collect::<Vec<_>>();
            let mut models = Vec::new();
            super::push_loc_model_refs(def.id, &pairs, registry, &mut models);
            let code = if def.opcode_order.contains(&5) { 5 } else { 1 };
            p.extend(models.into_iter().map(|(key, value)| (code, key, value)));
        }
        if let Some(desc) = &def.desc {
            p.push((3, "desc".into(), desc.to_string()));
        }
        if let Some(value) = def.active {
            p.push((
                19,
                "active".into(),
                if value == 1 { "yes" } else { "no" }.into(),
            ));
        }
        if def.hillskew == 1 {
            p.push((21, "hillskew".into(), "yes".into()));
        }
        if let Some(value) = def.mapfunction {
            p.push((60, "mapfunction".into(), value.to_string()));
        }
        if let Some(value) = def.mapscene {
            p.push((68, "mapscene".into(), value.to_string()));
        }
        if def.opcode_order.contains(&69) {
            let value = def.forceapproach as u8;
            let dir = match value {
                14 => "north",
                13 => "east",
                11 => "south",
                7 => "west",
                _ => panic!("Unrecognized loc config forceapproach flags: {value}"),
            };
            p.push((69, "forceapproach".into(), dir.into()));
        }
        #[cfg(since_254)]
        if let Some(value) = def.raiseobject {
            p.push((
                75,
                "raiseobject".into(),
                if value == 1 { "yes" } else { "no" }.into(),
            ));
        }
        #[cfg(since_289)]
        if let Ok(var) = u16::try_from(def.transform_varbit) {
            p.push((77, "multivar".into(), super::varbit_ref(var, registry)));
            if let Some(locs) = &def.multiloc {
                for (i, &loc) in locs.iter().enumerate() {
                    if loc != -1 {
                        p.push((
                            77,
                            "multiloc".into(),
                            format!("{i},{}", super::loc_ref(loc as u16, registry)),
                        ));
                    }
                }
            }
        }
    }

    #[cfg(since_647)]
    let model_code = if def.hd_models.is_some() { 5 } else { 1 };
    #[cfg(since_647)]
    for (key, models) in [
        (
            if model_code == 5 { "modelhd" } else { "model" },
            &def.models,
        ),
        ("modelsd", &def.hd_models),
    ] {
        for group in models.as_deref().unwrap_or_default() {
            let shape = loc_shape_key(group.shape);
            for &model in &group.models {
                p.push((
                    model_code,
                    key.into(),
                    format!("{shape},{}", registry.reference("model", model)),
                ));
            }
        }
    }

    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }

    if def.opcode_order.contains(&14) || def.width != 1 {
        p.push((14, "width".into(), def.width.to_string()));
    }

    if def.opcode_order.contains(&15) || def.length != 1 {
        p.push((15, "length".into(), def.length.to_string()));
    }

    if !def.blockwalk {
        p.push((17, "blockwalk".into(), "no".into()));
    }

    if !def.blockrange {
        p.push((18, "blockrange".into(), "no".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&19) || def.active.is_some() {
        let active = match def.active.unwrap_or(0) {
            0 => "no".to_owned(),
            1 => "yes".to_owned(),
            other => other.to_string(),
        };
        p.push((19, "active".into(), active));
    }

    #[cfg(since_647)]
    if def.hillskew == 1 {
        p.push((21, "hillskew".into(), "yes".into()));
    }

    if def.sharelight {
        p.push((22, "sharelight".into(), "yes".into()));
    }

    if def.occlude {
        p.push((23, "occlude".into(), "yes".into()));
    }

    #[cfg(before_647)]
    if def.opcode_order.contains(&24) || def.anim.is_some() {
        let name = registry
            .config_name("seq", def.anim.unwrap_or(u16::MAX) as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", def.anim.unwrap_or(u16::MAX)));
        p.push((24, "anim".into(), name));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&24) || def.anim.is_some() {
        p.push((
            24,
            "anim".into(),
            registry.reference("seq", def.anim.unwrap_or(u16::MAX)),
        ));
    }

    if def.hasalpha {
        p.push((25, "hasalpha".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.interact_type == 1 {
        p.push((27, "blockwalk".into(), "yes".into()));
    }

    if def.opcode_order.contains(&28) || def.wallwidth != 16 {
        p.push((28, "wallwidth".into(), def.wallwidth.to_string()));
    }

    if def.opcode_order.contains(&29) || def.ambient != 0 {
        p.push((29, "ambient".into(), def.ambient.to_string()));
    }

    for (slot, opt) in def.op.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }

    if def.opcode_order.contains(&39) || def.contrast != 0 {
        p.push((39, "contrast".into(), def.contrast.to_string()));
    }

    #[cfg(since_647)]
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            let [(ks, vs), (kd, vd)] = super::recol_pair(i + 1, src[i], dst[i], _reverse_hsl);
            p.push((40, ks, vs));
            p.push((40, kd, vd));
        }
    }

    #[cfg(since_647)]
    if let (Some(src), Some(dst)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            p.push((
                41,
                format!("retex{}s", i + 1),
                registry.named("texture", src[i]),
            ));
            p.push((
                41,
                format!("retex{}d", i + 1),
                registry.named("texture", dst[i]),
            ));
        }
    }

    #[cfg(since_647)]
    if let Some(palette) = &def.recol_palette {
        for (i, b) in palette.iter().enumerate() {
            p.push((42, format!("recolpalette{}", i + 1), b.to_string()));
        }
    }

    if def.mirror {
        p.push((62, "mirror".into(), "yes".into()));
    }

    if !def.shadow {
        p.push((64, "shadow".into(), "no".into()));
    }

    if def.opcode_order.contains(&65) || def.resizex != 128 {
        p.push((65, "resizex".into(), def.resizex.to_string()));
    }

    if def.opcode_order.contains(&66) || def.resizey != 128 {
        p.push((66, "resizey".into(), def.resizey.to_string()));
    }

    if def.opcode_order.contains(&67) || def.resizez != 128 {
        p.push((67, "resizez".into(), def.resizez.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&69) || def.forceapproach as u8 != 0 {
        p.push((69, "forceapproach".into(), def.forceapproach.config_str()));
    }

    #[cfg(before_647)]
    for (code, key, value) in [
        (70, "offsetx", def.offsetx),
        (71, "offsety", def.offsety),
        (72, "offsetz", def.offsetz),
    ] {
        if def.opcode_order.contains(&code) || value != 0 {
            p.push((code, key.into(), (value as u16).to_string()));
        }
    }
    #[cfg(since_647)]
    for (code, key, value) in [
        (70, "offsetx", def.offsetx),
        (71, "offsety", def.offsety),
        (72, "offsetz", def.offsetz),
    ] {
        if def.opcode_order.contains(&code) || value != 0 {
            p.push((code, key.into(), value.to_string()));
        }
    }

    if def.forcedecor {
        p.push((73, "forcedecor".into(), "yes".into()));
    }

    #[cfg(since_245_2)]
    if def.breakroutefinding {
        p.push((74, "breakroutefinding".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&75) || def.raiseobject.is_some() {
        let raise = match def.raiseobject.unwrap_or(0) {
            0 => "no".to_owned(),
            1 => "yes".to_owned(),
            other => other.to_string(),
        };
        p.push((75, "raiseobject".into(), raise));
    }

    #[cfg(since_647)]
    if let Some(multiloc) = &def.multiloc {
        let has_default =
            def.opcode_order.contains(&92) || multiloc.last().copied().unwrap_or(-1) != -1;
        p.extend(super::multi_lines(
            (if has_default { 92 } else { 77 }, "multiloc"),
            "loc",
            has_default,
            (def.transform_varbit, def.transform_varp),
            multiloc,
            registry,
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&78) || def.ambient_sfx_id != 0 {
        let kind = if def.bgsound_vorbis {
            "vorbis"
        } else {
            "synth"
        };
        p.push((
            78,
            "bgsound".into(),
            format!(
                "{},{}",
                registry.reference(kind, def.ambient_sfx_id as u16),
                def.ambient_sfx_range
            ),
        ));
    }

    #[cfg(since_647)]
    if let Some(ids) = &def.sfx_random_ids {
        let kind = if def.randomsound_vorbis {
            "vorbis"
        } else {
            "synth"
        };
        let mut values = vec![
            def.sfx_random_pri.to_string(),
            def.sfx_random_sec.to_string(),
            def.ambient_sfx_range.to_string(),
        ];
        values.extend(ids.iter().map(|&id| registry.reference(kind, id)));
        p.push((79, "randomsound".into(), values.join(",")));
    }

    #[cfg(since_647)]
    if def.hillskew == 2 {
        p.push((
            81,
            "hillchange".into(),
            format!("tree_skew,{}", def.hillskew_param / 256),
        ));
    }

    #[cfg(since_647)]
    if def.hide_minimap {
        p.push((82, "istexture".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if !def.minimap_visible {
        p.push((88, "hardshadow".into(), "no".into()));
    }

    #[cfg(since_647)]
    if !def.blocks_npcs {
        p.push((89, "randomanimframe".into(), "no".into()));
    }

    #[cfg(since_647)]
    if def.unknown_90 {
        p.push((90, "unknown90".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.members_only {
        p.push((91, "members".into(), "yes".into()));
    }

    // Disambiguate by range: if param fits in 0..65535, it is the 2 byte form.
    #[cfg(since_647)]
    if def.hillskew == 3 {
        let code = if (0..=65535).contains(&def.hillskew_param) {
            93
        } else {
            162
        };
        p.push((
            code,
            "hillchange".into(),
            format!("rotate,{}", def.hillskew_param),
        ));
    }

    #[cfg(since_647)]
    if def.hillskew == 4 {
        p.push((94, "hillchange".into(), "ceiling_skew".into()));
    }

    #[cfg(since_647)]
    if def.hillskew == 5 {
        p.push((
            95,
            "hillchange".into(),
            format!("skew_to_fit,{}", def.hillskew_param & 0xffff),
        ));
    }

    #[cfg(since_647)]
    if def.unknown_97 {
        p.push((97, "msirotate".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.unknown_98 {
        p.push((98, "hasparticles".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if let Some((op, cursor)) = def.cursor_op1 {
        p.push((
            99,
            "cursor1".into(),
            format!("{},{}", op + 1, registry.reference("cursor", cursor)),
        ));
    }

    #[cfg(since_647)]
    if let Some((op, cursor)) = def.cursor_op2 {
        p.push((
            100,
            "cursor2".into(),
            format!("{},{}", op + 1, registry.reference("cursor", cursor)),
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&101) || def.mapfunction.unwrap_or(0) != 0 {
        p.push((
            101,
            "mapfunction".into(),
            def.mapfunction.unwrap_or(0).to_string(),
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&102) || def.msi_id != -1 {
        p.push((
            102,
            "msi".into(),
            registry.reference("msi", def.msi_id as u16),
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&103) {
        p.push((103, "occlude".into(), "no".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&104) || def.interaction_side_flags != 255 {
        p.push((
            104,
            "bgsoundvolume".into(),
            def.interaction_side_flags.to_string(),
        ));
    }

    #[cfg(since_647)]
    if def.unknown_105 {
        p.push((105, "unknown105".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if let (Some(ids), Some(weights)) = (&def.random_anim_ids, &def.random_anim_weights) {
        for (&id, weight) in ids.iter().zip(weights.iter()) {
            p.push((
                106,
                "anim".into(),
                format!("{},{weight}", registry.reference("seq", id)),
            ));
        }
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&107) || def.map_element != -1 {
        p.push((
            107,
            "mapelement".into(),
            registry.reference("mel", def.map_element as u16),
        ));
    }

    #[cfg(since_647)]
    for (slot, opt) in def.members_op.iter().enumerate() {
        if let Some(text) = opt {
            p.push((
                150 + slot as u8,
                format!("membersop{}", slot + 1),
                text.to_string(),
            ));
        }
    }

    #[cfg(since_647)]
    if let Some(quests) = &def.quest_ids {
        let quests: Vec<_> = quests
            .iter()
            .map(|&quest| registry.reference("quest", quest))
            .collect();
        p.push((160, "quest".into(), quests.join(",")));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&163) || def.op163 != [0; 4] {
        p.push((
            163,
            "tint".into(),
            format!(
                "{},{},{},{}",
                def.op163[0], def.op163[1], def.op163[2], def.op163[3]
            ),
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&164) || def.op164 != 0 {
        p.push((164, "unknown164".into(), def.op164.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&165) || def.op165 != 0 {
        p.push((165, "unknown165".into(), def.op165.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&166) || def.op166 != 0 {
        p.push((166, "unknown166".into(), def.op166.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&167) || def.op167 != 0 {
        p.push((167, "raiseobjectgrounddecor".into(), def.op167.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&170) || def.op170 != 0 {
        p.push((170, "unknown170".into(), def.op170.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&171) || def.op171 != 0 {
        p.push((171, "unknown171".into(), def.op171.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&173) || def.op173 != (0, 0) {
        p.push((
            173,
            "bgsoundrate".into(),
            format!("{},{}", def.op173.0, def.op173.1),
        ));
    }

    #[cfg(since_647)]
    if def.unknown_177 {
        p.push((177, "unknown177".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&178) || def.op178 != 0 {
        p.push((178, "bgsounddropoffrange".into(), def.op178.to_string()));
    }

    #[cfg(since_647)]
    p.extend(emit_params(
        &def.raw_opcodes,
        def.params.as_deref(),
        registry,
    ));

    #[cfg(since_647)]
    {
        super::reorder_by_opcode(&def.opcode_order, p)
    }
    #[cfg(before_647)]
    {
        let mut props = super::reorder_by_opcode(&def.opcode_order, p);
        let model_ids = def
            .models
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|m| m.model)
            .collect::<Vec<_>>();
        if def.opcode_order.contains(&40)
            && let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d)
        {
            for (i, (&s, &d)) in src.iter().zip(dst.iter()).enumerate() {
                let idx = i + 1;
                let s_rgb = _reverse_hsl.get(&s);
                let d_rgb = _reverse_hsl.get(&d);
                if s >= 100 || d >= 100 {
                    props.push((
                        format!("recol{idx}s"),
                        s_rgb.copied().unwrap_or(s).to_string(),
                    ));
                    props.push((
                        format!("recol{idx}d"),
                        d_rgb.copied().unwrap_or(d).to_string(),
                    ));
                } else if s_rgb.is_none()
                    || d_rgb.is_none()
                    || super::models_have_texture(&model_ids, s, &registry.model_textures)
                {
                    props.push((format!("retex{idx}s"), super::texture_ref(s, registry)));
                    props.push((format!("retex{idx}d"), super::texture_ref(d, registry)));
                } else {
                    props.push((format!("recol{idx}s"), s_rgb.copied().unwrap().to_string()));
                    props.push((format!("recol{idx}d"), d_rgb.copied().unwrap().to_string()));
                }
            }
        }
        props
    }
}
