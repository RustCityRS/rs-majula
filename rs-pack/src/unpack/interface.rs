#[cfg(since_647)]
use crate::archive::INTERFACES as INTERFACE_ARCHIVE;
#[cfg(since_647)]
use crate::cache::r#if::HookArg;
use crate::cache::r#if::IfTypeRaw;
#[cfg(before_647)]
use crate::pack::pack_registry::PackRegistry;
#[cfg(before_647)]
use crate::pack::util::walk;
use crate::types::IfComponentType;
#[cfg(before_647)]
use crate::types::{Font, IfButtonType, PlayerStat};
#[cfg(before_647)]
use anyhow::Result;
#[cfg(since_647)]
use anyhow::{Context, Result};
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::Container;
#[cfg(since_647)]
use serde::{Deserialize, Serialize};
#[cfg(before_647)]
use std::collections::{HashMap, HashSet};
use std::fs;
#[cfg(since_647)]
use std::path::Path;
#[cfg(before_647)]
use std::path::{Path, PathBuf};
#[cfg(since_647)]
use tracing::info;
#[cfg(before_647)]
use tracing::{debug, warn};

#[cfg(before_647)]
struct Ctx<'a> {
    if_names: HashMap<u16, String>,
    registry: &'a PackRegistry,
}

#[cfg(before_647)]
impl Ctx<'_> {
    fn iface(&self, id: u16) -> &str {
        self.if_names
            .get(&id)
            .map(String::as_str)
            .unwrap_or_else(|| panic!("No interface name for id {id}"))
    }

    fn iface_suffix(&self, id: u16) -> &str {
        let name = self.iface(id);
        name.split_once(':').map(|(_, s)| s).unwrap_or(name)
    }

    fn obj(&self, id: u16) -> &str {
        self.registry
            .obj
            .get_by_id(id)
            .unwrap_or_else(|| panic!("No obj name for id {id} referenced by interface"))
    }

    fn varp(&self, id: u16) -> &str {
        self.registry
            .varp
            .get_by_id(id)
            .unwrap_or_else(|| panic!("No varp name for id {id} referenced by interface"))
    }

    #[cfg(since_254)]
    fn varbit(&self, id: u16) -> &str {
        self.registry
            .varbit
            .get_by_id(id)
            .unwrap_or_else(|| panic!("No varbit name for id {id} referenced by interface"))
    }

    fn model(&self, id: u16) -> &str {
        self.registry
            .model
            .get_by_id(id)
            .unwrap_or_else(|| panic!("No model name for id {id} referenced by interface"))
    }

    fn seq(&self, id: u16) -> &str {
        self.registry
            .seq
            .get_by_id(id)
            .unwrap_or_else(|| panic!("No seq name for id {id} referenced by interface"))
    }
}

#[cfg(before_647)]
fn decode(buf: &mut Packet) -> (Vec<u16>, HashMap<u16, IfTypeRaw>) {
    let _count = buf.g2();
    let mut order: Vec<u16> = Vec::new();
    let mut comps: HashMap<u16, IfTypeRaw> = HashMap::new();
    let mut layer: i32 = -1;

    while buf.remaining() > 0 {
        let mut id = buf.g2();
        if id == 0xFFFF {
            layer = buf.g2() as i32;
            id = buf.g2();
        }

        order.push(id);
        let com = IfTypeRaw::decode(buf, id, layer, false);
        comps.insert(id, com);
    }

    (order, comps)
}

