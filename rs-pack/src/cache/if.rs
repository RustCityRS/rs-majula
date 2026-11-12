#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use crate::cache::decode::{read_id, read_optional_id};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
use crate::types::{IfButtonType, IfComponentType};
#[cfg(since_647)]
use anyhow::{Context, Result};
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::js5::split_group_entries;
use rs_ver::STRING_TERMINATOR;
#[cfg(since_647)]
use runec::clientscript::HookValue;
#[cfg(since_647)]
use std::array::from_fn;
use std::collections::HashMap;
#[cfg(before_647)]
use std::collections::HashSet;

pub struct IfType {
    pub id: u16,
    pub root_layer: i32,
    pub com_name: Option<Box<str>>,
    pub overlay: bool,
    pub button_type: IfButtonType,
    pub width: u16,
    pub height: u16,
    pub draggable: bool,
    pub operable: bool,
    pub usable: bool,
    pub margin_x: u8,
    pub margin_y: u8,
    pub slot_offsets: Option<Box<[(i16, i16)]>>,
    pub iop: Option<Box<[Option<Box<str>>]>>,
    pub action_target: u16,
    pub x: i16,
    pub y: i16,
    pub scrolls: bool,
    pub layer: bool,
    #[cfg(since_647)]
    pub active_properties: u32,
}

impl IfType {
    /// Where an inventory's slot is drawn, from its interface's top left:
    /// a 32 by 32 square from there.
    pub fn slot_at(&self, slot: u16) -> Option<(i16, i16)> {
        let columns = self.width.max(1);
        let (col, row) = (slot % columns, slot / columns);
        if row >= self.height {
            return None;
        }
        let (dx, dy) = self
            .slot_offsets
            .as_deref()
            .and_then(|o| o.get(usize::from(slot)))
            .copied()
            .unwrap_or_default();
        Some((
            self.x + col as i16 * (32 + i16::from(self.margin_x)) + dx,
            self.y + row as i16 * (32 + i16::from(self.margin_y)) + dy,
        ))
    }
}

impl From<IfTypeRaw> for IfType {
    fn from(raw: IfTypeRaw) -> Self {
        let inventory = matches!(
            raw.com_type,
            IfComponentType::Inv | IfComponentType::InvText
        );
        IfType {
            id: raw.id,
            root_layer: raw.root_layer,
            com_name: raw.com_name,
            overlay: raw.overlay,
            button_type: raw.button_type,
            width: raw.width,
            height: raw.height,
            draggable: {
                #[cfg(before_245_2)]
                {
                    raw.draggable
                }
                #[cfg(since_245_2)]
                {
                    raw.draggable || raw.swappable
                }
            },
            operable: raw.operable,
            usable: raw.usable,
            margin_x: raw.margin_x as u8,
            margin_y: raw.margin_y as u8,
            slot_offsets: (!raw.slots.is_empty()).then(|| {
                raw.slots
                    .iter()
                    .map(|slot| slot.as_ref().map(|(x, y, _)| (*x, *y)).unwrap_or_default())
                    .collect()
            }),
            iop: inventory.then(|| {
                raw.iop
                    .into_iter()
                    .map(|s| (!s.is_empty()).then(|| s.into_boxed_str()))
                    .collect()
            }),
            action_target: raw.action_target,
            x: 0,
            y: 0,
            scrolls: false,
            layer: raw.com_type == IfComponentType::Layer,
            #[cfg(since_647)]
            active_properties: raw.active_properties,
        }
    }
}

