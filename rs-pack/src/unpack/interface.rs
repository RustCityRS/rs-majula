#[cfg(since_647)]
use crate::archive::INTERFACES as INTERFACE_ARCHIVE;
use crate::cache::r#if::IfTypeRaw;
#[cfg(since_647)]
use crate::cache::r#if::{
    DEFAULT_COLOUR, HOOK_NAMES, SIZE_MODES, TARGET_MASK, TEXT_ALIGN_H, TEXT_ALIGN_V, X_MODES,
    Y_MODES, hook_transmit_list, mode_name,
};
#[cfg(since_647)]
use crate::pack::interface::{encode_component, parse_components};
#[cfg(since_647)]
use crate::pack::names::AssetNames;
#[cfg(before_647)]
use crate::pack::pack_registry::PackRegistry;
#[cfg(before_647)]
use crate::pack::util::walk;
use crate::types::IfComponentType;
#[cfg(before_647)]
use crate::types::{Font, IfButtonType, PlayerStat};
#[cfg(since_647)]
use crate::unpack::config::format_colour;
#[cfg(before_647)]
use anyhow::Result;
#[cfg(since_647)]
use anyhow::{Context, Result, bail, ensure};
use rs_io::Packet;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use runec::clientscript::decompile::{Signature, component_hook_text};
#[cfg(since_647)]
use runec::symbol::SymbolRegistry;
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(before_647)]
use std::collections::{HashMap, HashSet};
#[cfg(since_647)]
use std::fmt::Write;
use std::fs;
#[cfg(since_647)]
use std::path::Path;
#[cfg(before_647)]
use std::path::{Path, PathBuf};
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

/// An interface as its group holds it: components in position order.
#[cfg(since_647)]
pub struct Interface {
    pub id: u32,
    pub components: Vec<IfTypeRaw>,
    bytes: Vec<Vec<u8>>,
}

#[cfg(since_647)]
pub fn decode_interfaces(store: &mut Js5Store) -> Result<Vec<Interface>> {
    let groups = store
        .reference_table(INTERFACE_ARCHIVE)
        .context("Missing interface table")?
        .groups
        .clone();
    let mut interfaces = Vec::with_capacity(groups.len());
    for info in &groups {
        let group = store
            .read_group(INTERFACE_ARCHIVE, info.id)?
            .context("Missing interface group")?;
        let bytes: Vec<Vec<u8>> = group
            .entries
            .iter()
            .map(|entry| entry.data().to_vec())
            .collect();
        let components = bytes
            .iter()
            .enumerate()
            .map(|(position, data)| {
                IfTypeRaw::decode(
                    &mut Packet::from(data.clone()),
                    position as u16,
                    info.id as i32,
                )
            })
            .collect();
        interfaces.push(Interface {
            id: info.id,
            components,
            bytes,
        });
    }
    Ok(interfaces)
}

/// Writes each interface's source, once it reads back to the cache's bytes.
#[cfg(since_647)]
pub fn write_interfaces(
    interfaces: &[Interface],
    output_dir: &Path,
    signatures: &HashMap<u32, Signature>,
    names: &AssetNames,
    registry: &SymbolRegistry,
) -> Result<u32> {
    let dir = output_dir.join("interfaces");
    fs::create_dir_all(&dir)?;
    for interface in interfaces {
        let mut text = String::new();
        for (position, component) in interface.components.iter().enumerate() {
            if position > 0 {
                text.push('\n');
            }
            emit_component(
                &mut text,
                component,
                interface.id,
                position as u32,
                signatures,
                names,
                registry,
            )
            .with_context(|| format!("Interface {}:{position}", interface.id))?;
        }
        let parsed = parse_components(&text, interface.id, names, registry)
            .with_context(|| format!("Reading interface {}", interface.id))?;
        ensure!(
            parsed.len() == interface.bytes.len(),
            "Interface {} text round trip changed its component count",
            interface.id
        );
        for ((position, component), bytes) in parsed.iter().zip(&interface.bytes) {
            let encoded = encode_component(component);
            if &encoded != bytes {
                let mut shown = String::new();
                emit_component(
                    &mut shown,
                    component,
                    interface.id,
                    *position,
                    signatures,
                    names,
                    registry,
                )?;
                bail!(
                    "Interface {}:{position} text round trip changed bytes:\n{bytes:?}\n{encoded:?}\n{shown}",
                    interface.id
                );
            }
        }
        fs::write(
            dir.join(format!("{}.if3", names.name("interface", interface.id))),
            text,
        )?;
    }
    Ok(interfaces.len() as u32)
}