#[cfg(before_647)]
fn build_names(
    order: &[u16],
    comps: &HashMap<u16, IfTypeRaw>,
    registry: &PackRegistry,
) -> HashMap<u16, String> {
    let mut names = registry.interface.id_to_debugname.clone();
    let max_id = comps.keys().copied().max().unwrap_or(0);

    // Roots, in id order: every root advances the counter; only unnamed /
    // `inter_`-named roots take an `inter_<n>` name.
    let mut if_id = 0usize;
    for id in 0..=max_id {
        let Some(c) = comps.get(&id) else {
            continue;
        };
        if !c.is_root() {
            continue;
        }
        let regenerate = names.get(&id).is_none_or(|n| n.starts_with("inter_"));
        if regenerate {
            names.insert(id, format!("inter_{if_id}"));
        }
        if_id += 1;
    }

    // Children, in packing order: every child advances its root's counter;
    // unnamed / `com_`-named children take `<root>:com_<position>`.
    let mut com_count: HashMap<u16, usize> = HashMap::new();
    for &id in order {
        let c = &comps[&id];
        if c.is_root() {
            continue;
        }
        let root_id = c.root_layer as u16;
        let pos = *com_count.get(&root_id).unwrap_or(&0);
        let regenerate = match names.get(&id) {
            None => true,
            Some(n) => match n.split_once(':') {
                Some((_, suffix)) => suffix.starts_with("com_"),
                None => true, // root-style name on a child (e.g. stale id reuse)
            },
        };
        if regenerate {
            let root_name = names
                .get(&root_id)
                .unwrap_or_else(|| panic!("child {id} has unnamed root {root_id}"))
                .clone();
            names.insert(id, format!("{root_name}:com_{pos}"));
        }
        *com_count.entry(root_id).or_insert(0) += 1;
    }

    names
}

#[cfg(before_647)]
fn action_target_names(flags: u16) -> String {
    let mut targets = Vec::new();
    if flags & 0x1 != 0 {
        targets.push("obj");
    }
    if flags & 0x2 != 0 {
        targets.push("npc");
    }
    if flags & 0x4 != 0 {
        targets.push("loc");
    }
    if flags & 0x8 != 0 {
        targets.push("player");
    }
    if flags & 0x10 != 0 {
        targets.push("heldobj");
    }
    targets.join(",")
}

#[cfg(before_647)]
fn next_op(ops: &[u16], idx: &mut usize) -> u16 {
    let v = ops[*idx];
    *idx += 1;
    v
}

#[cfg(before_647)]
fn stat_name(id: u16) -> &'static str {
    PlayerStat::try_from(id as u8)
        .expect("unknown player stat")
        .config_str()
}

#[cfg(before_647)]
fn export_scripts(lines: &mut Vec<String>, com: &IfTypeRaw, ctx: &Ctx) {
    for (i, ops) in com.scripts.iter().enumerate() {
        let j = i + 1;
        if ops.len() <= 1 {
            // empty script ([0] or [])
            lines.push(format!("script{j}op1="));
            continue;
        }

        let mut k = 1;
        let mut idx = 0;
        let end = ops.len() - 1; // trailing 0 terminator
        while idx < end {
            let op = ops[idx];
            idx += 1;
            let body = match op {
                1 => format!("stat_level,{}", stat_name(next_op(ops, &mut idx))),
                2 => format!("stat_base_level,{}", stat_name(next_op(ops, &mut idx))),
                3 => format!("stat_xp,{}", stat_name(next_op(ops, &mut idx))),
                4 => {
                    let inv = next_op(ops, &mut idx);
                    let obj = next_op(ops, &mut idx);
                    format!("inv_count,{},{}", ctx.iface(inv), ctx.obj(obj))
                }
                5 => format!("pushvar,{}", ctx.varp(next_op(ops, &mut idx))),
                6 => format!("stat_xp_remaining,{}", stat_name(next_op(ops, &mut idx))),
                7 => "op7".to_string(),
                8 => "op8".to_string(),
                9 => "op9".to_string(),
                10 => {
                    let inv = next_op(ops, &mut idx);
                    let obj = next_op(ops, &mut idx);
                    format!("inv_contains,{},{}", ctx.iface(inv), ctx.obj(obj))
                }
                11 => "runenergy".to_string(),
                12 => "runweight".to_string(),
                13 => {
                    let varp = next_op(ops, &mut idx);
                    let bit = next_op(ops, &mut idx);
                    format!("testbit,{},{}", ctx.varp(varp), bit)
                }
                #[cfg(since_254)]
                14 => format!("push_varbit,{}", ctx.varbit(next_op(ops, &mut idx))),
                #[cfg(since_254)]
                15 => "subtract".to_string(),
                #[cfg(since_254)]
                16 => "divide".to_string(),
                #[cfg(since_254)]
                17 => "multiply".to_string(),
                #[cfg(since_254)]
                18 => "coordx".to_string(),
                #[cfg(since_254)]
                19 => "coordz".to_string(),
                #[cfg(since_254)]
                20 => format!("push_constant,{}", next_op(ops, &mut idx)),
                other => panic!("Unknown interface script op {other} (component {})", com.id),
            };
            lines.push(format!("script{j}op{k}={body}"));
            k += 1;
        }
    }
}

