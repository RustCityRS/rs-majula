#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
#[cfg(since_647)]
use super::order::emit_params;
use crate::cache::npc::NpcTypeRaw;
use std::collections::HashMap;

#[cfg(before_647)]
fn name_for_model(id: u16, registry: &UnpackedPacks) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc"))
}

#[cfg(before_647)]
fn name_for_head_model(id: u16, registry: &UnpackedPacks) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}_npc_head"))
}

#[cfg(since_647)]
fn name_for_model(id: u16, registry: &UnpackedPacks) -> String {
    registry.reference("model", id)
}

#[cfg(since_647)]
fn name_for_head_model(id: u16, registry: &UnpackedPacks) -> String {
    registry.reference("model", id)
}

pub fn emit_npc_props(
    def: &NpcTypeRaw,
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    if def.raw_opcodes.len() > 1
        && (cfg!(since_647) || super::has_repeated_opcodes(&def.opcode_order))
    {
        #[cfg(since_647)]
        let vorbis = def.vorbis;
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
                #[cfg(since_647)]
                {
                    value.vorbis = vorbis;
                }
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
            #[cfg(before_647)]
            {
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
            #[cfg(since_647)]
            {
                let [(ks, vs), (kd, vd)] = super::recol_pair(i + 1, src[i], dst[i], _reverse_hsl);
                p.push((40, ks, vs));
                p.push((40, kd, vd));
            }
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
        if cfg!(before_647) && def.vislevel == Some(0) {
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
        let has_default =
            def.opcode_order.contains(&118) || transforms.last().copied().unwrap_or(-1) != -1;
        p.extend(super::multi_lines(
            (if has_default { 118 } else { 106 }, "multinpc"),
            "npc",
            has_default,
            (def.transform_varbit, def.transform_varp),
            transforms,
            registry,
        ));
    }
    #[cfg(since_647)]
    if !def.clickable {
        p.push((107, "active".into(), "no".into()));
    }
    #[cfg(since_647)]
    if !def.slowwalk {
        p.push((109, "walksmoothing".into(), "no".into()));
    }
    #[cfg(since_647)]
    if !def.animateidle {
        p.push((111, "spotshadow".into(), "no".into()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&113)
        || def.primary_shadow_colour != 0
        || def.secondary_shadow_colour != 0
    {
        p.push((
            113,
            "spotshadowcolour".into(),
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
            "spotshadowtrans".into(),
            format!(
                "{},{}",
                def.primary_shadow_modifier, def.secondary_shadow_modifier
            ),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&119) || def.walkmask != 0 {
        p.push((119, "unknown119".into(), def.walkmask.to_string()));
    }
    #[cfg(since_647)]
    if let Some(translations) = &def.translations {
        for (index, [x, y, z]) in translations.iter() {
            p.push((121, format!("modeloffset{index}"), format!("{x},{y},{z}")));
        }
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&122) || def.hitbar_sprite != -1 {
        p.push((122, "unknown122".into(), def.hitbar_sprite.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&123) || def.height != -1 {
        p.push((123, "overlayheight".into(), def.height.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&125) || def.respawn_direction != 0 {
        p.push((125, "respawndir".into(), def.respawn_direction.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&127) || def.render_emote != -1 {
        p.push((
            127,
            "bas".into(),
            registry.reference("bas", def.render_emote as u16),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&128) || def.op128 != 0 {
        let mode = match def.op128 {
            0 => "crawl".to_owned(),
            1 => "walk".to_owned(),
            2 => "run".to_owned(),
            other => other.to_string(),
        };
        p.push((128, "defaultmovemode".into(), mode));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&134)
        || def.idle_sound != -1
        || def.crawl_sound != -1
        || def.walk_sound != -1
        || def.run_sound != -1
    {
        let kind = if def.vorbis { "vorbis" } else { "synth" };
        let sounds = [
            def.idle_sound,
            def.crawl_sound,
            def.walk_sound,
            def.run_sound,
        ]
        .map(|sound| registry.reference(kind, sound as u16));
        p.push((
            134,
            "bgsound".into(),
            format!("{},{}", sounds.join(","), def.sound_distance),
        ));
    }
    #[cfg(since_647)]
    if def.primary_cursor != 0 || def.primary_cursor_op != 0 {
        p.push((
            135,
            "cursor1".into(),
            format!(
                "{},{}",
                def.primary_cursor_op + 1,
                registry.reference("cursor", def.primary_cursor as u16)
            ),
        ));
    }
    #[cfg(since_647)]
    if def.secondary_cursor != 0 || def.secondary_cursor_op != 0 {
        p.push((
            136,
            "cursor2".into(),
            format!(
                "{},{}",
                def.secondary_cursor_op + 1,
                registry.reference("cursor", def.secondary_cursor as u16)
            ),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&137) || def.attack_cursor != -1 {
        p.push((
            137,
            "cursorattack".into(),
            registry.reference("cursor", def.attack_cursor as u16),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&138) || def.army_icon != -1 {
        p.push((
            138,
            "covermarker".into(),
            registry.reference("sprites", def.army_icon as u16),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&139) || def.sprite_id != -1 {
        p.push((139, "unknown139".into(), def.sprite_id.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&140) || def.ambient_sound_volume != 0 {
        p.push((
            140,
            "bgsoundvolume".into(),
            def.ambient_sound_volume.to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.visible_priority {
        p.push((141, "familiar".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&142) || def.mapfunction != -1 {
        p.push((
            142,
            "mapelement".into(),
            registry.reference("mel", def.mapfunction as u16),
        ));
    }
    #[cfg(since_647)]
    if def.invisible_priority {
        p.push((143, "alwaysonbottom".into(), "yes".into()));
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
        let (op, value) = if def.main_option_index == 1 {
            (158, "yes")
        } else {
            (159, "no")
        };
        p.push((op, "reprioritiseattackop".into(), value.into()));
    }
    #[cfg(since_647)]
    if let Some(campaigns) = &def.campaigns {
        let quests: Vec<_> = campaigns
            .iter()
            .map(|&quest| registry.reference("quest", quest as u16))
            .collect();
        p.push((160, "quest".into(), quests.join(",")));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&163) || def.slayer_type != 0 {
        p.push((163, "roughbounding".into(), def.slayer_type.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&164) || def.sound_rate_min != 0 || def.sound_rate_max != 0 {
        p.push((
            164,
            "bgsoundrate".into(),
            format!("{},{}", def.sound_rate_min, def.sound_rate_max),
        ));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&165) || def.pick_size_shift != 0 {
        p.push((165, "unknown165".into(), def.pick_size_shift.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&168) || def.sound_range_min != 0 {
        p.push((
            168,
            "bgsounddropoffrange".into(),
            def.sound_range_min.to_string(),
        ));
    }
    #[cfg(since_647)]
    p.extend(emit_params(
        &def.raw_opcodes,
        def.params.as_deref(),
        registry,
    ));

    super::reorder_by_opcode(&def.opcode_order, p)
}