#[cfg(since_647)]
fn emit_component(
    out: &mut String,
    comp: &IfTypeRaw,
    group: u32,
    position: u32,
    signatures: &HashMap<u32, Signature>,
    names: &AssetNames,
    registry: &SymbolRegistry,
) -> Result<()> {
    let name = names.component_name(group, position);
    writeln!(out, "// {group}:{position}\n[{name}]")?;
    let type_name = match comp.com_type {
        IfComponentType::Layer => "layer",
        IfComponentType::Rect => "rectangle",
        IfComponentType::Text => "text",
        IfComponentType::Graphic => "graphic",
        IfComponentType::Model => "model",
        IfComponentType::Line => "line",
        other => bail!("Component type {} isn't written out", other as u8),
    };
    writeln!(out, "type={type_name}")?;
    if let Some(name) = &comp.com_name {
        writeln!(out, "name={name}")?;
    }
    if comp.client_code != 0 {
        writeln!(out, "clientcode={}", comp.client_code)?;
    }
    if comp.x != 0 {
        writeln!(out, "x={}", comp.x)?;
    }
    if comp.y != 0 {
        writeln!(out, "y={}", comp.y)?;
    }
    if comp.width != 0 {
        writeln!(out, "width={}", comp.width)?;
    }
    if comp.height != 0 {
        writeln!(out, "height={}", comp.height)?;
    }
    if comp.width_mode != 0 {
        writeln!(
            out,
            "widthmode={}",
            mode_name(&SIZE_MODES, comp.width_mode as u8)
        )?;
    }
    if comp.height_mode != 0 {
        writeln!(
            out,
            "heightmode={}",
            mode_name(&SIZE_MODES, comp.height_mode as u8)
        )?;
    }
    if comp.x_mode != 0 {
        writeln!(out, "xmode={}", mode_name(&X_MODES, comp.x_mode as u8))?;
    }
    if comp.y_mode != 0 {
        writeln!(out, "ymode={}", mode_name(&Y_MODES, comp.y_mode as u8))?;
    }
    if comp.parent_id != u16::MAX {
        writeln!(
            out,
            "layer={}",
            names.component_name(group, comp.parent_id as u32)
        )?;
    }
    if comp.flags & 1 != 0 {
        writeln!(out, "hide=yes")?;
    }
    ensure!(
        comp.flags & !1 == 0,
        "Unknown component flags {}",
        comp.flags
    );
    match comp.com_type {
        IfComponentType::Layer => emit_layer(out, comp)?,
        IfComponentType::Rect => {
            writeln!(out, "colour={}", format_colour(comp.colour))?;
            if comp.fill {
                writeln!(out, "fill=yes")?;
            }
            if comp.trans != 0 {
                writeln!(out, "trans={}", comp.trans)?;
            }
        }
        IfComponentType::Text => emit_text(out, comp, names)?,
        IfComponentType::Graphic => emit_graphic(out, comp, names)?,
        IfComponentType::Model => emit_model(out, comp, names)?,
        IfComponentType::Line => {
            if comp.line_width != 1 {
                writeln!(out, "linewid={}", comp.line_width)?;
            }
            writeln!(out, "colour={}", format_colour(comp.colour))?;
            if comp.line_rtl {
                writeln!(out, "linedirection=yes")?;
            }
        }
        _ => {}
    }
    emit_properties(out, comp)?;
    emit_ops(out, comp, names)?;
    if !comp.target_verb.is_empty() {
        writeln!(out, "targetverb={}", comp.target_verb)?;
    }
    for (key, kind, value) in [
        ("targetobj", "obj", comp.target_object_id),
        ("targetcursor2", "cursor", comp.target_enter_cursor),
        ("targetcursor3", "cursor", comp.target_leave_cursor),
    ] {
        if value != -1 {
            writeln!(out, "{key}={}", names.reference(kind, value as i64))?;
        }
    }
    emit_hooks(out, comp, signatures, names, registry)
}