#[cfg(before_647)]
fn export_comparators(lines: &mut Vec<String>, com: &IfTypeRaw) {
    for (i, &(comparator, operand)) in com.comparators.iter().enumerate() {
        let j = i + 1;
        let name = match comparator {
            1 => "eq",
            2 => "lt",
            3 => "gt",
            4 => "neq",
            other => panic!(
                "Unknown interface comparator {other} (component {})",
                com.id
            ),
        };
        lines.push(format!("script{j}={name},{operand}"));
    }
}

#[cfg(before_647)]
fn export_component(
    lines: &mut Vec<String>,
    com: &IfTypeRaw,
    x: i16,
    y: i16,
    parent: &str,
    comps: &HashMap<u16, IfTypeRaw>,
    ctx: &Ctx,
) {
    lines.push(format!("[{}]", ctx.iface_suffix(com.id)));
    if !parent.is_empty() {
        lines.push(format!("layer={parent}"));
    }
    lines.push(format!("type={}", com.com_type.config_str()));
    lines.push(format!("x={x}"));
    lines.push(format!("y={y}"));
    if com.button_type != IfButtonType::None {
        lines.push(format!("buttontype={}", com.button_type.config_str()));
    }
    if com.client_code != 0 {
        lines.push(format!("clientcode={}", com.client_code));
    }
    if com.width != 0 {
        lines.push(format!("width={}", com.width));
    }
    if com.height != 0 {
        lines.push(format!("height={}", com.height));
    }
    #[cfg(since_244)]
    if com.trans != 0 {
        lines.push(format!("trans={}", com.trans));
    }
    if let Some(over_layer) = com.over_layer {
        lines.push(format!("overlayer={}", ctx.iface_suffix(over_layer)));
    }

    export_scripts(lines, com, ctx);
    export_comparators(lines, com);

    export_type_specific(lines, com, ctx);

    // button target / inv action
    if com.button_type == IfButtonType::Target || com.com_type == IfComponentType::Inv {
        if !com.action_verb.is_empty() {
            lines.push(format!("actionverb={}", com.action_verb));
        }
        if com.action_target != 0 {
            lines.push(format!(
                "actiontarget={}",
                action_target_names(com.action_target)
            ));
        }
        if !com.action.is_empty() {
            lines.push(format!("action={}", com.action));
        }
    }
    // normal / toggle / select / pause option
    if matches!(
        com.button_type,
        IfButtonType::Normal | IfButtonType::Toggle | IfButtonType::Select | IfButtonType::Pause
    ) && !com.option.is_empty()
    {
        lines.push(format!("option={}", com.option));
    }

    // nested layer children
    if com.com_type == IfComponentType::Layer && !com.children.is_empty() {
        let suffix = ctx.iface_suffix(com.id).to_string();
        export_children(lines, com, &suffix, false, comps, ctx);
    }
}