#[derive(Default)]
pub struct IfTypeRaw {
    pub id: u16,
    pub root_layer: i32,
    pub overlay: bool,
    #[cfg(since_647)]
    pub version: u8,
    pub com_type: IfComponentType,
    pub com_name: Option<Box<str>>,
    pub button_type: IfButtonType,
    pub client_code: u16,
    #[cfg(since_647)]
    pub x: i16,
    #[cfg(since_647)]
    pub y: i16,
    pub width: u16,
    pub height: u16,
    #[cfg(since_244)]
    pub trans: u8,
    #[cfg(since_647)]
    pub width_mode: i8,
    #[cfg(since_647)]
    pub height_mode: i8,
    #[cfg(since_647)]
    pub x_mode: i8,
    #[cfg(since_647)]
    pub y_mode: i8,
    pub over_layer: Option<u16>,
    #[cfg(since_647)]
    pub parent_id: Option<u16>,
    #[cfg(since_647)]
    pub flags: u8,
    pub comparators: Vec<(u8, i16)>,
    pub scripts: Vec<Vec<u16>>,
    #[cfg(since_647)]
    pub scroll_width: u16,
    pub scroll: u16,
    pub hide: bool,
    pub children: Vec<(u16, i16, i16)>,
    #[cfg(since_647)]
    pub no_click_through: bool,
    pub draggable: bool,
    pub operable: bool,
    pub usable: bool,
    #[cfg(since_245_2)]
    pub swappable: bool,
    pub margin_x: i32,
    pub margin_y: i32,
    pub slots: Vec<Option<(i16, i16, String)>>,
    pub iop: Vec<String>,
    pub fill: bool,
    pub center: bool,
    pub font: i32,
    pub shadowed: bool,
    pub text: String,
    pub active_text: String,
    #[cfg(since_647)]
    pub text_style: u8,
    #[cfg(since_647)]
    pub text_halign: u8,
    #[cfg(since_647)]
    pub text_valign: u8,
    pub colour: i32,
    pub active_colour: i32,
    pub over_colour: i32,
    #[cfg(since_245_2)]
    pub active_over_colour: i32,
    #[cfg(before_647)]
    pub graphic: String,
    #[cfg(since_647)]
    pub graphic: Option<i32>,
    pub active_graphic: String,
    #[cfg(since_647)]
    pub angle_2d: u16,
    #[cfg(since_647)]
    pub tiling: bool,
    #[cfg(since_647)]
    pub alpha: bool,
    #[cfg(since_647)]
    pub outline: u8,
    #[cfg(since_647)]
    pub graphic_shadow: i32,
    #[cfg(since_647)]
    pub vflip: bool,
    #[cfg(since_647)]
    pub hflip: bool,
    pub model: i32,
    pub active_model: i32,
    #[cfg(since_647)]
    pub model_flags: u8,
    #[cfg(since_647)]
    pub model_origin_x: i16,
    #[cfg(since_647)]
    pub model_origin_y: i16,
    #[cfg(since_647)]
    pub sprite_y_offset: i16,
    pub xan: u16,
    pub yan: u16,
    #[cfg(since_647)]
    pub model_zan: u16,
    pub zoom: i32,
    pub anim: i32,
    pub active_anim: i32,
    #[cfg(since_647)]
    pub dynamic_width: u16,
    #[cfg(since_647)]
    pub dynamic_height: u16,
    #[cfg(since_647)]
    pub line_width: u8,
    #[cfg(since_647)]
    pub line_rtl: bool,
    pub action_verb: String,
    pub action: String,
    pub action_target: u16,
    pub option: String,
    #[cfg(since_647)]
    pub active_properties: u32,
    #[cfg(since_647)]
    pub opkeys: Vec<OpKey>,
    #[cfg(since_647)]
    pub op_base: String,
    #[cfg(since_647)]
    pub ops: Vec<String>,
    #[cfg(since_647)]
    pub op_cursor_count: u8,
    #[cfg(since_647)]
    pub op_cursors: Vec<(u8, u16)>,
    #[cfg(since_647)]
    pub pause_text: String,
    #[cfg(since_647)]
    pub drag_deadzone: u8,
    #[cfg(since_647)]
    pub drag_deadtime: u8,
    #[cfg(since_647)]
    pub drag_renderbehavior: u8,
    #[cfg(since_647)]
    pub target_verb: String,
    #[cfg(since_647)]
    pub target_object_id: Option<u16>,
    #[cfg(since_647)]
    pub target_enter_cursor: Option<u16>,
    #[cfg(since_647)]
    pub target_leave_cursor: Option<u16>,
    #[cfg(since_647)]
    pub hooks: [Option<Vec<HookValue>>; 20],
    #[cfg(since_647)]
    pub transmit_lists: [Option<Vec<i32>>; 5],
}