#[cfg(since_647)]
fn emit_layer(out: &mut String, comp: &IfTypeRaw) -> Result<()> {
    if comp.scroll_width != 0 {
        writeln!(out, "scrollwidth={}", comp.scroll_width)?;
    }
    if comp.scroll != 0 {
        writeln!(out, "scrollheight={}", comp.scroll)?;
    }
    if comp.no_click_through {
        writeln!(out, "noclickthrough=yes")?;
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_text(out: &mut String, comp: &IfTypeRaw, names: &AssetNames) -> Result<()> {
    writeln!(
        out,
        "textfont={}",
        names.reference("fontmetrics", comp.font as i64)
    )?;
    if !comp.text.is_empty() {
        writeln!(out, "text={}", comp.text)?;
    }
    if comp.text_style != 0 {
        writeln!(out, "textlineheight={}", comp.text_style)?;
    }
    if comp.text_halign != 0 {
        writeln!(
            out,
            "textalignh={}",
            mode_name(&TEXT_ALIGN_H, comp.text_halign)
        )?;
    }
    if comp.text_valign != 0 {
        writeln!(
            out,
            "textalignv={}",
            mode_name(&TEXT_ALIGN_V, comp.text_valign)
        )?;
    }
    if comp.shadowed {
        writeln!(out, "textshadow=yes")?;
    }
    if comp.colour != DEFAULT_COLOUR {
        writeln!(out, "colour={}", format_colour(comp.colour))?;
    }
    if comp.trans != 0 {
        writeln!(out, "trans={}", comp.trans)?;
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_graphic(out: &mut String, comp: &IfTypeRaw, names: &AssetNames) -> Result<()> {
    if comp.graphic != -1 {
        let graphic = names.reference("sprites", comp.graphic as i64);
        if graphic.contains(',') {
            writeln!(out, "graphic=\"{graphic}\"")?;
        } else {
            writeln!(out, "graphic={graphic}")?;
        }
    }
    if comp.angle_2d != 0 {
        writeln!(out, "2dangle={}", comp.angle_2d)?;
    }
    if comp.tiling {
        writeln!(out, "tiling=yes")?;
    }
    if comp.alpha {
        writeln!(out, "alpha=yes")?;
    }
    if comp.trans != 0 {
        writeln!(out, "trans={}", comp.trans)?;
    }
    if comp.outline != 0 {
        writeln!(out, "outline={}", comp.outline)?;
    }
    if comp.graphic_shadow != 0 {
        writeln!(out, "graphicshadow={}", comp.graphic_shadow)?;
    }
    if comp.vflip {
        writeln!(out, "vflip=yes")?;
    }
    if comp.hflip {
        writeln!(out, "hflip=yes")?;
    }
    if comp.colour != DEFAULT_COLOUR {
        writeln!(out, "colour={}", format_colour(comp.colour))?;
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_model(out: &mut String, comp: &IfTypeRaw, names: &AssetNames) -> Result<()> {
    writeln!(out, "model={}", names.reference("model", comp.model as i64))?;
    if comp.model_origin_x != 0 || comp.model_origin_y != 0 {
        writeln!(
            out,
            "modelorigin={},{}",
            comp.model_origin_x, comp.model_origin_y
        )?;
    }
    if comp.xan != 0 || comp.yan != 0 || comp.model_zan != 0 {
        writeln!(
            out,
            "modelangle={},{},{}",
            comp.xan, comp.yan, comp.model_zan
        )?;
    }
    if comp.zoom != 100 {
        writeln!(out, "modelzoom={}", comp.zoom)?;
    }
    if comp.model_flags & 2 != 0 {
        writeln!(out, "modeloffset={}", comp.sprite_y_offset)?;
    }
    if comp.anim != -1 {
        writeln!(
            out,
            "modelanim={}",
            names.reference("seq", comp.anim as i64)
        )?;
    }
    if comp.model_flags & 4 != 0 {
        writeln!(out, "modelorthog=yes")?;
    }
    if comp.model_flags & 8 != 0 {
        writeln!(out, "modelnodepth=yes")?;
    }
    if comp.dynamic_width != 0 {
        writeln!(out, "modelobjwidth={}", comp.dynamic_width)?;
    }
    if comp.dynamic_height != 0 {
        writeln!(out, "modelobjheight={}", comp.dynamic_height)?;
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_properties(out: &mut String, comp: &IfTypeRaw) -> Result<()> {
    let properties = comp.active_properties;
    ensure!(
        properties >> 23 == 0,
        "Unknown active properties {properties}"
    );
    if properties & 1 != 0 {
        writeln!(out, "pausebutton=yes")?;
    }
    let ops: Vec<_> = (1..=10)
        .filter(|op| (properties >> op) & 1 != 0)
        .map(|op: u32| op.to_string())
        .collect();
    if !ops.is_empty() {
        writeln!(out, "transmitop={}", ops.join(","))?;
    }
    let depth = (properties >> 18) & 7;
    if depth != 0 {
        writeln!(out, "dragdepth={depth}")?;
    }
    if (properties >> 21) & 1 != 0 {
        writeln!(out, "candrop=yes")?;
    }
    let targets: Vec<_> = TARGET_MASK
        .iter()
        .enumerate()
        .filter(|(bit, _)| (properties >> (11 + bit)) & 1 != 0)
        .map(|(_, target)| *target)
        .collect();
    if !targets.is_empty() {
        writeln!(out, "targetmask={}", targets.join(","))?;
    }
    if (properties >> 22) & 1 != 0 {
        writeln!(out, "targetable=yes")?;
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_ops(out: &mut String, comp: &IfTypeRaw, names: &AssetNames) -> Result<()> {
    if !comp.op_base.is_empty() {
        writeln!(out, "opbase={}", comp.op_base)?;
    }
    if !comp.pause_text.is_empty() {
        writeln!(out, "pausetext={}", comp.pause_text)?;
    }
    for (op, text) in comp.ops.iter().enumerate() {
        if !text.is_empty() {
            writeln!(out, "op{}={text}", op + 1)?;
        }
    }
    if comp.drag_deadzone != 0 {
        writeln!(out, "dragdeadzone={}", comp.drag_deadzone)?;
    }
    if comp.drag_deadtime != 0 {
        writeln!(out, "dragdeadtime={}", comp.drag_deadtime)?;
    }
    if comp.drag_renderbehavior != 0 {
        writeln!(out, "dragrenderbehaviour={}", comp.drag_renderbehavior)?;
    }
    ensure!(
        comp.op_cursor_count as usize == comp.op_cursors.len(),
        "Op cursor count {} with {} cursors",
        comp.op_cursor_count,
        comp.op_cursors.len()
    );
    for &(op, cursor) in &comp.op_cursors {
        writeln!(
            out,
            "opcursor{op}={}",
            names.reference("cursor", cursor as i64)
        )?;
    }
    for key in &comp.opkeys {
        write!(out, "opkey{}={},{}", key.index + 1, key.code, key.mask)?;
        // The rate has no key of its own: it follows the code and mask when it's set.
        if key.rate_duration != 0 {
            write!(out, ",{}", key.rate_duration)?;
        }
        out.push('\n');
    }
    Ok(())
}

#[cfg(since_647)]
fn emit_hooks(
    out: &mut String,
    comp: &IfTypeRaw,
    signatures: &HashMap<u32, Signature>,
    names: &AssetNames,
    registry: &SymbolRegistry,
) -> Result<()> {
    for (hook, values) in comp.hooks.iter().enumerate() {
        let list = hook_transmit_list(hook);
        let transmits = list.and_then(|(list, kind)| {
            comp.transmit_lists[list]
                .as_deref()
                .map(|transmits| (kind, transmits))
        });
        let Some(values) = values else {
            ensure!(
                transmits.is_none(),
                "{} has a transmit list without a script",
                HOOK_NAMES[hook]
            );
            continue;
        };
        let text = component_hook_text(values, transmits, signatures, names, registry)?;
        writeln!(out, "{}={text}", HOOK_NAMES[hook])?;
    }
    Ok(())
}