#[cfg(before_647)]
fn export_type_specific(lines: &mut Vec<String>, com: &IfTypeRaw, ctx: &Ctx) {
    match com.com_type {
        IfComponentType::Layer => {
            if com.scroll != 0 {
                lines.push(format!("scroll={}", com.scroll));
            }
            if com.hide {
                lines.push("hide=yes".to_string());
            }
        }
        IfComponentType::Inv => {
            if com.draggable {
                lines.push("draggable=yes".to_string());
            }
            if com.operable {
                lines.push("interactable=yes".to_string());
            }
            if com.usable {
                lines.push("usable=yes".to_string());
            }
            #[cfg(since_245_2)]
            if com.swappable {
                lines.push("swappable=yes".to_string());
            }
            if com.margin_x != 0 || com.margin_y != 0 {
                lines.push(format!("margin={},{}", com.margin_x, com.margin_y));
            }
            for (i, slot) in com.slots.iter().enumerate() {
                if let Some((sx, sy, sprite)) = slot {
                    if *sx != 0 || *sy != 0 {
                        lines.push(format!("slot{}={}:{},{}", i + 1, sprite, sx, sy));
                    } else {
                        lines.push(format!("slot{}={}", i + 1, sprite));
                    }
                }
            }
            export_options(lines, com);
        }
        IfComponentType::Rect => {
            if com.fill {
                lines.push("fill=yes".to_string());
            }
            export_colours(lines, com);
        }
        IfComponentType::Text => {
            if com.center {
                lines.push("center=yes".to_string());
            }
            lines.push(format!(
                "font={}",
                Font::try_from(com.font as u8)
                    .expect("unknown interface font")
                    .name()
            ));
            if com.shadowed {
                lines.push("shadowed=yes".to_string());
            }
            if !com.text.is_empty() {
                lines.push(format!("text={}", com.text));
            }
            if !com.active_text.is_empty() {
                lines.push(format!("activetext={}", com.active_text));
            }
            export_colours(lines, com);
        }
        IfComponentType::Graphic => {
            if !com.graphic.is_empty() {
                lines.push(format!("graphic={}", com.graphic));
            }
            if !com.active_graphic.is_empty() {
                lines.push(format!("activegraphic={}", com.active_graphic));
            }
        }
        IfComponentType::Model => {
            if com.model >= 0 {
                let model = com.model as u16;
                lines.push(format!("model={}", ctx.model(model)));
            }
            if com.active_model >= 0 {
                let model = com.active_model as u16;
                lines.push(format!("activemodel={}", ctx.model(model)));
            }
            if com.anim >= 0 {
                let anim = com.anim as u16;
                lines.push(format!("anim={}", ctx.seq(anim)));
            }
            if com.active_anim >= 0 {
                let anim = com.active_anim as u16;
                lines.push(format!("activeanim={}", ctx.seq(anim)));
            }
            if com.zoom != 0 {
                lines.push(format!("zoom={}", com.zoom));
            }
            if com.xan != 0 {
                lines.push(format!("xan={}", com.xan));
            }
            if com.yan != 0 {
                lines.push(format!("yan={}", com.yan));
            }
        }
        IfComponentType::InvText => {
            if com.center {
                lines.push("center=yes".to_string());
            }
            lines.push(format!(
                "font={}",
                Font::try_from(com.font as u8)
                    .expect("unknown interface font")
                    .name()
            ));
            if com.shadowed {
                lines.push("shadowed=yes".to_string());
            }
            if com.colour != 0 {
                lines.push(format!("colour=0x{:06X}", com.colour));
            }
            if com.margin_x != 0 || com.margin_y != 0 {
                lines.push(format!("margin={},{}", com.margin_x, com.margin_y));
            }
            if com.operable {
                lines.push("interactable=yes".to_string());
            }
            export_options(lines, com);
        }
    }
}

#[cfg(before_647)]
fn export_colours(lines: &mut Vec<String>, com: &IfTypeRaw) {
    if com.colour != 0 {
        lines.push(format!("colour=0x{:06X}", com.colour));
    }
    if com.active_colour != 0 {
        lines.push(format!("activecolour=0x{:06X}", com.active_colour));
    }
    if com.over_colour != 0 {
        lines.push(format!("overcolour=0x{:06X}", com.over_colour));
    }
    #[cfg(since_245_2)]
    if com.active_over_colour != 0 {
        lines.push(format!("activeovercolour=0x{:06X}", com.active_over_colour));
    }
}

#[cfg(before_647)]
fn export_options(lines: &mut Vec<String>, com: &IfTypeRaw) {
    for (i, iop) in com.iop.iter().enumerate() {
        if !iop.is_empty() {
            lines.push(format!("option{}={}", i + 1, iop));
        }
    }
}

