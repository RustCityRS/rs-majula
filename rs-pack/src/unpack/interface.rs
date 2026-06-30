use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::Result;
use rs_io::Packet;
use rs_io::jag::JagFile;
use tracing::{debug, warn};

use crate::pack::pack_registry::PackRegistry;
use crate::pack::util::walk;
use crate::types::Font;

const STAT_NAMES: [&str; 21] = [
    "attack",
    "defence",
    "strength",
    "hitpoints",
    "ranged",
    "prayer",
    "magic",
    "cooking",
    "woodcutting",
    "fletching",
    "fishing",
    "firemaking",
    "crafting",
    "smithing",
    "mining",
    "herblore",
    "agility",
    "thieving",
    "stat18",
    "stat19",
    "runecraft",
];

#[derive(Default)]
struct Component {
    id: u16,
    root_layer: i32,
    com_type: u8,
    button_type: u8,
    client_code: u16,
    width: u16,
    height: u16,
    #[cfg(since_244)]
    trans: u8,
    over_layer: Option<u16>,
    comparators: Vec<(u8, i16)>,
    scripts: Vec<Vec<u16>>,
    // layer
    scroll: u16,
    hide: bool,
    children: Vec<(u16, i16, i16)>,
    // inv
    draggable: bool,
    interactable: bool,
    usable: bool,
    #[cfg(since_245_2)]
    swappable: bool,
    margin_x: i32,
    margin_y: i32,
    slots: Vec<Option<(i16, i16, String)>>,
    iops: Vec<String>,
    // rect
    fill: bool,
    // text / invtext
    center: bool,
    font: u8,
    shadowed: bool,
    text: String,
    active_text: String,
    colour: i32,
    active_colour: i32,
    over_colour: i32,
    #[cfg(since_245_2)]
    active_over_colour: i32,
    // graphic
    graphic: String,
    active_graphic: String,
    // model
    model: Option<u16>,
    active_model: Option<u16>,
    anim: Option<u16>,
    active_anim: Option<u16>,
    zoom: u16,
    xan: u16,
    yan: u16,
    // button target / inv action
    action_verb: String,
    action: String,
    action_target: u16,
    // button option
    option: String,
}

impl Component {
    fn is_root(&self) -> bool {
        self.id as i32 == self.root_layer
    }
}

struct Ctx<'a> {
    if_names: HashMap<u16, String>,
    registry: &'a PackRegistry,
}

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

fn read_optional_id(buf: &mut Packet) -> Option<u16> {
    let b = buf.g1();
    if b == 0 {
        None
    } else {
        Some(((b as u16 - 1) << 8) + buf.g1() as u16)
    }
}