impl IfTypeRaw {
    pub fn decode(
        buf: &mut Packet,
        id: u16,
        root_layer: i32,
        #[cfg(before_647)] server: bool,
    ) -> Self {
        let mut com = Self {
            id,
            root_layer,
            #[cfg(since_647)]
            line_width: 1,
            ..Self::default()
        };
        #[cfg(before_647)]
        {
            if server {
                com.com_name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str());
                com.overlay = buf.g1() == 1;
            }
            com.com_type = IfComponentType::try_from(buf.g1()).unwrap();
            com.button_type = IfButtonType::try_from(buf.g1()).unwrap();
        }
        #[cfg(since_647)]
        {
            com.version = buf.g1();
            let kind = buf.g1();
            if kind & 0x80 != 0 {
                com.com_name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str());
            }
            com.com_type = IfComponentType::try_from(kind & 0x7f).unwrap();
        }
        com.client_code = buf.g2();
        #[cfg(since_647)]
        {
            com.x = buf.g2s();
            com.y = buf.g2s();
        }
        com.width = buf.g2();
        com.height = buf.g2();
        #[cfg(before_647)]
        {
            #[cfg(since_244)]
            if !server {
                com.trans = buf.g1();
            }
            com.over_layer = read_optional_id(buf);
            com.comparators = (0..buf.g1()).map(|_| (buf.g1(), buf.g2s())).collect();
            com.scripts = (0..buf.g1())
                .map(|_| (0..buf.g2()).map(|_| buf.g2()).collect())
                .collect();
        }
        #[cfg(since_647)]
        {
            com.width_mode = buf.g1s();
            com.height_mode = buf.g1s();
            com.x_mode = buf.g1s();
            com.y_mode = buf.g1s();
            com.parent_id = read_optional_id(buf);
            com.flags = buf.g1();
        }
        match com.com_type {
            IfComponentType::Layer => {
                #[cfg(since_647)]
                {
                    com.scroll_width = buf.g2();
                }
                com.scroll = buf.g2();
                #[cfg(before_647)]
                {
                    com.hide = buf.g1() != 0;
                    #[cfg(rev = "225")]
                    let count = buf.g1() as usize;
                    #[cfg(since_244)]
                    let count = buf.g2() as usize;
                    com.children = (0..count)
                        .map(|_| (buf.g2(), buf.g2s(), buf.g2s()))
                        .collect();
                }
                #[cfg(since_647)]
                if com.version == u8::MAX {
                    com.no_click_through = buf.g1() == 1;
                }
            }
            #[cfg(before_647)]
            IfComponentType::Inv => {
                com.draggable = buf.g1() != 0;
                com.operable = buf.g1() != 0;
                com.usable = buf.g1() != 0;
                #[cfg(since_245_2)]
                {
                    com.swappable = buf.g1() != 0;
                }
                com.margin_x = buf.g1() as i32;
                com.margin_y = buf.g1() as i32;
                com.slots = (0..20)
                    .map(|_| {
                        (buf.g1() != 0)
                            .then(|| (buf.g2s(), buf.g2s(), buf.gjstr(STRING_TERMINATOR)))
                    })
                    .collect();
                com.iop = (0..5).map(|_| buf.gjstr(STRING_TERMINATOR)).collect();
            }
            IfComponentType::Rect => {
                #[cfg(since_647)]
                {
                    com.colour = buf.g4s();
                }
                com.fill = buf.g1() != 0;
                #[cfg(since_647)]
                {
                    com.trans = buf.g1();
                }
            }
            IfComponentType::Text => {
                #[cfg(before_647)]
                {
                    com.center = buf.g1() != 0;
                    com.font = buf.g1() as i32;
                    com.shadowed = buf.g1() != 0;
                }
                #[cfg(since_647)]
                {
                    com.font = read_id(buf);
                }
                com.text = buf.gjstr(STRING_TERMINATOR);
                #[cfg(before_647)]
                {
                    com.active_text = buf.gjstr(STRING_TERMINATOR);
                }
                #[cfg(since_647)]
                {
                    com.text_style = buf.g1();
                    com.text_halign = buf.g1();
                    com.text_valign = buf.g1();
                    com.shadowed = buf.g1() == 1;
                    com.colour = buf.g4s();
                    com.trans = buf.g1();
                }
            }
            IfComponentType::Graphic => {
                #[cfg(before_647)]
                {
                    com.graphic = buf.gjstr(STRING_TERMINATOR);
                    com.active_graphic = buf.gjstr(STRING_TERMINATOR);
                }
                #[cfg(since_647)]
                {
                    let graphic = buf.g4s();
                    com.graphic = (graphic != -1).then_some(graphic);
                    com.angle_2d = buf.g2();
                    let flags = buf.g1();
                    com.tiling = flags & 1 != 0;
                    com.alpha = flags & 2 != 0;
                    com.trans = buf.g1();
                    com.outline = buf.g1();
                    com.graphic_shadow = buf.g4s();
                    com.vflip = buf.g1() == 1;
                    com.hflip = buf.g1() == 1;
                    com.colour = buf.g4s();
                }
            }
            IfComponentType::Model => {
                #[cfg(before_647)]
                {
                    com.model = read_optional_id(buf).map_or(-1, i32::from);
                    com.active_model = read_optional_id(buf).map_or(-1, i32::from);
                    com.anim = read_optional_id(buf).map_or(-1, i32::from);
                    com.active_anim = read_optional_id(buf).map_or(-1, i32::from);
                    com.zoom = buf.g2() as i32;
                    com.xan = buf.g2();
                    com.yan = buf.g2();
                }
                #[cfg(since_647)]
                {
                    com.model = read_id(buf);
                    com.model_flags = buf.g1();
                    let position = com.model_flags & 1 != 0;
                    let offset = !position && com.model_flags & 2 != 0;
                    if position || offset {
                        com.model_origin_x = buf.g2s();
                        com.model_origin_y = buf.g2s();
                        if offset {
                            com.sprite_y_offset = buf.g2s();
                        }
                        com.xan = buf.g2();
                        com.yan = buf.g2();
                        com.model_zan = buf.g2();
                        com.zoom = if position {
                            buf.g2() as i32
                        } else {
                            buf.g1s() as i32
                        };
                    }
                    com.anim = read_id(buf);
                    if com.width_mode != 0 {
                        com.dynamic_width = buf.g2();
                    }
                    if com.height_mode != 0 {
                        com.dynamic_height = buf.g2();
                    }
                }
            }
            #[cfg(before_647)]
            IfComponentType::InvText => {
                com.center = buf.g1() != 0;
                com.font = buf.g1() as i32;
                com.shadowed = buf.g1() != 0;
                com.colour = buf.g4s();
                com.margin_x = buf.g2() as i32;
                com.margin_y = buf.g2() as i32;
                com.operable = buf.g1() != 0;
                com.iop = (0..5).map(|_| buf.gjstr(STRING_TERMINATOR)).collect();
            }
            #[cfg(since_647)]
            IfComponentType::Line => {
                com.line_width = buf.g1();
                com.colour = buf.g4s();
                com.line_rtl = buf.g1() == 1;
            }
            #[cfg(since_647)]
            IfComponentType::Inv | IfComponentType::InvText => {}
        }
        #[cfg(before_647)]
        {
            if matches!(com.com_type, IfComponentType::Rect | IfComponentType::Text) {
                com.colour = buf.g4s();
                com.active_colour = buf.g4s();
                com.over_colour = buf.g4s();
                #[cfg(since_245_2)]
                {
                    com.active_over_colour = buf.g4s();
                }
            }
            if com.button_type == IfButtonType::Target || com.com_type == IfComponentType::Inv {
                com.action_verb = buf.gjstr(STRING_TERMINATOR);
                com.action = buf.gjstr(STRING_TERMINATOR);
                com.action_target = buf.g2();
            }
            if matches!(
                com.button_type,
                IfButtonType::Normal
                    | IfButtonType::Toggle
                    | IfButtonType::Select
                    | IfButtonType::Pause
            ) {
                com.option = buf.gjstr(STRING_TERMINATOR);
            }
        }
        #[cfg(since_647)]
        {
            com.active_properties = buf.g3() as u32;
            loop {
                let header = buf.g1();
                if header == 0 {
                    break;
                }
                let rate = (((header as u16) << 8) | buf.g1() as u16) & 0xfff;
                com.opkeys.push(OpKey {
                    index: (header >> 4) - 1,
                    rate_duration: if rate == 0xfff { -1 } else { rate as i32 },
                    code: buf.g1s(),
                    mask: buf.g1s(),
                });
            }
            com.op_base = buf.gjstr(STRING_TERMINATOR);
            let ops = buf.g1();
            com.ops = (0..ops & 0xf)
                .map(|_| buf.gjstr(STRING_TERMINATOR))
                .collect();
            com.op_cursor_count = ops >> 4;
            com.op_cursors = (0..com.op_cursor_count.min(2))
                .map(|_| (buf.g1(), buf.g2()))
                .collect();
            com.pause_text = buf.gjstr(STRING_TERMINATOR);
            com.drag_deadzone = buf.g1();
            com.drag_deadtime = buf.g1();
            com.drag_renderbehavior = buf.g1();
            com.target_verb = buf.gjstr(STRING_TERMINATOR);
            if (com.active_properties >> 11) & 0x7f != 0 {
                com.target_object_id = read_optional_id(buf);
                com.target_enter_cursor = read_optional_id(buf);
                com.target_leave_cursor = read_optional_id(buf);
            }
            com.hooks = from_fn(|_| parse_hook(buf));
            com.transmit_lists = from_fn(|_| parse_transmitlist(buf));
        }
        com
    }

    pub fn is_root(&self) -> bool {
        self.id as i32 == self.root_layer
    }
}

