use super::UnpackedPacks;
use crate::cache::flo::FloTypeRaw;
#[cfg(before_647)]
use crate::cache::provider::CacheType;
use std::collections::HashMap;

pub fn emit_flo_props(
    def: &FloTypeRaw,
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if def.opcode_order.contains(&1) {
            p.push((1, "colour".into(), format!("0x{:06X}", def.colour)));
        }
        if let Some(texture) = def.texture {
            p.push((2, "texture".into(), super::texture_ref(texture, _registry)));
        }
        if def.overlay {
            p.push((3, "overlay".into(), "yes".into()));
        }
        if !def.occlude {
            p.push((5, "occlude".into(), "no".into()));
        }
        if let Some(name) = def.debugname() {
            _registry.flo_names.insert(def.id, name.to_string());
        }
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&1) {
        p.push((1, "color".into(), def.colour.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&2) {
        p.push((2, "texture_u8".into(), def.texture_u8_raw.to_string()));
    }
    #[cfg(since_647)]
    if def.opcode_order.contains(&3) {
        p.push((3, "texture".into(), def.texture_u16_raw.to_string()));
    }
    #[cfg(since_647)]
    if !def.occlude {
        p.push((5, "op5_occlude_false".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if def.water_color >= 0 {
        p.push((
            7,
            "water_color".into(),
            (def.water_color as u32).to_string(),
        ));
    }
    #[cfg(since_647)]
    if def.op8_flomark {
        p.push((8, "op8_flomark".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if let Some(v) = def.op9_u16 {
        p.push((9, "op9_u16".into(), v.to_string()));
    }
    #[cfg(since_647)]
    if def.op10_bool34_false {
        p.push((10, "op10_bool34_false".into(), "yes".into()));
    }
    #[cfg(since_647)]
    for v in &def.op11_values {
        p.push((11, "op11_u8".into(), v.to_string()));
    }
    #[cfg(since_647)]
    if def.op12_bool33_true {
        p.push((12, "op12_bool33_true".into(), "yes".into()));
    }
    #[cfg(since_647)]
    if let Some(v) = def.op13_u24 {
        p.push((13, "op13_u24".into(), v.to_string()));
    }
    #[cfg(since_647)]
    if let Some(v) = def.op14_u8 {
        p.push((14, "op14_u8".into(), v.to_string()));
    }
    #[cfg(since_647)]
    if let Some(v) = def.op16_u8 {
        p.push((16, "op16_u8".into(), v.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