#[cfg(before_647)]
fn export_children(
    lines: &mut Vec<String>,
    parent_com: &IfTypeRaw,
    parent_suffix: &str,
    is_root: bool,
    comps: &HashMap<u16, IfTypeRaw>,
    ctx: &Ctx,
) {
    for (i, &(cid, cx, cy)) in parent_com.children.iter().enumerate() {
        if !is_root || i > 0 {
            lines.push(String::new());
        }
        let Some(child) = comps.get(&cid) else {
            warn!("interface child {cid} missing from component table");
            continue;
        };
        export_component(lines, child, cx, cy, parent_suffix, comps, ctx);
    }
}

#[cfg(before_647)]
fn read_top_lines(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut top = Vec::new();
    for line in text.lines() {
        if line.starts_with('[') {
            break;
        }
        top.push(line.to_string());
    }
    while top.last().is_some_and(|l| l.trim().is_empty()) {
        top.pop();
    }
    top
}

#[cfg(before_647)]
pub fn unpack_interface(
    jag: &JagFile,
    src_scripts_dir: &Path,
    out_scripts_dir: &Path,
    out_pack_dir: &Path,
    registry: &PackRegistry,
) -> Result<()> {
    let Some(mut buf) = jag.read("data") else {
        warn!("interface archive has no 'data' file; skipping");
        return Ok(());
    };

    let (order, comps) = decode(&mut buf);
    let if_names = build_names(&order, &comps, registry);
    let ctx = Ctx { if_names, registry };

    // interface.order (packing order) and interface.pack (id=name).
    fs::create_dir_all(out_pack_dir)?;
    let order_text = order
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(out_pack_dir.join("interface.order"), order_text + "\n")?;

    let max_id = ctx.if_names.keys().copied().max().unwrap_or(0);
    let mut pack_lines = Vec::with_capacity(ctx.if_names.len());
    for id in 0..=max_id {
        if let Some(name) = ctx.if_names.get(&id) {
            pack_lines.push(format!("{id}={name}"));
        }
    }
    fs::write(
        out_pack_dir.join("interface.pack"),
        pack_lines.join("\n") + "\n",
    )?;

    // Map existing root debugname -> its path relative to the source scripts
    // dir, and collect overlay roots (read-only inputs).
    let mut name_to_rel: HashMap<String, PathBuf> = HashMap::new();
    let mut overlay_roots: HashSet<String> = HashSet::new();
    for path in walk(src_scripts_dir) {
        if path.extension().and_then(|e| e.to_str()) != Some("if") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if read_top_lines(&path).iter().any(|l| l == "type=overlay") {
            overlay_roots.insert(stem.to_string());
        }
        if let Ok(rel) = path.strip_prefix(src_scripts_dir) {
            name_to_rel.insert(stem.to_string(), rel.to_path_buf());
        }
    }

    let mut written = 0usize;
    for com in comps.values() {
        if !com.is_root() {
            continue;
        }
        let root_name = ctx.iface(com.id).to_string();

        let mut header: Vec<String> = Vec::new();
        if overlay_roots.contains(&root_name) {
            header.push("type=overlay".to_string());
        }
        if com.scroll != 0 {
            header.push(format!("scroll={}", com.scroll));
        }
        if com.hide {
            header.push("hide=yes".to_string());
        }

        let mut body: Vec<String> = Vec::new();
        export_children(&mut body, com, "", true, &comps, &ctx);

        let mut out = String::new();
        if !header.is_empty() {
            out.push_str(&header.join("\n"));
            out.push_str("\n\n");
        }
        out.push_str(&body.join("\n"));
        out.push('\n');

        let rel = name_to_rel
            .get(&root_name)
            .cloned()
            .unwrap_or_else(|| Path::new("interfaces").join(format!("{root_name}.if")));
        let dest = out_scripts_dir.join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&dest, out)?;
        written += 1;
    }

    debug!(
        "  Regenerated interface: {} components, {written} .if files into {}",
        comps.len(),
        out_scripts_dir.display()
    );
    Ok(())
}