#[cfg(before_647)]
fn read_optional_id(buf: &mut Packet) -> Option<u16> {
    let b = buf.g1();
    if b == 0 {
        None
    } else {
        Some(((b as u16 - 1) << 8) + buf.g1() as u16)
    }
}

#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct OpKey {
    pub index: u8,
    pub rate_duration: i32,
    pub code: i8,
    pub mask: i8,
}

/// A component's hooks, in the order its bytes hold them.
#[cfg(since_647)]
pub const HOOK_NAMES: [&str; 20] = [
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
    "onmouserepeat",
    "onclick",
    "onclickrepeat",
    "onrelease",
    "onhold",
    "ondrag",
    "ondragcomplete",
    "onscrollwheel",
    "onvarctransmit",
    "onvarcstrtransmit",
];

/// The transmit list a hook goes with, and what that list holds.
#[cfg(since_647)]
pub const fn hook_transmit_list(hook: usize) -> Option<(usize, &'static str)> {
    match hook {
        5 => Some((0, "varp")),
        6 => Some((1, "inv")),
        7 => Some((2, "stat")),
        18 => Some((3, "varc")),
        19 => Some((4, "varcstr")),
        _ => None,
    }
}

/// What a text or graphic component's colour is when it doesn't say.
#[cfg(since_647)]
pub const DEFAULT_COLOUR: i32 = 0xffffff;
#[cfg(since_647)]
pub const SIZE_MODES: [&str; 3] = ["abs", "minus", "proportion"];
#[cfg(since_647)]
pub const X_MODES: [&str; 6] = [
    "abs_left",
    "abs_centre",
    "abs_right",
    "proportion_left",
    "proportion_centre",
    "proportion_right",
];
#[cfg(since_647)]
pub const Y_MODES: [&str; 6] = [
    "abs_top",
    "abs_centre",
    "abs_bottom",
    "proportion_top",
    "proportion_centre",
    "proportion_bottom",
];
#[cfg(since_647)]
pub const TEXT_ALIGN_H: [&str; 3] = ["left", "centre", "right"];
#[cfg(since_647)]
pub const TEXT_ALIGN_V: [&str; 3] = ["top", "centre", "bottom"];
/// What a component can target, by its bit in the target mask.
#[cfg(since_647)]
pub const TARGET_MASK: [&str; 7] = ["obj", "npc", "loc", "player", "heldobj", "com", "coord"];

