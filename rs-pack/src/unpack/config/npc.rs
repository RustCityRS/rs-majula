#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
use crate::cache::npc::NpcTypeRaw;
use std::collections::HashMap;

fn name_for_model(id: u16, registry: &UnpackedPacks) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc"))
}

fn name_for_head_model(id: u16, registry: &UnpackedPacks) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc_head"))
}

#[cfg(since_647)]
fn name_for_seq(id: i32, registry: &UnpackedPacks) -> String {
    if id < 0 {
        return id.to_string();
    }
    registry
        .config_name("seq", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("seq_{id}"))
}

#[cfg(since_647)]
fn name_for_varbit(id: i32, registry: &UnpackedPacks) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .config_name("varbit", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("varbit_{id}"))
}

#[cfg(since_647)]
fn name_for_varp(id: i32, registry: &UnpackedPacks) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .config_name("varp", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("varp_{id}"))
}

#[cfg(since_647)]
fn name_for_npc(id: i32, registry: &UnpackedPacks) -> String {
    if id < 0 {
        return "null".to_string();
    }
    registry
        .config_name("npc", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("npc_{id}"))
}

pub fn emit_npc_props(
    def: &NpcTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if def.raw_opcodes.len() > 1
        && (cfg!(since_647) || super::has_repeated_opcodes(&def.opcode_order))
    {
        return super::emit_repeated::<NpcTypeRaw>(
            def.id,
            &def.raw_opcodes,
            registry,
            _reverse_hsl,
            emit_npc_props,
            false,
            |value| {
                value.opcode_order.clear();
                value.raw_opcodes.clear();
            },
        );
    }
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        for &id in def.models.as_deref().unwrap_or_default() {
            super::model_ref(id, "npc", ModelCategory::Npc, registry);
        }
        for &id in def.heads.as_deref().unwrap_or_default() {
            super::model_ref(id, "npc_head", ModelCategory::Npc, registry);
        }
        if let Some(desc) = &def.desc {
            p.push((3, "desc".into(), desc.to_string()));
        }
        if let Some(anim) = def.readyanim {
            p.push((13, "readyanim".into(), super::seq_ref(anim, registry)));
        }
        if let Some(anim) = def.walkanim {
            let anim = super::seq_ref(anim, registry);
            p.push((14, "walkanim".into(), anim.clone()));
            if def.opcode_order.contains(&17) {
                let anims = [
                    anim,
                    super::seq_ref(def.walkanim_b.unwrap_or(u16::MAX), registry),
                    super::seq_ref(def.walkanim_r.unwrap_or(u16::MAX), registry),
                    super::seq_ref(def.walkanim_l.unwrap_or(u16::MAX), registry),
                ];
                p.push((17, "walkanim".into(), anims.join(",")));
            }
        }
        #[cfg(before_254)]
        if def.hasalpha {
            p.push((16, "hasalpha".into(), "yes".into()));
        }
        for (code, key, value) in [
            (90, "resizex", def.resizex),
            (91, "resizey", def.resizey),
            (92, "resizez", def.resizez),
        ] {
            if let Some(value) = value {
                p.push((code, key.into(), value.to_string()));
            }
        }
    }
    if let Some(models) = &def.models {
        for (i, id) in models.iter().enumerate() {
            p.push((1, format!("model{}", i + 1), name_for_model(*id, registry)));
        }
    }
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }
    if def.opcode_order.contains(&12) || def.size != 1 {
        p.push((12, "size".into(), def.size.to_string()));
    }
    for (slot, opt) in def.op.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            p.push((
                40,
                format!("recol{}s", i + 1),
                super::recol_value(src[i], dst[i], _reverse_hsl).to_string(),
            ));
            p.push((
                40,
                format!("recol{}d", i + 1),
                super::recol_value(dst[i], src[i], _reverse_hsl).to_string(),
            ));
        }
    }
    #[cfg(since_647)]
    if let (Some(src), Some(dst)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            p.push((
                41,
                format!("retex{}s", i + 1),
                super::recol_value(src[i], dst[i], _reverse_hsl).to_string(),
            ));
            p.push((
                41,
                format!("retex{}d", i + 1),
                super::recol_value(dst[i], src[i], _reverse_hsl).to_string(),
            ));
        }
    }
    #[cfg(since_647)]
    if let Some(palette) = &def.colour_palette {
        for (i, c) in palette.iter().enumerate() {
            p.push((42, format!("colour{}", i + 1), c.to_string()));
        }
    }
    if let Some(heads) = &def.heads {
        for (i, id) in heads.iter().enumerate() {
            p.push((
                60,
                format!("head{}", i + 1),
                name_for_head_model(*id, registry),
            ));
        }
    }
    if !def.minimap {
        p.push((93, "minimap".into(), "no".into()));
    }
    if def.opcode_order.contains(&95) || def.vislevel.is_some() {
        if def.vislevel == Some(0) {
            p.push((95, "vislevel".into(), "hide".into()));
        } else {
            p.push((95, "vislevel".into(), def.vislevel.unwrap_or(0).to_string()));
        }
    }
    if def.opcode_order.contains(&97) || def.resizeh != 128 {
        p.push((97, "resizeh".into(), def.resizeh.to_string()));
    }
    if def.opcode_order.contains(&98) || def.resizev != 128 {
        p.push((98, "resizev".into(), def.resizev.to_string()));
    }
    #[cfg(since_244)]
    if def.alwaysontop {
        p.push((99, "alwaysontop".into(), "yes".into()));
    }
    #[cfg(since_244)]
    if def.opcode_order.contains(&100) || def.ambient != 0 {
        p.push((100, "ambient".into(), def.ambient.to_string()));
    }
    #[cfg(since_244)]
    if def.opcode_order.contains(&101) || def.contrast != 0 {
        p.push((101, "contrast".into(), def.contrast.to_string()));
    }
    #[cfg(since_244)]
    if def.opcode_order.contains(&102) || def.headicon.is_some() {
        p.push((
            102,
            "headicon".into(),
            def.headicon.unwrap_or(0).to_string(),
        ));
    }
    #[cfg(since_289)]
    if def.opcode_order.contains(&103) || def.turnspeed != 32 {
        p.push((103, "turnspeed".into(), def.turnspeed.to_string()));
    }
    #[cfg(since_647)]
    if let Some(transforms) = &def.transforms {
        let op = if transforms.last().copied().unwrap_or(-1) != -1 {
            118
        } else {
            106
        };
        p.push((
            op,
            "multivar".into(),
            name_for_varbit(def.transform_varbit, registry),
        ));
        p.push((
            op,
            "multivarp".into(),
            name_for_varp(def.transform_varp, registry),
        ));
        let default_transform = *transforms.last().unwrap_or(&-1);
        let entries = &transforms[..transforms.len().saturating_sub(1)];
        if default_transform != -1 {
            p.push((
                op,
                "multidefault".into(),
                name_for_npc(default_transform, registry),
            ));
        }
        for (i, t) in entries.iter().enumerate() {
            p.push((op, format!("multi{}", i + 1), name_for_npc(*t, registry)));
        }
    }
    #[cfg(since_647)]
    if !def.clickable {
        p.push((107, "clickable".into(), "no".into()));
    }
    #[cfg(since_647)]
    if !def.slowwalk {
        p.push((109, "slowwalk".into(), "no".into()));
    }
    #[cfg(since_647)]
    if !def.animateidle {
        p.push((111, "animateidle".into(), "no".into()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&113)
        || def.primary_shadow_colour != 0
        || def.secondary_shadow_colour != 0
    {
        p.push((
            113,
            "shadowcolour".into(),
            format!(
                "{},{}",
                def.primary_shadow_colour, def.secondary_shadow_colour
            ),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&114)
        || def.primary_shadow_modifier != 0
        || def.secondary_shadow_modifier != 0
    {
        p.push((
            114,
            "shadowmod".into(),
            format!(
                "{},{}",
                def.primary_shadow_modifier, def.secondary_shadow_modifier
            ),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&119) || def.walkmask != 0 {
        p.push((119, "walkmask".into(), def.walkmask.to_string()));
    }
    #[cfg(since_647)]
    if let Some(translations) = &def.translations {
        for (i, (idx, [x, y, z])) in translations.iter().enumerate() {
            p.push((
                121,
                format!("translate{}", i + 1),
                format!("{idx},{x},{y},{z}"),
            ));
        }
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&122) || def.hitbar_sprite != -1 {
        p.push((122, "hitbar".into(), def.hitbar_sprite.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&123) || def.height != -1 {
        p.push((123, "height".into(), def.height.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&125) || def.respawn_direction != 0 {
        p.push((125, "respawndir".into(), def.respawn_direction.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&127) || def.render_emote != -1 {
        p.push((
            127,
            "readyanim".into(),
            name_for_seq(def.render_emote, registry),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&128) || def.op128 != 0 {
        p.push((128, "unknown128".into(), def.op128.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&134)
        || def.idle_sound != -1
        || def.crawl_sound != -1
        || def.walk_sound != -1
        || def.run_sound != -1
    {
        p.push((134, "idlesound".into(), def.idle_sound.to_string()));
        p.push((134, "crawlsound".into(), def.crawl_sound.to_string()));
        p.push((134, "walksound".into(), def.walk_sound.to_string()));
        p.push((134, "runsound".into(), def.run_sound.to_string()));
        p.push((134, "sounddistance".into(), def.sound_distance.to_string()));
    }
    #[cfg(since_647)]
    if def.primary_cursor != 0 || def.primary_cursor_op != 0 {
        p.push((
            135,
            "cursor1".into(),
            format!("{},{}", def.primary_cursor_op, def.primary_cursor),
        ));
    }
    #[cfg(since_647)]
    if def.secondary_cursor != 0 || def.secondary_cursor_op != 0 {
        p.push((
            136,
            "cursor2".into(),
            format!("{},{}", def.secondary_cursor_op, def.secondary_cursor),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&137) || def.attack_cursor != -1 {
        p.push((137, "attackcursor".into(), def.attack_cursor.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&138) || def.army_icon != -1 {
        p.push((138, "armyicon".into(), def.army_icon.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&139) || def.sprite_id != -1 {
        p.push((139, "spriteid".into(), def.sprite_id.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&140) || def.ambient_sound_volume != 0 {
        p.push((
            140,
            "soundvolume".into(),
            def.ambient_sound_volume.to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.visible_priority {
        p.push((141, "visiblepriority".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&142) || def.mapfunction != -1 {
        p.push((142, "mapfunction".into(), def.mapfunction.to_string()));
    }
    #[cfg(since_647)]
    if def.invisible_priority {
        p.push((143, "invisiblepriority".into(), "yes".into()));
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
    if def.opcode_order.contains(&155)
        || def.hue != 0
        || def.saturation != 0
        || def.lightness != 0
        || def.opacity != 0
    {
        p.push((
            155,
            "tint".into(),
            format!(
                "{},{},{},{}",
                def.hue, def.saturation, def.lightness, def.opacity
            ),
        ));
    }
    #[cfg(since_647)]
    if def.main_option_index != -1 {
        let op = if def.main_option_index == 1 { 158 } else { 159 };
        p.push((
            op,
            "mainoptionindex".into(),
            def.main_option_index.to_string(),
        ));
    }
    #[cfg(since_647)]
    if let Some(campaigns) = &def.campaigns {
        for (i, v) in campaigns.iter().enumerate() {
            p.push((160, format!("campaign{}", i + 1), v.to_string()));
        }
    }
    #[cfg(since_647)]
    if def.vorbis {
        p.push((162, "vorbis".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&163) || def.slayer_type != 0 {
        p.push((163, "slayertype".into(), def.slayer_type.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&164) || def.sound_rate_min != 0 || def.sound_rate_max != 0 {
        p.push((
            164,
            "soundrate".into(),
            format!("{},{}", def.sound_rate_min, def.sound_rate_max),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&165) || def.pick_size_shift != 0 {
        p.push((165, "picksizeshift".into(), def.pick_size_shift.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&168) || def.sound_range_min != 0 {
        p.push((168, "soundrangemin".into(), def.sound_range_min.to_string()));
    }
    #[cfg(since_647)]
    p.extend(super::order::emit_params(
        &def.raw_opcodes,
        def.params.as_deref(),
        registry,
    ));

    super::reorder_by_opcode(&def.opcode_order, p)
}