#[cfg(since_647)]
pub fn unpack_interfaces(store: &mut Js5Store, content_dir: &Path) -> Result<u32> {
    let if_dir = content_dir.join("interfaces").join("_unpack");
    fs::create_dir_all(&if_dir)?;

    let group_count = store.group_count(INTERFACE_ARCHIVE).unwrap_or(0);
    let mut manifest = InterfaceManifest {
        archive_id: INTERFACE_ARCHIVE,
        groups: Vec::new(),
    };
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(raw) = store.read_raw_group_bytes(INTERFACE_ARCHIVE, group_id)? else {
            continue;
        };
        if raw.len() < 5 {
            continue;
        }

        let container = Container::parse(&raw)?;

        let Some(group) = store
            .read_group(INTERFACE_ARCHIVE, group_id)
            .with_context(|| format!("read interface group {}", group_id))?
        else {
            continue;
        };

        let mut text = String::new();
        let mut comp_count = 0u32;

        for entry in &group.entries {
            if entry.data().is_empty() {
                continue;
            }
            let comp = IfTypeRaw::decode(
                &mut Packet::from(entry.data().to_vec()),
                entry.id as u16,
                group_id as i32,
            );
            text.push_str(&emit_component_text(&comp, entry.id));
            text.push('\n');
            comp_count += 1;
        }

        let file_name = format!("{}.if", group_id);
        fs::write(if_dir.join(&file_name), &text)?;

        manifest.groups.push(InterfaceGroupEntry {
            group_id,
            file_name,
            compression: container.compression as u8,
            version: container.version.unwrap_or(0),
            compressed_len: container.compressed_len,
            decompressed_len: container.uncompressed_len,
            crc: container.crc(),
            component_count: comp_count,
        });
        written += 1;
    }

    let manifest_path = if_dir.join("manifest.json");
    fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

    info!(
        "Interfaces unpacked: {} groups, {} total components",
        written,
        manifest
            .groups
            .iter()
            .map(|g| g.component_count)
            .sum::<u32>()
    );

    Ok(written)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(since_647)]
pub struct InterfaceManifest {
    pub archive_id: u8,
    pub groups: Vec<InterfaceGroupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(since_647)]
pub struct InterfaceGroupEntry {
    pub group_id: u32,
    pub file_name: String,
    pub compression: u8,
    pub version: u16,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub crc: u32,
    pub component_count: u32,
}