#[cfg(since_647)]
pub fn mode_name(modes: &[&str], mode: u8) -> String {
    modes
        .get(mode as usize)
        .map_or_else(|| format!("unknown_{mode}"), |name| (*name).to_owned())
}

#[cfg(since_647)]
pub fn mode_value(modes: &[&str], name: &str) -> Option<u8> {
    modes
        .iter()
        .position(|mode| *mode == name)
        .map(|mode| mode as u8)
        .or_else(|| name.strip_prefix("unknown_")?.parse().ok())
}

#[cfg(since_647)]
fn parse_hook(buf: &mut Packet) -> Option<Vec<HookValue>> {
    let count = buf.g1();
    if count == 0 {
        return None;
    }
    let mut args = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let is_str = buf.g1();
        if is_str == 0 {
            args.push(HookValue::Int(buf.g4s()));
        } else {
            args.push(HookValue::String(buf.gjstr(STRING_TERMINATOR)));
        }
    }
    Some(args)
}

#[cfg(since_647)]
fn parse_transmitlist(buf: &mut Packet) -> Option<Vec<i32>> {
    let count = buf.g1();
    if count == 0 {
        return None;
    }
    let mut ids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        ids.push(buf.g4s());
    }
    Some(ids)
}

pub struct IfTypeProvider {
    pub names: HashMap<Box<str>, i32>,
    pub types: HashMap<i32, Box<IfType>>,
}