fn decode(buf: &mut Packet) -> (Vec<u16>, HashMap<u16, Component>) {
    let _count = buf.g2();
    let mut order: Vec<u16> = Vec::new();
    let mut comps: HashMap<u16, Component> = HashMap::new();
    let mut layer: i32 = -1;

    while buf.remaining() > 0 {
        let mut id = buf.g2();
        if id == 0xFFFF {
            layer = buf.g2() as i32;
            id = buf.g2();
        }

        order.push(id);
        let mut com = Component {
            id,
            root_layer: layer,
            ..Default::default()
        };

        com.com_type = buf.g1();
        com.button_type = buf.g1();
        com.client_code = buf.g2();
        com.width = buf.g2();
        com.height = buf.g2();
        #[cfg(since_244)]
        {
            com.trans = buf.g1();
        }
        com.over_layer = read_optional_id(buf);

        let comparator_count = buf.g1();
        for _ in 0..comparator_count {
            let comparator = buf.g1();
            let operand = buf.g2s();
            com.comparators.push((comparator, operand));
        }

        let script_count = buf.g1();
        for _ in 0..script_count {
            let opcode_count = buf.g2() as usize;
            let mut ops = Vec::with_capacity(opcode_count);
            for _ in 0..opcode_count {
                ops.push(buf.g2());
            }
            com.scripts.push(ops);
        }

        match com.com_type {
            0 => {
                // layer
                com.scroll = buf.g2();
                com.hide = buf.g1() != 0;
                #[cfg(rev = "225")]
                let child_count = buf.g1() as usize;
                #[cfg(since_244)]
                let child_count = buf.g2() as usize;
                for _ in 0..child_count {
                    let cid = buf.g2();
                    let cx = buf.g2s();
                    let cy = buf.g2s();
                    com.children.push((cid, cx, cy));
                }
            }
            2 => {
                // inv
                com.draggable = buf.g1() != 0;
                com.interactable = buf.g1() != 0;
                com.usable = buf.g1() != 0;
                #[cfg(since_245_2)]
                {
                    com.swappable = buf.g1() != 0;
                }
                com.margin_x = buf.g1() as i32;
                com.margin_y = buf.g1() as i32;
                com.slots = Vec::with_capacity(20);
                for _ in 0..20 {
                    if buf.g1() != 0 {
                        let x = buf.g2s();
                        let y = buf.g2s();
                        let sprite = buf.gjstr(10);
                        com.slots.push(Some((x, y, sprite)));
                    } else {
                        com.slots.push(None);
                    }
                }
                com.iops = (0..5).map(|_| buf.gjstr(10)).collect();
            }
            3 => {
                // rect
                com.fill = buf.g1() != 0;
            }
            4 => {
                // text
                com.center = buf.g1() != 0;
                com.font = buf.g1();
                com.shadowed = buf.g1() != 0;
                com.text = buf.gjstr(10);
                com.active_text = buf.gjstr(10);
            }
            5 => {
                // graphic
                com.graphic = buf.gjstr(10);
                com.active_graphic = buf.gjstr(10);
            }
            6 => {
                // model
                com.model = read_optional_id(buf);
                com.active_model = read_optional_id(buf);
                com.anim = read_optional_id(buf);
                com.active_anim = read_optional_id(buf);
                com.zoom = buf.g2();
                com.xan = buf.g2();
                com.yan = buf.g2();
            }
            7 => {
                // invtext
                com.center = buf.g1() != 0;
                com.font = buf.g1();
                com.shadowed = buf.g1() != 0;
                com.colour = buf.g4s();
                com.margin_x = buf.g2() as i32;
                com.margin_y = buf.g2() as i32;
                com.interactable = buf.g1() != 0;
                com.iops = (0..5).map(|_| buf.gjstr(10)).collect();
            }
            other => panic!("Unknown interface component type {other} (id {id})"),
        }

        // colors for rect / text (invtext reads its own color above)
        if com.com_type == 3 || com.com_type == 4 {
            com.colour = buf.g4s();
            com.active_colour = buf.g4s();
            com.over_colour = buf.g4s();
            #[cfg(since_245_2)]
            {
                com.active_over_colour = buf.g4s();
            }
        }

        // target / inv action verbs
        if com.button_type == 2 || com.com_type == 2 {
            com.action_verb = buf.gjstr(10);
            com.action = buf.gjstr(10);
            com.action_target = buf.g2();
        }

        // normal / toggle / select / pause option
        if matches!(com.button_type, 1 | 4 | 5 | 6) {
            com.option = buf.gjstr(10);
        }

        comps.insert(id, com);
    }

    (order, comps)
}

fn build_names(
    order: &[u16],
    comps: &HashMap<u16, Component>,
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

fn type_name(com_type: u8) -> &'static str {
    match com_type {
        0 => "layer",
        2 => "inv",
        3 => "rect",
        4 => "text",
        5 => "graphic",
        6 => "model",
        7 => "invtext",
        other => panic!("Unknown interface component type {other}"),
    }
}

fn button_name(button_type: u8) -> &'static str {
    match button_type {
        1 => "normal",
        2 => "target",
        3 => "close",
        4 => "toggle",
        5 => "select",
        6 => "pause",
        other => panic!("Unknown interface button type {other}"),
    }
}

fn font_name(font: u8) -> &'static str {
    let f = match font {
        0 => Font::P11,
        1 => Font::P12,
        2 => Font::B12,
        3 => Font::Q8,
        other => panic!("Unknown interface font {other}"),
    };
    f.name()
}

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

fn next_op(ops: &[u16], idx: &mut usize) -> u16 {
    let v = ops[*idx];
    *idx += 1;
    v
}