#[cfg(since_647)]
pub fn emit_component_text(comp: &IfTypeRaw, comp_index: u32) -> String {
    let mut out = String::new();
    let type_name = match comp.com_type {
        IfComponentType::Layer => "layer",
        IfComponentType::Rect => "rect",
        IfComponentType::Text => "text",
        IfComponentType::Graphic => "graphic",
        IfComponentType::Model => "model",
        IfComponentType::Line => "line",
        t => {
            out.push_str(&format!("[com_{}]\ntype={}\n", comp_index, t as u8));
            return out;
        }
    };

    out.push_str(&format!("[com_{}]\n", comp_index));
    out.push_str(&format!("type={}\n", type_name));
    if let Some(ref name) = comp.com_name {
        out.push_str(&format!("name={}\n", name));
    }
    if comp.client_code != 0 {
        out.push_str(&format!("client_code={}\n", comp.client_code));
    }
    if comp.x != 0 {
        out.push_str(&format!("x={}\n", comp.x));
    }
    if comp.y != 0 {
        out.push_str(&format!("y={}\n", comp.y));
    }
    if comp.width != 0 {
        out.push_str(&format!("width={}\n", comp.width));
    }
    if comp.height != 0 {
        out.push_str(&format!("height={}\n", comp.height));
    }
    if comp.width_mode != 0 {
        out.push_str(&format!("width_mode={}\n", comp.width_mode));
    }
    if comp.height_mode != 0 {
        out.push_str(&format!("height_mode={}\n", comp.height_mode));
    }
    if comp.x_mode != 0 {
        out.push_str(&format!("x_mode={}\n", comp.x_mode));
    }
    if comp.y_mode != 0 {
        out.push_str(&format!("y_mode={}\n", comp.y_mode));
    }
    if comp.parent_id != 65535 {
        out.push_str(&format!("parent_id={}\n", comp.parent_id));
    }
    if comp.flags != 0 {
        out.push_str(&format!("flags={}\n", comp.flags));
    }

    match comp.com_type {
        IfComponentType::Layer => {
            if comp.scroll_width != 0 {
                out.push_str(&format!("scroll_width={}\n", comp.scroll_width));
            }
            if comp.scroll != 0 {
                out.push_str(&format!("scroll_height={}\n", comp.scroll));
            }
            if comp.no_click_through {
                out.push_str("no_click_through=yes\n");
            }
        }
        IfComponentType::Graphic => {
            out.push_str(&format!("graphic={}\n", comp.graphic));
            if comp.angle_2d != 0 {
                out.push_str(&format!("angle_2d={}\n", comp.angle_2d));
            }
            if comp.tiling {
                out.push_str("tiling=yes\n");
            }
            if comp.alpha {
                out.push_str("alpha=yes\n");
            }
            if comp.trans != 0 {
                out.push_str(&format!("trans={}\n", comp.trans));
            }
            if comp.outline != 0 {
                out.push_str(&format!("outline={}\n", comp.outline));
            }
            if comp.graphic_shadow != 0 {
                out.push_str(&format!("graphic_shadow={}\n", comp.graphic_shadow));
            }
            if comp.vflip {
                out.push_str("vflip=yes\n");
            }
            if comp.hflip {
                out.push_str("hflip=yes\n");
            }
            if comp.colour != 0 {
                out.push_str(&format!("color={}\n", comp.colour));
            }
        }
        IfComponentType::Model => {
            out.push_str(&format!("model={}\n", comp.model));
            if comp.model_flags != 0 {
                out.push_str(&format!("model_flags={}\n", comp.model_flags));
            }
            if comp.model_origin_x != 0 {
                out.push_str(&format!("model_origin_x={}\n", comp.model_origin_x));
            }
            if comp.model_origin_y != 0 {
                out.push_str(&format!("model_origin_y={}\n", comp.model_origin_y));
            }
            if comp.xan != 0 {
                out.push_str(&format!("model_xan={}\n", comp.xan));
            }
            if comp.yan != 0 {
                out.push_str(&format!("model_yan={}\n", comp.yan));
            }
            if comp.model_zan != 0 {
                out.push_str(&format!("model_zan={}\n", comp.model_zan));
            }
            if comp.zoom != 0 {
                out.push_str(&format!("model_zoom={}\n", comp.zoom));
            }
            if comp.sprite_y_offset != 0 {
                out.push_str(&format!("sprite_y_offset={}\n", comp.sprite_y_offset));
            }
            if comp.anim != 0 {
                out.push_str(&format!("animation={}\n", comp.anim));
            }
            if comp.dynamic_width != 0 {
                out.push_str(&format!("dynamic_width={}\n", comp.dynamic_width));
            }
            if comp.dynamic_height != 0 {
                out.push_str(&format!("dynamic_height={}\n", comp.dynamic_height));
            }
        }
        IfComponentType::Text => {
            out.push_str(&format!("text_font={}\n", comp.font));
            if !comp.text.is_empty() {
                out.push_str(&format!("text={}\n", comp.text));
            }
            if comp.text_style != 0 {
                out.push_str(&format!("text_style={}\n", comp.text_style));
            }
            if comp.text_halign != 0 {
                out.push_str(&format!("text_halign={}\n", comp.text_halign));
            }
            if comp.text_valign != 0 {
                out.push_str(&format!("text_valign={}\n", comp.text_valign));
            }
            if comp.shadowed {
                out.push_str("text_shadow=yes\n");
            }
            if comp.colour != 0 {
                out.push_str(&format!("color={}\n", comp.colour));
            }
            if comp.trans != 0 {
                out.push_str(&format!("trans={}\n", comp.trans));
            }
        }
        IfComponentType::Rect => {
            if comp.colour != 0 {
                out.push_str(&format!("color={}\n", comp.colour));
            }
            if comp.fill {
                out.push_str("fill=yes\n");
            }
            if comp.trans != 0 {
                out.push_str(&format!("trans={}\n", comp.trans));
            }
        }
        IfComponentType::Line => {
            if comp.line_width != 0 {
                out.push_str(&format!("line_width={}\n", comp.line_width));
            }
            if comp.colour != 0 {
                out.push_str(&format!("color={}\n", comp.colour));
            }
            if comp.line_rtl {
                out.push_str("line_rtl=yes\n");
            }
        }
        _ => {}
    }

    if comp.active_properties != 0 {
        out.push_str(&format!("active_properties={}\n", comp.active_properties));
    }
    for ok in &comp.opkeys {
        out.push_str(&format!(
            "opkey={},{},{},{}\n",
            ok.index, ok.rate_duration, ok.code, ok.mask
        ));
    }
    if !comp.op_base.is_empty() {
        out.push_str(&format!("op_base={}\n", comp.op_base));
    }
    for (i, op) in comp.ops.iter().enumerate() {
        out.push_str(&format!("op{}={}\n", i + 1, op));
    }
    if comp.op_cursor_count != 0 {
        out.push_str(&format!("op_cursor_count={}\n", comp.op_cursor_count));
    }
    for &(idx, cursor) in &comp.op_cursors {
        out.push_str(&format!("op_cursor={},{}\n", idx, cursor));
    }
    if !comp.pause_text.is_empty() {
        out.push_str(&format!("pause_text={}\n", comp.pause_text));
    }
    if comp.drag_deadzone != 0 {
        out.push_str(&format!("drag_deadzone={}\n", comp.drag_deadzone));
    }
    if comp.drag_deadtime != 0 {
        out.push_str(&format!("drag_deadtime={}\n", comp.drag_deadtime));
    }
    if comp.drag_renderbehavior != 0 {
        out.push_str(&format!(
            "drag_renderbehavior={}\n",
            comp.drag_renderbehavior
        ));
    }
    if !comp.target_verb.is_empty() {
        out.push_str(&format!("target_verb={}\n", comp.target_verb));
    }
    if comp.target_object_id != -1 {
        out.push_str(&format!("target_object_id={}\n", comp.target_object_id));
    }
    if comp.target_enter_cursor != -1 {
        out.push_str(&format!(
            "target_enter_cursor={}\n",
            comp.target_enter_cursor
        ));
    }
    if comp.target_leave_cursor != -1 {
        out.push_str(&format!(
            "target_leave_cursor={}\n",
            comp.target_leave_cursor
        ));
    }

    let hook_names = [
        "onload",
        "onmouseover",
        "onmouseleave",
        "ontargetleave",
        "ontargetenter",
        "onvartransmit",
        "oninvtransmit",
        "onstattransmit",
        "ontimer",
        "onop",
        "hook10",
        "hook11",
        "hook12",
        "hook13",
        "hook14",
        "hook15",
        "hook16",
        "hook17",
        "hook18",
        "hook19",
    ];
    for (i, hook) in comp.hooks.iter().enumerate() {
        if let Some(args) = hook {
            let args_str: Vec<String> = args
                .iter()
                .map(|a| match a {
                    HookArg::Int(v) => format!("i{}", v),
                    HookArg::Str(s) => {
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
                        format!("s\"{}\"", escaped)
                    }
                })
                .collect();
            out.push_str(&format!("{}={}\n", hook_names[i], args_str.join(",")));
        }
    }

    let tl_names = [
        "vartransmit",
        "invtransmit",
        "stattransmit",
        "varctransmit",
        "varcstrtransmit",
    ];
    for (i, tl) in comp.transmit_lists.iter().enumerate() {
        if let Some(ids) = tl {
            let ids_str: Vec<String> = ids.iter().map(|v| v.to_string()).collect();
            out.push_str(&format!("{}_list={}\n", tl_names[i], ids_str.join(",")));
        }
    }

    out
}