impl IfTypeProvider {
    #[cfg(before_647)]
    pub fn from_bytes(dat: &[u8]) -> IfTypeProvider {
        let mut buf = Packet::from(dat.to_vec());
        let count = buf.g2() as usize;

        let mut names = HashMap::new();
        let mut types = HashMap::with_capacity(count);

        let mut root_layer: i32 = -1;
        // Each layer's children and where it puts them: (layer, child, x, y).
        let mut placed: Vec<(u16, u16, i16, i16)> = Vec::new();
        let mut scrolling = HashSet::new();

        while buf.remaining() > 0 {
            let mut id = buf.g2();
            if id == 0xFFFF {
                root_layer = buf.g2() as i32;
                id = buf.g2();
            }

            let com = IfTypeRaw::decode(&mut buf, id, root_layer, true);

            if let Some(name) = &com.com_name
                && !name.is_empty()
            {
                names.insert(name.clone(), id as i32);
            }

            if com.com_type == IfComponentType::Layer && com.scroll > com.height {
                scrolling.insert(id);
            }
            for &(child, x, y) in &com.children {
                placed.push((id, child, x, y));
            }
            if (id as usize) < count {
                types.insert(id as i32, Box::new(IfType::from(com)));
            }
        }

        // Each component's place: its own in its layer, plus its layers'.
        let parents: HashMap<u16, (u16, i16, i16)> = placed
            .into_iter()
            .map(|(layer, child, x, y)| (child, (layer, x, y)))
            .collect();
        for (&id, com) in &mut types {
            let mut at = id as u16;
            // A layer can't hold itself, but stop all the same if one does.
            for _ in 0..32 {
                let Some(&(layer, x, y)) = parents.get(&at) else {
                    break;
                };
                com.x += x;
                com.y += y;
                com.scrolls |= scrolling.contains(&layer);
                at = layer;
            }
        }

        IfTypeProvider { names, types }
    }

    #[cfg(since_647)]
    pub fn from_cache(cache: &PackedCache) -> Result<Self> {
        let table = cache
            .reference_table(archive::INTERFACES)
            .context("Missing interfaces archive")?;
        let mut types = HashMap::new();
        for group in &table.groups {
            let data = cache
                .read_group_raw_decompressed(archive::INTERFACES, group.id)?
                .with_context(|| format!("Missing interface {}", group.id))?;
            for entry in split_group_entries(data, &group.files) {
                let mut buf = Packet::from(entry.data().to_vec());
                let raw = IfTypeRaw::decode(&mut buf, entry.id as u16, group.id as i32);
                let com = ((group.id as i32) << 16) | entry.id as i32;
                types.insert(com, Box::new(IfType::from(raw)));
            }
        }
        Ok(Self {
            names: HashMap::new(),
            types,
        })
    }

    pub fn get_by_id(&self, id: impl Into<i32>) -> Option<&IfType> {
        self.types.get(&id.into()).map(Box::as_ref)
    }

    pub fn get_by_debugname(&self, name: &str) -> Option<&IfType> {
        self.names.get(name).and_then(|&id| self.get_by_id(id))
    }

    pub fn count(&self) -> usize {
        self.types.len()
    }
}
