#[cfg(before_647)]
use super::ModelCategory;
use super::UnpackedPacks;
#[cfg(since_647)]
use super::emit_params;
use crate::cache::obj::ObjTypeRaw;
use std::collections::HashMap;

fn model_name(registry: &UnpackedPacks, id: u16) -> String {
    registry
        .config_name("model", id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}"))
}

fn obj_name(registry: &UnpackedPacks, id: u16) -> String {
    if let Some(name) = registry.config_name("obj", id as u32) {
        #[cfg(before_647)]
        return name.to_string();
        #[cfg(since_647)]
        if registry.config_id("obj", name) == Some(id as u32) {
            return name.to_string();
        }
    }
    format!("obj_{id}")
}

pub fn emit_obj_props(
    def: &ObjTypeRaw,
    opcode: u8,
    _data: &[u8],
    registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if opcode == 1 {
            super::model_ref(def.model, "obj", ModelCategory::Obj, registry);
        }
        for id in [
            def.manwear,
            def.manwear2,
            def.womanwear,
            def.womanwear2,
            def.manwear3,
            def.womanwear3,
            def.manhead,
            def.womanhead,
            def.manhead2,
            def.womanhead2,
        ]
        .into_iter()
        .flatten()
        {
            super::model_ref(id, "obj_wear", ModelCategory::Obj, registry);
        }
        if let Some(desc) = &def.desc {
            p.push((3, "desc".into(), desc.to_string()));
        }
        if def.code9 {
            p.push((9, "code9".into(), "yes".into()));
        }
        if let Some(seq) = def.code10 {
            p.push((10, "code10".into(), super::seq_ref(seq, registry)));
        }
    }
    if opcode == 1 {
        p.push((1, "model".into(), model_name(registry, def.model)));
    }
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }
    if opcode == 4 {
        p.push((4, "2dzoom".into(), def.zoom2d.to_string()));
    }
    if opcode == 5 {
        p.push((5, "2dxan".into(), def.xan2d.to_string()));
    }
    if opcode == 6 {
        p.push((6, "2dyan".into(), def.yan2d.to_string()));
    }
    if opcode == 7 {
        p.push((7, "2dxof".into(), def.xof2d.to_string()));
    }
    if opcode == 8 {
        p.push((8, "2dyof".into(), def.yof2d.to_string()));
    }
    if def.stackable {
        p.push((11, "stackable".into(), "yes".into()));
    }
    if opcode == 12 {
        p.push((12, "cost".into(), def.cost.to_string()));
    }
    if def.members {
        p.push((16, "members".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if let Some(multistack) = def.multistack {
        p.push((18, "multistack".into(), obj_name(registry, multistack)));
    }
    if opcode == 23 {
        p.push((23, "manwear".into(), {
            let name = model_name(registry, def.manwear.unwrap_or(0));
            #[cfg(before_647)]
            let name = format!("{name},{}", def.manweary as u8);
            name
        }));
    }
    if opcode == 24 {
        p.push((
            24,
            "manwear2".into(),
            model_name(registry, def.manwear2.unwrap_or(0)),
        ));
    }
    if opcode == 25 {
        p.push((25, "womanwear".into(), {
            let name = model_name(registry, def.womanwear.unwrap_or(0));
            #[cfg(before_647)]
            let name = format!("{name},{}", def.womanweary as u8);
            name
        }));
    }
    if opcode == 26 {
        p.push((
            26,
            "womanwear2".into(),
            model_name(registry, def.womanwear2.unwrap_or(0)),
        ));
    }
    for (slot, opt) in def.op.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt
            && opcode == 30 + slot as u8
        {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }
    for (slot, opt) in def.iop.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt
            && opcode == 35 + slot as u8
        {
            p.push((
                35 + slot as u8,
                format!("iop{}", slot + 1),
                text.to_string(),
            ));
        }
    }
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            let n = i + 1;
            #[cfg(before_647)]
            {
                p.push((
                    40,
                    format!("recol{n}s"),
                    super::recol_value(src[i], dst[i], _reverse_hsl).to_string(),
                ));
                p.push((
                    40,
                    format!("recol{n}d"),
                    super::recol_value(dst[i], src[i], _reverse_hsl).to_string(),
                ));
            }
            #[cfg(since_647)]
            {
                let [(ks, vs), (kd, vd)] = super::recol_pair(n, src[i], dst[i], _reverse_hsl);
                p.push((40, ks, vs));
                p.push((40, kd, vd));
            }
        }
    }
    #[cfg(since_647)]
    if let (Some(dst), Some(src)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}s"), registry.named("texture", src[i])));
            p.push((41, format!("retex{n}d"), registry.named("texture", dst[i])));
        }
    }
    #[cfg(since_647)]
    if let Some(palette) = &def.recol_palette {
        for (i, b) in palette.iter().enumerate() {
            p.push((42, format!("recolpalette{}", i + 1), b.to_string()));
        }
    }
    #[cfg(since_647)]
    if def.stockmarket {
        p.push((65, "stockmarket".into(), "yes".into()));
    }
    if opcode == 78 {
        p.push((
            78,
            "manwear3".into(),
            model_name(registry, def.manwear3.unwrap_or(0)),
        ));
    }
    if opcode == 79 {
        p.push((
            79,
            "womanwear3".into(),
            model_name(registry, def.womanwear3.unwrap_or(0)),
        ));
    }
    if opcode == 90 {
        p.push((
            90,
            "manhead".into(),
            model_name(registry, def.manhead.unwrap_or(0)),
        ));
    }
    if opcode == 91 {
        p.push((
            91,
            "womanhead".into(),
            model_name(registry, def.womanhead.unwrap_or(0)),
        ));
    }
    if opcode == 92 {
        p.push((
            92,
            "manhead2".into(),
            model_name(registry, def.manhead2.unwrap_or(0)),
        ));
    }
    if opcode == 93 {
        p.push((
            93,
            "womanhead2".into(),
            model_name(registry, def.womanhead2.unwrap_or(0)),
        ));
    }
    if opcode == 95 {
        p.push((95, "2dzan".into(), def.zan2d.to_string()));
    }
    if opcode == 96 {
        p.push((96, "dummyitem".into(), (def.dummyitem as u8).to_string()));
    }
    if opcode == 97 {
        p.push((
            97,
            "certlink".into(),
            obj_name(registry, def.certlink.unwrap_or(0)),
        ));
    }
    if opcode == 98 {
        p.push((
            98,
            "certtemplate".into(),
            obj_name(registry, def.certtemplate.unwrap_or(0)),
        ));
    }
    if let (Some(countobj), Some(countco)) = (&def.countobj, &def.countco) {
        for (slot, (obj, count)) in countobj.iter().zip(countco.iter()).enumerate() {
            let code = 100 + slot as u8;
            if opcode == code {
                p.push((
                    code,
                    format!("count{}", slot + 1),
                    format!("{},{}", obj_name(registry, *obj), count),
                ));
            }
        }
    }
    #[cfg(since_244)]
    if opcode == 110 {
        p.push((
            110,
            "resizex".into(),
            def.resizex.unwrap_or(128).to_string(),
        ));
    }
    #[cfg(since_244)]
    if opcode == 111 {
        p.push((
            111,
            "resizey".into(),
            def.resizey.unwrap_or(128).to_string(),
        ));
    }
    #[cfg(since_244)]
    if opcode == 112 {
        p.push((
            112,
            "resizez".into(),
            def.resizez.unwrap_or(128).to_string(),
        ));
    }
    #[cfg(since_244)]
    if opcode == 113 {
        p.push((113, "ambient".into(), def.ambient.to_string()));
    }
    #[cfg(since_244)]
    if opcode == 114 {
        p.push((114, "contrast".into(), def.contrast.to_string()));
    }
    #[cfg(since_289)]
    if opcode == 115 {
        p.push((115, "team".into(), def.team.to_string()));
    }
    #[cfg(since_647)]
    if let Some(lentlink) = def.lentlink {
        p.push((121, "lentlink".into(), obj_name(registry, lentlink)));
    }
    #[cfg(since_647)]
    if let Some(lenttemplate) = def.lenttemplate {
        p.push((122, "lenttemplate".into(), obj_name(registry, lenttemplate)));
    }
    #[cfg(since_647)]
    if opcode == 125 {
        p.push((
            125,
            "manwearoff".into(),
            format!("{},{},{}", def.manwear_xoff, def.manweary, def.manwear_zoff),
        ));
    }
    #[cfg(since_647)]
    if opcode == 126 {
        p.push((
            126,
            "womanwearoff".into(),
            format!(
                "{},{},{}",
                def.womanwear_xoff, def.womanweary, def.womanwear_zoff
            ),
        ));
    }
    #[cfg(since_647)]
    if let Some((a, b)) = def.cursor1 {
        p.push((
            127,
            "cursor1".into(),
            format!("{},{}", a + 1, registry.reference("cursor", b)),
        ));
    }
    #[cfg(since_647)]
    if let Some((a, b)) = def.cursor2 {
        p.push((
            128,
            "cursor2".into(),
            format!("{},{}", a + 1, registry.reference("cursor", b)),
        ));
    }
    #[cfg(since_647)]
    if let Some((a, b)) = def.icursor1 {
        p.push((
            129,
            "icursor1".into(),
            format!("{},{}", a + 1, registry.reference("cursor", b)),
        ));
    }
    #[cfg(since_647)]
    if let Some((a, b)) = def.icursor2 {
        p.push((
            130,
            "icursor2".into(),
            format!("{},{}", a + 1, registry.reference("cursor", b)),
        ));
    }
    #[cfg(since_647)]
    if let Some(quests) = &def.quests {
        for q in quests.iter() {
            p.push((132, "quest".into(), q.to_string()));
        }
    }
    #[cfg(since_647)]
    if opcode == 134 {
        p.push((134, "picksizeshift".into(), def.picksizeshift.to_string()));
    }
    #[cfg(since_647)]
    if let Some(boughtlink) = def.boughtlink {
        p.push((139, "boughtlink".into(), obj_name(registry, boughtlink)));
    }
    #[cfg(since_647)]
    if let Some(boughttemplate) = def.boughttemplate {
        p.push((
            140,
            "boughttemplate".into(),
            obj_name(registry, boughttemplate),
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
