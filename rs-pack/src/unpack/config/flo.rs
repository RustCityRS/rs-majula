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
        let name = def.debugname();
        if !name.is_empty() {
            _registry.flo_names.insert(def.id, name.to_string());
        }
    }
    #[cfg(since_647)]
    {
        if def.opcode_order.contains(&1) {
            p.push((1, "colour".into(), format!("0x{:x}", def.colour)));
        }
        if def.opcode_order.contains(&2) {
            p.push((2, "unknown2".into(), def.texture_u8_raw.to_string()));
        }
        if def.opcode_order.contains(&3) {
            let material = _registry.reference("texture", def.texture_u16_raw);
            p.push((3, "material".into(), material));
        }
        if !def.occlude {
            p.push((5, "occlude".into(), "no".into()));
        }
        if let Some(colour) = def.water_color {
            p.push((7, "mapcolour".into(), format!("0x{colour:x}")));
        }
        if def.op8_flomark {
            p.push((8, "unknown8".into(), "yes".into()));
        }
        if def.opcode_order.contains(&9) {
            let v = def.op9_u16;
            p.push((9, "texturescale".into(), v.to_string()));
        }
        if def.op10_bool34_false {
            p.push((10, "unknown10".into(), "yes".into()));
        }
        for v in &def.op11_values {
            p.push((11, "priority".into(), v.to_string()));
        }
        if def.op12_bool33_true {
            p.push((12, "smoothedges".into(), "yes".into()));
        }
        if def.opcode_order.contains(&13) {
            let v = def.op13_u24;
            p.push((13, "waterfogcolour".into(), format!("0x{v:x}")));
        }
        if def.opcode_order.contains(&14) {
            let v = def.op14_u8;
            p.push((14, "waterfogscale".into(), v.to_string()));
        }
        if def.opcode_order.contains(&16) {
            let v = def.op16_u8;
            p.push((16, "waterfogoffset".into(), v.to_string()));
        }
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}
