use super::UnpackedPacks;
use crate::cache::flo::FloTypeRaw;
#[cfg(before_647)]
use crate::cache::provider::CacheType;
use std::collections::HashMap;

pub fn emit_flo_props(
    def: &FloTypeRaw,
    opcode: u8,
    _data: &[u8],
    _registry: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    #[cfg(before_647)]
    {
        if opcode == 1 {
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
        if opcode == 1 {
            p.push((1, "colour".into(), format!("0x{:x}", def.colour)));
        }
        if opcode == 2 {
            p.push((
                2,
                "texture".into(),
                _registry.reference("texture", def.texture_u8_raw as u16),
            ));
        }
        if opcode == 3 {
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
        if opcode == 9 {
            let v = def.texturescale;
            p.push((9, "texturescale".into(), v.to_string()));
        }
        if !def.hardshadow {
            p.push((10, "hardshadow".into(), "no".into()));
        }
        if let Some(v) = def.priority.last() {
            p.push((11, "priority".into(), v.to_string()));
        }
        if def.smoothedges {
            p.push((12, "smoothedges".into(), "yes".into()));
        }
        if opcode == 13 {
            let v = def.waterfogcolour;
            p.push((13, "waterfogcolour".into(), format!("0x{v:x}")));
        }
        if opcode == 14 {
            let v = def.waterfogscale;
            p.push((14, "waterfogscale".into(), v.to_string()));
        }
        if opcode == 16 {
            let v = def.waterfogoffset;
            p.push((16, "waterfogoffset".into(), v.to_string()));
        }
    }
    p.into_iter()
        .filter(|(code, _, _)| *code == opcode)
        .map(|(_, key, value)| (key, value))
        .collect()
}
