#[cfg(since_530)]
use crate::archive;
#[cfg(since_530)]
use crate::cache::decode::{read_big_id, read_optional_id};
#[cfg(since_530)]
use crate::cache::js5::PackedCache;
use crate::types::{IfButtonType, IfComponentType};
#[cfg(since_530)]
use anyhow::{Context, Result};
use rs_io::Packet;
#[cfg(since_530)]
use rs_io::js5::unpack_group_data;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;
#[cfg(since_530)]
use runec::clientscript::HookArg;
#[cfg(since_530)]
use std::array::from_fn;
use std::collections::HashMap;
#[cfg(before_530)]
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
    pub target_mask: u16,
    pub x: i16,
    pub y: i16,
    pub scrolls: bool,
    pub layer: bool,
    #[cfg(since_530)]
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
            target_mask: raw.target_mask,
            x: 0,
            y: 0,
            scrolls: false,
            layer: raw.com_type == IfComponentType::Layer,
            #[cfg(since_530)]
            active_properties: raw.active_properties,
        }
    }
}

#[derive(Default)]
pub struct IfTypeRaw {
    pub id: u16,
    pub root_layer: i32,
    pub overlay: bool,
    #[cfg(since_530)]
    pub version: u8,
    #[cfg(all(since_530, before_647))]
    pub if1: bool,
    pub com_type: IfComponentType,
    pub com_name: Option<Box<str>>,
    pub button_type: IfButtonType,
    pub client_code: u16,
    #[cfg(since_530)]
    pub x: i16,
    #[cfg(since_530)]
    pub y: i16,
    pub width: u16,
    pub height: u16,
    #[cfg(since_244)]
    pub trans: u8,
    #[cfg(since_530)]
    pub width_mode: i8,
    #[cfg(since_530)]
    pub height_mode: i8,
    #[cfg(since_530)]
    pub x_mode: i8,
    #[cfg(since_530)]
    pub y_mode: i8,
    pub over_layer: Option<u16>,
    #[cfg(since_530)]
    pub layer: Option<u16>,
    #[cfg(since_530)]
    pub flags: u8,
    pub comparators: Vec<(u8, i16)>,
    pub scripts: Vec<Vec<u16>>,
    #[cfg(since_530)]
    pub scroll_width: u16,
    pub scroll_height: u16,
    pub hide: bool,
    pub children: Vec<(u16, i16, i16)>,
    #[cfg(since_530)]
    pub no_click_through: bool,
    #[cfg(all(since_530, before_647))]
    pub unknown_1: (u16, u8),
    pub draggable: bool,
    pub operable: bool,
    pub usable: bool,
    #[cfg(since_245_2)]
    pub swappable: bool,
    pub margin_x: i32,
    pub margin_y: i32,
    #[cfg(before_530)]
    pub slots: Vec<Option<(i16, i16, String)>>,
    #[cfg(since_530)]
    pub slots: Vec<Option<(i16, i16, i32)>>,
    pub iop: Vec<String>,
    pub fill: bool,
    pub center: bool,
    pub font: TypeId,
    pub shadowed: bool,
    pub text: String,
    pub active_text: String,
    #[cfg(since_530)]
    pub line_spacing: u8,
    #[cfg(since_530)]
    pub text_halign: u8,
    #[cfg(since_530)]
    pub text_valign: u8,
    pub colour: i32,
    pub active_colour: i32,
    pub over_colour: i32,
    #[cfg(since_245_2)]
    pub active_over_colour: i32,
    #[cfg(before_530)]
    pub graphic: String,
    #[cfg(since_530)]
    pub graphic: Option<TypeId>,
    #[cfg(before_530)]
    pub active_graphic: String,
    #[cfg(since_530)]
    pub active_graphic: Option<TypeId>,
    #[cfg(since_530)]
    pub angle_2d: u16,
    #[cfg(since_530)]
    pub tiling: bool,
    #[cfg(since_530)]
    pub alpha: bool,
    #[cfg(since_530)]
    pub outline: u8,
    #[cfg(since_530)]
    pub graphic_shadow: i32,
    #[cfg(since_530)]
    pub vflip: bool,
    #[cfg(since_530)]
    pub hflip: bool,
    pub model: TypeId,
    pub active_model: TypeId,
    #[cfg(since_647)]
    pub model_flags: u8,
    #[cfg(since_530)]
    pub model_origin_x: i16,
    #[cfg(since_530)]
    pub model_origin_y: i16,
    #[cfg(since_647)]
    pub model_origin_z: i16,
    pub xan: u16,
    pub yan: u16,
    #[cfg(since_530)]
    pub model_zan: u16,
    pub zoom: i32,
    pub anim: TypeId,
    pub active_anim: TypeId,
    #[cfg(all(since_530, before_647))]
    pub model_ortho: bool,
    #[cfg(all(since_530, before_647))]
    pub model_unknown: (u16, u16, u8),
    #[cfg(since_530)]
    pub dynamic_width: u16,
    #[cfg(since_530)]
    pub dynamic_height: u16,
    #[cfg(since_530)]
    pub line_width: u8,
    #[cfg(since_530)]
    pub line_rtl: bool,
    pub action_verb: String,
    pub action: String,
    pub target_mask: u16,
    pub option: String,
    #[cfg(since_530)]
    pub active_properties: u32,
    #[cfg(since_530)]
    pub opkeys: Vec<OpKey>,
    #[cfg(since_530)]
    pub op_base: String,
    #[cfg(since_530)]
    pub ops: Vec<String>,
    #[cfg(since_530)]
    pub op_cursor_count: u8,
    #[cfg(since_530)]
    pub op_cursors: Vec<(u8, TypeId)>,
    #[cfg(since_647)]
    pub pause_text: String,
    #[cfg(since_530)]
    pub drag_deadzone: u8,
    #[cfg(since_530)]
    pub drag_deadtime: u8,
    #[cfg(since_530)]
    pub drag_render_behaviour: u8,
    #[cfg(since_530)]
    pub target_verb: String,
    #[cfg(since_530)]
    pub target_object_id: Option<TypeId>,
    #[cfg(since_530)]
    pub valid_target_cursor: Option<TypeId>,
    #[cfg(since_530)]
    pub invalid_target_cursor: Option<TypeId>,
    #[cfg(since_742)]
    pub click_mask: bool,
    #[cfg(since_742)]
    pub font_mono: bool,
    #[cfg(since_742)]
    pub text_max_lines: u8,
    #[cfg(since_742)]
    pub mouse_over_cursor: Option<TypeId>,
    #[cfg(since_742)]
    pub int_params: Vec<(u32, i32)>,
    #[cfg(since_742)]
    pub string_params: Vec<(u32, String)>,
    #[cfg(since_530)]
    pub hooks: [Option<Vec<HookArg>>; HOOK_NAMES.len()],
    #[cfg(since_530)]
    pub transmit_lists: [Option<Vec<i32>>; 5],
}