fn export_scripts(lines: &mut Vec<String>, com: &Component, ctx: &Ctx) {
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
                1 => format!("stat_level,{}", STAT_NAMES[next_op(ops, &mut idx) as usize]),
                2 => format!(
                    "stat_base_level,{}",
                    STAT_NAMES[next_op(ops, &mut idx) as usize]
                ),
                3 => format!("stat_xp,{}", STAT_NAMES[next_op(ops, &mut idx) as usize]),
                4 => {
                    let inv = next_op(ops, &mut idx);
                    let obj = next_op(ops, &mut idx);
                    format!("inv_count,{},{}", ctx.iface(inv), ctx.obj(obj))
                }
                5 => format!("pushvar,{}", ctx.varp(next_op(ops, &mut idx))),
                6 => format!(
                    "stat_xp_remaining,{}",
                    STAT_NAMES[next_op(ops, &mut idx) as usize]
                ),
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

fn export_comparators(lines: &mut Vec<String>, com: &Component) {
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

fn export_component(
    lines: &mut Vec<String>,
    com: &Component,
    x: i16,
    y: i16,
    parent: &str,
    comps: &HashMap<u16, Component>,
    ctx: &Ctx,
) {
    lines.push(format!("[{}]", ctx.iface_suffix(com.id)));
    if !parent.is_empty() {
        lines.push(format!("layer={parent}"));
    }
    lines.push(format!("type={}", type_name(com.com_type)));
    lines.push(format!("x={x}"));
    lines.push(format!("y={y}"));
    if com.button_type != 0 {
        lines.push(format!("buttontype={}", button_name(com.button_type)));
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
    if com.button_type == 2 || com.com_type == 2 {
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
    if matches!(com.button_type, 1 | 4 | 5 | 6) && !com.option.is_empty() {
        lines.push(format!("option={}", com.option));
    }

    // nested layer children
    if com.com_type == 0 && !com.children.is_empty() {
        let suffix = ctx.iface_suffix(com.id).to_string();
        export_children(lines, com, &suffix, false, comps, ctx);
    }
}

fn export_type_specific(lines: &mut Vec<String>, com: &Component, ctx: &Ctx) {
    match com.com_type {
        0 => {
            if com.scroll != 0 {
                lines.push(format!("scroll={}", com.scroll));
            }
            if com.hide {
                lines.push("hide=yes".to_string());
            }
        }
        2 => {
            if com.draggable {
                lines.push("draggable=yes".to_string());
            }
            if com.interactable {
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
        3 => {
            if com.fill {
                lines.push("fill=yes".to_string());
            }
            export_colours(lines, com);
        }
        4 => {
            if com.center {
                lines.push("center=yes".to_string());
            }
            lines.push(format!("font={}", font_name(com.font)));
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
        5 => {
            if !com.graphic.is_empty() {
                lines.push(format!("graphic={}", com.graphic));
            }
            if !com.active_graphic.is_empty() {
                lines.push(format!("activegraphic={}", com.active_graphic));
            }
        }
        6 => {
            if let Some(model) = com.model {
                lines.push(format!("model={}", ctx.model(model)));
            }
            if let Some(model) = com.active_model {
                lines.push(format!("activemodel={}", ctx.model(model)));
            }
            if let Some(anim) = com.anim {
                lines.push(format!("anim={}", ctx.seq(anim)));
            }
            if let Some(anim) = com.active_anim {
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
        7 => {
            if com.center {
                lines.push("center=yes".to_string());
            }
            lines.push(format!("font={}", font_name(com.font)));
            if com.shadowed {
                lines.push("shadowed=yes".to_string());
            }
            if com.colour != 0 {
                lines.push(format!("colour=0x{:06X}", com.colour));
            }
            if com.margin_x != 0 || com.margin_y != 0 {
                lines.push(format!("margin={},{}", com.margin_x, com.margin_y));
            }
            if com.interactable {
                lines.push("interactable=yes".to_string());
            }
            export_options(lines, com);
        }
        other => panic!("Unknown interface component type {other}"),
    }
}

fn export_colours(lines: &mut Vec<String>, com: &Component) {
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

fn export_options(lines: &mut Vec<String>, com: &Component) {
    for (i, iop) in com.iops.iter().enumerate() {
        if !iop.is_empty() {
            lines.push(format!("option{}={}", i + 1, iop));
        }
    }
}

fn export_children(
    lines: &mut Vec<String>,
    parent_com: &Component,
    parent_suffix: &str,
    is_root: bool,
    comps: &HashMap<u16, Component>,
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

fn read_top_lines(path: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
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
    std::fs::create_dir_all(out_pack_dir)?;
    let order_text = order
        .iter()
        .map(u16::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(out_pack_dir.join("interface.order"), order_text + "\n")?;

    let max_id = ctx.if_names.keys().copied().max().unwrap_or(0);
    let mut pack_lines = Vec::with_capacity(ctx.if_names.len());
    for id in 0..=max_id {
        if let Some(name) = ctx.if_names.get(&id) {
            pack_lines.push(format!("{id}={name}"));
        }
    }
    std::fs::write(
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
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&dest, out)?;
        written += 1;
    }

    debug!(
        "  Regenerated interface: {} components, {written} .if files into {}",
        comps.len(),
        out_scripts_dir.display()
    );
    Ok(())
}
