#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
#[cfg(since_647)]
use super::emit_params;
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
    opcode: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
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
            if opcode == 17 {
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
    if opcode == 12 {
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
    if opcode == 95 {
        if cfg!(before_647) && def.vislevel == Some(0) {
            p.push((95, "vislevel".into(), "hide".into()));
        } else {
            p.push((95, "vislevel".into(), def.vislevel.unwrap_or(0).to_string()));
        }
    }
    if opcode == 97 {
        p.push((97, "resizeh".into(), def.resizeh.to_string()));
    }
    if opcode == 98 {
        p.push((98, "resizev".into(), def.resizev.to_string()));
    }
    #[cfg(since_244)]
    if def.alwaysontop {
        p.push((99, "alwaysontop".into(), "yes".into()));
    }
    #[cfg(since_244)]
    if opcode == 100 {
        p.push((100, "ambient".into(), def.ambient.to_string()));
    }
    #[cfg(since_244)]
    if opcode == 101 {
        p.push((101, "contrast".into(), def.contrast.to_string()));
    }
    #[cfg(since_244)]
    if opcode == 102 {
        p.push((
            102,
            "headicon".into(),
            def.headicon.unwrap_or(0).to_string(),
        ));
    }
    #[cfg(since_289)]
    if opcode == 103 {
        p.push((103, "turnspeed".into(), def.turnspeed.to_string()));
    }
    #[cfg(since_647)]
    if let Some(transforms) = &def.transforms {
        let has_default = (opcode == 118) || transforms.last().copied().unwrap_or(-1) != -1;
        p.extend(super::multi_lines(
            (if has_default { 118 } else { 106 }, "multinpc"),
            "npc",
            has_default,
            (
                def.transform_varbit.map_or(-1, i32::from),
                def.transform_varp.map_or(-1, i32::from),
            ),
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
    if opcode == 113 {
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
    if opcode == 114 {
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
    if opcode == 119 {
        p.push((119, "unknown119".into(), def.walkmask.to_string()));
    }
    #[cfg(since_647)]
    if let Some(translations) = &def.translations {
        for (index, [x, y, z]) in translations.iter() {
            p.push((121, format!("modeloffset{index}"), format!("{x},{y},{z}")));
        }
    }
    #[cfg(since_647)]
    if let Some(hitbar_sprite) = def.hitbar_sprite {
        p.push((122, "unknown122".into(), hitbar_sprite.to_string()));
    }
    #[cfg(since_647)]
    if let Some(height) = def.height {
        p.push((123, "overlayheight".into(), height.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 125 {
        p.push((125, "respawndir".into(), def.respawn_direction.to_string()));
    }
    #[cfg(since_647)]
    if let Some(render_emote) = def.render_emote {
        p.push((127, "bas".into(), registry.reference("bas", render_emote)));
    }
    #[cfg(since_647)]
    if opcode == 128 {
        let mode = match def.op128 {
            0 => "crawl".to_owned(),
            1 => "walk".to_owned(),
            2 => "run".to_owned(),
            other => other.to_string(),
        };
        p.push((128, "defaultmovemode".into(), mode));
    }
    #[cfg(since_647)]
    if opcode == 134 {
        let kind = if def.vorbis { "vorbis" } else { "synth" };
        let sounds = [
            def.idle_sound,
            def.crawl_sound,
            def.walk_sound,
            def.run_sound,
        ]
        .map(|sound| registry.reference(kind, sound.unwrap_or(u16::MAX)));
        p.push((
            134,
            "bgsound".into(),
            format!("{},{}", sounds.join(","), def.sound_distance),
        ));
    }
    #[cfg(since_647)]
    if let (Some(cursor), Some(op)) = (def.primary_cursor, def.primary_cursor_op) {
        p.push((
            135,
            "cursor1".into(),
            format!("{},{}", op + 1, registry.reference("cursor", cursor)),
        ));
    }
    #[cfg(since_647)]
    if let (Some(cursor), Some(op)) = (def.secondary_cursor, def.secondary_cursor_op) {
        p.push((
            136,
            "cursor2".into(),
            format!("{},{}", op + 1, registry.reference("cursor", cursor)),
        ));
    }
    #[cfg(since_647)]
    if let Some(attack_cursor) = def.attack_cursor {
        p.push((
            137,
            "cursorattack".into(),
            registry.reference("cursor", attack_cursor),
        ));
    }
    #[cfg(since_647)]
    if let Some(army_icon) = def.army_icon {
        p.push((
            138,
            "covermarker".into(),
            registry.reference("sprites", army_icon),
        ));
    }
    #[cfg(since_647)]
    if let Some(sprite_id) = def.sprite_id {
        p.push((139, "unknown139".into(), sprite_id.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 140 {
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
    if let Some(mapfunction) = def.mapfunction {
        p.push((
            142,
            "mapelement".into(),
            registry.reference("mel", mapfunction),
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
    if opcode == 155 {
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
    if let Some(index) = def.main_option_index {
        let (op, value) = if index == 1 {
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
    if let Some(slayer_type) = def.slayer_type {
        p.push((163, "roughbounding".into(), slayer_type.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 164 {
        p.push((
            164,
            "bgsoundrate".into(),
            format!("{},{}", def.sound_rate_min, def.sound_rate_max),
        ));
    }
    #[cfg(since_647)]
    if opcode == 165 {
        p.push((165, "picksizeshift".into(), def.pick_size_shift.to_string()));
    }
    #[cfg(since_647)]
    if opcode == 168 {
        p.push((
            168,
            "bgsounddropoffrange".into(),
            def.sound_range_min.to_string(),
        ));
    }
    #[cfg(since_647)]
    if opcode == 249 {
        p.extend(emit_params(_data, registry));
    }

    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