impl IfTypeRaw {
    pub fn decode(
        buf: &mut Packet,
        id: u16,
        root_layer: i32,
        #[cfg(before_530)] server: bool,
    ) -> Self {
        let mut com = Self {
            id,
            root_layer,
            #[cfg(since_530)]
            line_width: 1,
            ..Self::default()
        };
        #[cfg(all(since_530, before_647))]
        if buf.data[buf.pos] != u8::MAX {
            com.decode_if1(buf);
            return com;
        }
        #[cfg(before_530)]
        {
            if server {
                com.com_name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str());
                com.overlay = buf.g1() == 1;
            }
            com.com_type = IfComponentType::try_from(buf.g1()).unwrap();
            com.button_type = IfButtonType::try_from(buf.g1()).unwrap();
        }
        #[cfg(since_530)]
        {
            com.version = buf.g1();
            let kind = buf.g1();
            if kind & 0x80 != 0 {
                com.com_name = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str());
            }
            com.com_type = IfComponentType::try_from(kind & 0x7f).unwrap();
        }
        com.client_code = buf.g2();
        #[cfg(since_530)]
        {
            com.x = buf.g2s();
            com.y = buf.g2s();
        }
        com.width = buf.g2();
        com.height = buf.g2();
        #[cfg(before_530)]
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
        #[cfg(since_530)]
        {
            com.width_mode = buf.g1s();
            com.height_mode = buf.g1s();
            com.x_mode = buf.g1s();
            com.y_mode = buf.g1s();
            com.layer = Some(buf.g2()).filter(|&id| id != u16::MAX);
            com.flags = buf.g1();
            #[cfg(since_742)]
            if com.format_version() >= 0 {
                com.no_click_through = com.flags & NO_CLICK_THROUGH != 0;
                com.flags &= !NO_CLICK_THROUGH;
            }
        }
        match com.com_type {
            IfComponentType::Layer => {
                #[cfg(since_530)]
                {
                    com.scroll_width = buf.g2();
                }
                com.scroll_height = buf.g2();
                #[cfg(before_530)]
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
                #[cfg(since_530)]
                if com.version == u8::MAX {
                    com.no_click_through = buf.g1() == 1;
                }
            }
            #[cfg(before_530)]
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
            IfComponentType::Rectangle => {
                #[cfg(since_530)]
                {
                    com.colour = buf.g4s();
                }
                com.fill = buf.g1() != 0;
                #[cfg(since_530)]
                {
                    com.trans = buf.g1();
                }
            }
            IfComponentType::Text => {
                #[cfg(before_530)]
                {
                    com.center = buf.g1() != 0;
                    com.font = TypeId::new(buf.g1().into());
                    com.shadowed = buf.g1() != 0;
                }
                #[cfg(since_530)]
                {
                    com.font = read_big_id(buf);
                }
                #[cfg(since_742)]
                if com.format_version() >= 2 {
                    com.font_mono = buf.g1() == 1;
                }
                com.text = buf.gjstr(STRING_TERMINATOR);
                #[cfg(before_530)]
                {
                    com.active_text = buf.gjstr(STRING_TERMINATOR);
                }
                #[cfg(since_530)]
                {
                    com.line_spacing = buf.g1();
                    com.text_halign = buf.g1();
                    com.text_valign = buf.g1();
                    com.shadowed = buf.g1() == 1;
                    com.colour = buf.g4s();
                    #[cfg(since_647)]
                    {
                        com.trans = buf.g1();
                    }
                }
                #[cfg(since_742)]
                if com.format_version() >= 0 {
                    com.text_max_lines = buf.g1();
                }
            }
            IfComponentType::Sprite => {
                #[cfg(before_530)]
                {
                    com.graphic = buf.gjstr(STRING_TERMINATOR);
                    com.active_graphic = buf.gjstr(STRING_TERMINATOR);
                }
                #[cfg(since_530)]
                {
                    com.graphic = TypeId::from_int(buf.g4s()).non_null();
                    com.angle_2d = buf.g2();
                    let flags = buf.g1();
                    com.tiling = flags & 1 != 0;
                    com.alpha = flags & 2 != 0;
                    com.trans = buf.g1();
                    com.outline = buf.g1();
                    com.graphic_shadow = buf.g4s();
                    com.vflip = buf.g1() == 1;
                    com.hflip = buf.g1() == 1;
                    #[cfg(since_647)]
                    {
                        com.colour = buf.g4s();
                    }
                    #[cfg(since_742)]
                    if com.format_version() >= 3 {
                        com.click_mask = buf.g1() == 1;
                    }
                }
            }
            IfComponentType::Model => {
                #[cfg(before_530)]
                {
                    com.model = read_optional_id(buf).map_or(TypeId::NULL, TypeId::new);
                    com.active_model = read_optional_id(buf).map_or(TypeId::NULL, TypeId::new);
                    com.anim = read_optional_id(buf).map_or(TypeId::NULL, TypeId::new);
                    com.active_anim = read_optional_id(buf).map_or(TypeId::NULL, TypeId::new);
                    com.zoom = buf.g2() as i32;
                    com.xan = buf.g2();
                    com.yan = buf.g2();
                }
                #[cfg(since_530)]
                {
                    com.model = read_big_id(buf);
                }
                #[cfg(all(since_530, before_647))]
                {
                    com.model_origin_x = buf.g2s();
                    com.model_origin_y = buf.g2s();
                    com.xan = buf.g2();
                    com.yan = buf.g2();
                    com.model_zan = buf.g2();
                    com.zoom = buf.g2() as i32;
                    com.anim = read_big_id(buf);
                    com.model_ortho = buf.g1() == 1;
                    com.model_unknown = (buf.g2(), buf.g2(), buf.g1());
                }
                #[cfg(since_647)]
                {
                    com.model_flags = buf.g1();
                    let position = com.model_flags & 1 != 0;
                    let offset = !position && com.model_flags & 2 != 0;
                    if position || offset {
                        com.model_origin_x = buf.g2s();
                        com.model_origin_y = buf.g2s();
                        if offset {
                            com.model_origin_z = buf.g2s();
                        }
                        com.xan = buf.g2();
                        com.yan = buf.g2();
                        com.model_zan = buf.g2();
                        com.zoom = if position {
                            buf.g2() as i32
                        } else if cfg!(since_742) {
                            buf.g2s() as i32
                        } else {
                            buf.g1s() as i32
                        };
                    }
                    com.anim = read_big_id(buf);
                }
                #[cfg(since_530)]
                {
                    if com.width_mode != 0 {
                        com.dynamic_width = buf.g2();
                    }
                    if com.height_mode != 0 {
                        com.dynamic_height = buf.g2();
                    }
                }
            }
            #[cfg(before_530)]
            IfComponentType::InvText => {
                com.center = buf.g1() != 0;
                com.font = TypeId::new(buf.g1().into());
                com.shadowed = buf.g1() != 0;
                com.colour = buf.g4s();
                com.margin_x = buf.g2() as i32;
                com.margin_y = buf.g2() as i32;
                com.operable = buf.g1() != 0;
                com.iop = (0..5).map(|_| buf.gjstr(STRING_TERMINATOR)).collect();
            }
            #[cfg(since_530)]
            IfComponentType::Line => {
                com.line_width = buf.g1();
                com.colour = buf.g4s();
                com.line_rtl = buf.g1() == 1;
            }
            #[cfg(since_530)]
            IfComponentType::Inv | IfComponentType::InvText => {}
            #[cfg(all(since_530, before_647))]
            IfComponentType::Unknown1 | IfComponentType::Unknown8 => {}
        }
        #[cfg(before_530)]
        {
            if matches!(
                com.com_type,
                IfComponentType::Rectangle | IfComponentType::Text
            ) {
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
                com.target_mask = buf.g2();
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
        #[cfg(since_530)]
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
                .map(|_| (buf.g1(), TypeId::new(buf.g2())))
                .collect();
            #[cfg(since_647)]
            {
                com.pause_text = buf.gjstr(STRING_TERMINATOR);
            }
            com.drag_deadzone = buf.g1();
            com.drag_deadtime = buf.g1();
            com.drag_render_behaviour = buf.g1();
            com.target_verb = buf.gjstr(STRING_TERMINATOR);
            if (com.active_properties >> 11) & 0x7f != 0 {
                com.target_object_id = read_optional_id(buf);
                com.valid_target_cursor = read_optional_id(buf);
                com.invalid_target_cursor = read_optional_id(buf);
            }
            #[cfg(since_742)]
            if com.format_version() >= 0 {
                com.mouse_over_cursor = read_optional_id(buf);
                com.int_params = (0..buf.g1())
                    .map(|_| (buf.g3() as u32, buf.g4s()))
                    .collect();
                com.string_params = (0..buf.g1())
                    .map(|_| {
                        let key = buf.g3() as u32;
                        buf.g1();
                        (key, buf.gjstr(STRING_TERMINATOR))
                    })
                    .collect();
            }
            let version = com.version;
            com.hooks = from_fn(|hook| has_hook(version, hook).then(|| parse_hook(buf)).flatten());
            com.transmit_lists = from_fn(|_| parse_transmitlist(buf));
        }
        com
    }

    #[cfg(all(since_530, before_647))]
    fn decode_if1(&mut self, buf: &mut Packet) {
        self.if1 = true;
        self.com_type = IfComponentType::try_from(buf.g1()).unwrap();
        self.button_type = IfButtonType::try_from(buf.g1()).unwrap();
        self.client_code = buf.g2();
        self.x = buf.g2s();
        self.y = buf.g2s();
        self.width = buf.g2();
        self.height = buf.g2();
        self.trans = buf.g1();
        self.layer = Some(buf.g2()).filter(|&id| id != u16::MAX);
        self.over_layer = Some(buf.g2()).filter(|&id| id != u16::MAX);
        self.comparators = (0..buf.g1()).map(|_| (buf.g1(), buf.g2s())).collect();
        self.scripts = (0..buf.g1())
            .map(|_| (0..buf.g2()).map(|_| buf.g2()).collect())
            .collect();
        match self.com_type {
            IfComponentType::Layer => {
                self.scroll_height = buf.g2();
                self.flags = buf.g1();
            }
            IfComponentType::Unknown1 => self.unknown_1 = (buf.g2(), buf.g1()),
            IfComponentType::Inv => {
                self.draggable = buf.g1() == 1;
                self.operable = buf.g1() == 1;
                self.usable = buf.g1() == 1;
                self.swappable = buf.g1() == 1;
                self.margin_x = buf.g1() as i32;
                self.margin_y = buf.g1() as i32;
                self.slots = (0..20)
                    .map(|_| (buf.g1() == 1).then(|| (buf.g2s(), buf.g2s(), buf.g4s())))
                    .collect();
                self.iop = (0..5).map(|_| buf.gjstr(STRING_TERMINATOR)).collect();
            }
            IfComponentType::Rectangle => self.fill = buf.g1() == 1,
            _ => {}
        }
        if matches!(
            self.com_type,
            IfComponentType::Unknown1 | IfComponentType::Text
        ) {
            self.text_halign = buf.g1();
            self.text_valign = buf.g1();
            self.line_spacing = buf.g1();
            self.font = read_big_id(buf);
            self.shadowed = buf.g1() == 1;
        }
        if self.com_type == IfComponentType::Text {
            self.text = buf.gjstr(STRING_TERMINATOR);
            self.active_text = buf.gjstr(STRING_TERMINATOR);
        }
        if matches!(
            self.com_type,
            IfComponentType::Unknown1 | IfComponentType::Rectangle | IfComponentType::Text
        ) {
            self.colour = buf.g4s();
        }
        if matches!(
            self.com_type,
            IfComponentType::Rectangle | IfComponentType::Text
        ) {
            self.active_colour = buf.g4s();
            self.over_colour = buf.g4s();
            self.active_over_colour = buf.g4s();
        }
        match self.com_type {
            IfComponentType::Sprite => {
                self.graphic = TypeId::from_int(buf.g4s()).non_null();
                self.active_graphic = TypeId::from_int(buf.g4s()).non_null();
            }
            IfComponentType::Model => {
                self.model = read_big_id(buf);
                self.active_model = read_big_id(buf);
                self.anim = read_big_id(buf);
                self.active_anim = read_big_id(buf);
                self.zoom = buf.g2() as i32;
                self.xan = buf.g2();
                self.yan = buf.g2();
            }
            IfComponentType::InvText => {
                self.text_halign = buf.g1();
                self.font = read_big_id(buf);
                self.shadowed = buf.g1() == 1;
                self.colour = buf.g4s();
                self.margin_x = buf.g2s() as i32;
                self.margin_y = buf.g2s() as i32;
                self.operable = buf.g1() == 1;
                self.iop = (0..5).map(|_| buf.gjstr(STRING_TERMINATOR)).collect();
            }
            IfComponentType::Unknown8 => self.text = buf.gjstr(STRING_TERMINATOR),
            _ => {}
        }
        if self.button_type == IfButtonType::Target || self.com_type == IfComponentType::Inv {
            self.action_verb = buf.gjstr(STRING_TERMINATOR);
            self.action = buf.gjstr(STRING_TERMINATOR);
            self.target_mask = buf.g2();
        }
        if matches!(
            self.button_type,
            IfButtonType::Normal
                | IfButtonType::Toggle
                | IfButtonType::Select
                | IfButtonType::Pause
        ) {
            self.option = buf.gjstr(STRING_TERMINATOR);
        }
    }

    pub fn is_root(&self) -> bool {
        self.id as i32 == self.root_layer
    }

    /// The layout version the component's bytes follow: 255 is the old one, -1.
    #[cfg(since_530)]
    pub fn format_version(&self) -> i32 {
        if self.version == u8::MAX {
            -1
        } else {
            self.version as i32
        }
    }
}

