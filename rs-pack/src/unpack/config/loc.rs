use super::UnpackedPacks;
use crate::cache::loc::LocTypeRaw;
use std::collections::HashMap;

pub fn emit_loc_props(
    def: &LocTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if def.raw_opcodes.len() > 1
        && (cfg!(since_647) || super::has_repeated_opcodes(&def.opcode_order))
    {
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
        if let Some(var) = def.multivar {
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
    for (prefix, models) in [
        (
            if model_code == 5 { "model_sd" } else { "model" },
            &def.models,
        ),
        ("model_hd", &def.hd_models),
    ] {
        if let Some(models) = models {
            for (index, model) in models.iter().enumerate() {
                let ids = model
                    .models
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(";");
                p.push((
                    model_code,
                    format!("{prefix}{}", index + 1),
                    format!("{},{}", model.shape as i8, ids),
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
        p.push((19, "active".into(), def.active.unwrap_or(0).to_string()));
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

    if def.opcode_order.contains(&24) || def.anim.is_some() {
        let name = registry
            .config_name("seq", def.anim.unwrap_or(u16::MAX) as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", def.anim.unwrap_or(u16::MAX)));
        p.push((24, "anim".into(), name));
    }

    if def.hasalpha {
        p.push((25, "hasalpha".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.interact_type == 1 {
        p.push((27, "interact_type1".into(), "yes".into()));
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
            p.push((40, format!("recol{}s", i + 1), src[i].to_string()));
            p.push((40, format!("recol{}d", i + 1), dst[i].to_string()));
        }
    }

    #[cfg(since_647)]
    if let (Some(src), Some(dst)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            p.push((41, format!("retex{}s", i + 1), src[i].to_string()));
            p.push((41, format!("retex{}d", i + 1), dst[i].to_string()));
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
        p.push((
            69,
            "forceapproach".into(),
            (def.forceapproach as u8).to_string(),
        ));
    }

    if def.opcode_order.contains(&70) || def.offsetx != 0 {
        let raw = def.offsetx as u16;
        p.push((70, "offsetx".into(), raw.to_string()));
    }

    if def.opcode_order.contains(&71) || def.offsety != 0 {
        let raw = def.offsety as u16;
        p.push((71, "offsety".into(), raw.to_string()));
    }

    if def.opcode_order.contains(&72) || def.offsetz != 0 {
        let raw = def.offsetz as u16;
        p.push((72, "offsetz".into(), raw.to_string()));
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
        p.push((
            75,
            "raiseobject".into(),
            def.raiseobject.unwrap_or(0).to_string(),
        ));
    }

    // so reorder_by_opcode keeps them together.
    #[cfg(since_647)]
    if let Some(multiloc) = &def.multiloc {
        let default_transform = *multiloc.last().unwrap_or(&-1);
        let entries = &multiloc[..multiloc.len().saturating_sub(1)];
        let op = if def.opcode_order.contains(&92) || default_transform != -1 {
            92
        } else {
            77
        };
        let name = |prefix: &str, id: i32| {
            if id == -1 {
                return "null".to_string();
            }
            registry
                .config_name(prefix, id as u32)
                .filter(|name| registry.config_id(prefix, name) == Some(id as u32))
                .map(str::to_string)
                .unwrap_or_else(|| format!("{prefix}_{id}"))
        };
        if def.transform_varbit != -1 {
            p.push((op, "multivar".into(), name("varbit", def.transform_varbit)));
        }
        if def.transform_varp != -1 {
            p.push((op, "multivar".into(), name("varp", def.transform_varp)));
        }
        if op == 92 {
            p.push((
                op,
                "multiloc".into(),
                format!("default,{}", name("loc", default_transform)),
            ));
        }
        for (i, t) in entries.iter().enumerate() {
            p.push((op, "multiloc".into(), format!("{i},{}", name("loc", *t))));
        }
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&78) || def.ambient_sfx_id != 0 {
        p.push((
            78,
            "ambient_sfx".into(),
            format!("{},{}", def.ambient_sfx_id, def.ambient_sfx_range),
        ));
    }

    #[cfg(since_647)]
    if let Some(ids) = &def.sfx_random_ids {
        let id_str = ids
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(";");
        p.push((
            79,
            "ambient_sfx_random".into(),
            format!(
                "{},{},{},{}",
                def.sfx_random_pri, def.sfx_random_sec, def.ambient_sfx_range, id_str
            ),
        ));
    }

    #[cfg(since_647)]
    if def.hillskew == 2 {
        p.push((
            81,
            "hillskew_t2".into(),
            (def.hillskew_param / 256).to_string(),
        ));
    }

    #[cfg(since_647)]
    if def.hide_minimap {
        p.push((82, "hidden".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if !def.minimap_visible {
        p.push((88, "nominimap_visible".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if !def.blocks_npcs {
        p.push((89, "noblocks_npcs".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.unknown_90 {
        p.push((90, "op90".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.members_only {
        p.push((91, "members_only".into(), "yes".into()));
    }

    // Disambiguate by range: if param fits in 0..65535, use t3_u16.
    #[cfg(since_647)]
    if def.hillskew == 3 {
        if def.hillskew_param >= 0 && def.hillskew_param <= 65535 {
            p.push((93, "hillskew_t3_u16".into(), def.hillskew_param.to_string()));
        } else {
            p.push((
                162,
                "hillskew_t3_i32".into(),
                def.hillskew_param.to_string(),
            ));
        }
    }

    #[cfg(since_647)]
    if def.hillskew == 4 {
        p.push((94, "hillskew_t4".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.hillskew == 5 {
        let raw = (def.hillskew_param & 0xFFFF) as u16;
        p.push((95, "hillskew_t5".into(), raw.to_string()));
    }

    #[cfg(since_647)]
    if def.unknown_97 {
        p.push((97, "op97".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.unknown_98 {
        p.push((98, "op98".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if let Some((a, b)) = def.cursor_op1 {
        p.push((99, "cursor1".into(), format!("{},{}", a, b)));
    }

    #[cfg(since_647)]
    if let Some((a, b)) = def.cursor_op2 {
        p.push((100, "cursor2".into(), format!("{},{}", a, b)));
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
        p.push((102, "msi_id".into(), def.msi_id.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&103) {
        p.push((103, "occlude_off".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&104) || def.interaction_side_flags != 255 {
        p.push((
            104,
            "interaction_sides".into(),
            def.interaction_side_flags.to_string(),
        ));
    }

    #[cfg(since_647)]
    if def.unknown_105 {
        p.push((105, "op105".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if let (Some(ids), Some(weights)) = (&def.random_anim_ids, &def.random_anim_weights) {
        for (i, (id, w)) in ids.iter().zip(weights.iter()).enumerate() {
            let name = registry
                .config_name("seq", *id as u32)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("seq_{id}"));
            p.push((
                106,
                format!("random_anim{}", i + 1),
                format!("{},{}", name, w),
            ));
        }
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&107) || def.map_element != -1 {
        p.push((107, "mapelement".into(), def.map_element.to_string()));
    }

    #[cfg(since_647)]
    for (slot, opt) in def.members_op.iter().enumerate() {
        if let Some(text) = opt {
            p.push((
                150 + slot as u8,
                format!("memberop{}", slot + 1),
                text.to_string(),
            ));
        }
    }

    #[cfg(since_647)]
    if let Some(quests) = &def.quest_ids {
        for (i, q) in quests.iter().enumerate() {
            p.push((160, format!("quest{}", i + 1), q.to_string()));
        }
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&163) || def.op163 != [0; 4] {
        p.push((
            163,
            "op163".into(),
            format!(
                "{},{},{},{}",
                def.op163[0], def.op163[1], def.op163[2], def.op163[3]
            ),
        ));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&164) || def.op164 != 0 {
        p.push((164, "op164".into(), def.op164.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&165) || def.op165 != 0 {
        p.push((165, "op165".into(), def.op165.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&166) || def.op166 != 0 {
        p.push((166, "op166".into(), def.op166.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&167) || def.op167 != 0 {
        p.push((167, "op167".into(), def.op167.to_string()));
    }

    #[cfg(since_647)]
    if def.unknown_168 {
        p.push((168, "op168".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.unknown_169 {
        p.push((169, "op169".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&170) || def.op170 != 0 {
        p.push((170, "op170".into(), def.op170.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&171) || def.op171 != 0 {
        p.push((171, "op171".into(), def.op171.to_string()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&173) || def.op173 != (0, 0) {
        p.push((
            173,
            "op173".into(),
            format!("{},{}", def.op173.0, def.op173.1),
        ));
    }

    #[cfg(since_647)]
    if def.unknown_177 {
        p.push((177, "op177".into(), "yes".into()));
    }

    #[cfg(since_647)]
    if def.opcode_order.contains(&178) || def.op178 != 0 {
        p.push((178, "op178".into(), def.op178.to_string()));
    }

    #[cfg(since_647)]
    p.extend(super::order::emit_params(
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