/// The flag bit a non-legacy component keeps `noclickthrough` in.
#[cfg(since_742)]
pub const NO_CLICK_THROUGH: u8 = 2;

/// Whether a component of `version` holds the hook at `hook`.
#[cfg(since_530)]
pub fn has_hook(version: u8, hook: usize) -> bool {
    version != u8::MAX || HOOK_NAMES[hook] != "ontargetuse"
}

#[cfg(before_530)]
fn read_optional_id(buf: &mut Packet) -> Option<u16> {
    let b = buf.g1();
    if b == 0 {
        None
    } else {
        Some(((b as u16 - 1) << 8) + buf.g1() as u16)
    }
}

#[cfg(since_530)]
#[derive(Debug, Clone)]
pub struct OpKey {
    pub index: u8,
    pub rate_duration: i32,
    pub code: i8,
    pub mask: i8,
}

/// A component's hooks, in the order its bytes hold them.
#[cfg(since_530)]
pub const HOOK_NAMES: [&str; if cfg!(since_742) { 21 } else { 20 }] = [
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
    #[cfg(since_742)]
    "ontargetuse",
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
#[cfg(since_530)]
pub const fn hook_transmit_list(hook: usize) -> Option<(usize, &'static str)> {
    match hook {
        5 => Some((0, "varp")),
        6 => Some((1, "inv")),
        7 => Some((2, "stat")),
        #[cfg(before_742)]
        18 => Some((3, "varc")),
        #[cfg(before_742)]
        19 => Some((4, "varcstr")),
        #[cfg(since_742)]
        19 => Some((3, "varc")),
        #[cfg(since_742)]
        20 => Some((4, "varcstr")),
        _ => None,
    }
}

#[cfg(all(since_530, before_647))]
pub const COMPARATORS: [&str; 4] = ["eq", "lt", "gt", "neq"];

/// An if1 script op's name and its operands, by op number from 1.
#[cfg(all(since_530, before_647))]
pub const SCRIPT_OPS: [(&str, &[&str]); 20] = [
    ("stat_level", &["stat"]),
    ("stat_base_level", &["stat"]),
    ("stat_xp", &["stat"]),
    ("inv_count", &["com", "obj"]),
    ("pushvar", &["varp"]),
    ("stat_xp_remaining", &["stat"]),
    ("op7", &["varp"]),
    ("op8", &[]),
    ("op9", &[]),
    ("inv_contains", &["com", "obj"]),
    ("runenergy", &[]),
    ("runweight", &[]),
    ("testbit", &["varp", "int"]),
    ("push_varbit", &["varpbit"]),
    ("subtract", &[]),
    ("divide", &[]),
    ("multiply", &[]),
    ("coordx", &[]),
    ("coordz", &[]),
    ("push_constant", &["int"]),
];

/// What a text or graphic component's colour is when it doesn't say.
#[cfg(since_530)]
pub const DEFAULT_COLOUR: i32 = 0xffffff;
#[cfg(since_530)]
pub const SIZE_MODES: [&str; 3] = ["abs", "minus", "proportion"];
#[cfg(since_530)]
pub const X_MODES: [&str; 6] = [
    "abs_left",
    "abs_centre",
    "abs_right",
    "proportion_left",
    "proportion_centre",
    "proportion_right",
];
#[cfg(since_530)]
pub const Y_MODES: [&str; 6] = [
    "abs_top",
    "abs_centre",
    "abs_bottom",
    "proportion_top",
    "proportion_centre",
    "proportion_bottom",
];
#[cfg(since_530)]
pub const TEXT_ALIGN_H: [&str; 3] = ["left", "centre", "right"];
#[cfg(since_530)]
pub const TEXT_ALIGN_V: [&str; 3] = ["top", "centre", "bottom"];
/// What a component can target, by its bit in the target mask.
#[cfg(since_530)]
pub const TARGET_MASK: [&str; 7] = ["obj", "npc", "loc", "player", "heldobj", "com", "coord"];

#[cfg(since_530)]
pub fn mode_name(modes: &[&str], mode: u8) -> String {
    modes
        .get(mode as usize)
        .map_or_else(|| format!("unknown_{mode}"), |name| (*name).to_owned())
}

#[cfg(since_530)]
pub fn mode_value(modes: &[&str], name: &str) -> Option<u8> {
    modes
        .iter()
        .position(|mode| *mode == name)
        .map(|mode| mode as u8)
        .or_else(|| name.strip_prefix("unknown_")?.parse().ok())
}

#[cfg(since_530)]
fn parse_hook(buf: &mut Packet) -> Option<Vec<HookArg>> {
    let count = buf.g1();
    if count == 0 {
        return None;
    }
    let mut args = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let is_str = buf.g1();
        if is_str == 0 {
            args.push(HookArg::Int(buf.g4s()));
        } else {
            args.push(HookArg::String(buf.gjstr(STRING_TERMINATOR)));
        }
    }
    Some(args)
}

#[cfg(since_530)]
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
    #[cfg(before_530)]
    pub fn from_bytes(dat: &[u8]) -> IfTypeProvider {
        let mut buf = Packet::from(dat.to_vec());
        let count = buf.g2() as usize;

        let mut names = HashMap::new();
        let mut types = HashMap::with_capacity(count);

        let mut root_layer: i32 = -1;
        // Each layer's children and where it puts them: (layer, child, x, y).
        let mut placed: Vec<(u16, u16, i16, i16)> = Vec::new();
        let mut scrolling = HashSet::new();

        while buf.get_space() > 0 {
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

            if com.com_type == IfComponentType::Layer && com.scroll_height > com.height {
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

    #[cfg(since_530)]
    pub fn from_cache(cache: &PackedCache) -> Result<Self> {
        let table = cache
            .index(archive::INTERFACES)
            .context("Missing interfaces archive")?;
        let mut types = HashMap::new();
        for group in &table.groups {
            let data = cache
                .read_group_raw_decompressed(archive::INTERFACES, group.id)?
                .with_context(|| format!("Missing interface {}", group.id))?;
            for entry in unpack_group_data(data, &group.files) {
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
