#![allow(clippy::field_reassign_with_default)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Interface archive format: component binary ↔ text.
//!
//! Archive 3 (INTERFACES) stores UI component definitions. Each group is
//! one interface; each file within the group is one component. The binary
//! format is decoded from the rev-647 client's `Component.decode()`.
//!
//! Version byte is always 0xFF in rev 647 (var2 = -1). Fields gated by
//! `if (var2 >= 0)` are never present: no custom properties, no onopt_hook,
//! no mouseover_cursor, no max_lines.

use serde::{Deserialize, Serialize};

pub const INTERFACE_ARCHIVE: u8 = 3;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceManifest {
    pub archive_id: u8,
    pub groups: Vec<InterfaceGroupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Default)]
pub struct Component {
    pub version: u8,
    pub comp_type: u8,
    pub name: Option<String>,
    pub client_code: u16,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub width_mode: i8,
    pub height_mode: i8,
    pub x_mode: i8,
    pub y_mode: i8,
    pub parent_id: u16,
    pub flags: u8,
    // Type 0 (layer)
    pub scroll_width: u16,
    pub scroll_height: u16,
    pub no_click_through: bool,
    // Type 5 (sprite)
    pub graphic: i32,
    pub angle_2d: u16,
    pub tiling: bool,
    pub alpha: bool,
    pub trans: u8,
    pub outline: u8,
    pub graphic_shadow: i32,
    pub vflip: bool,
    pub hflip: bool,
    // Type 6 (model)
    pub model: i32,
    pub model_flags: u8,
    pub model_origin_x: i16,
    pub model_origin_y: i16,
    pub model_xan: u16,
    pub model_yan: u16,
    pub model_zan: u16,
    pub model_zoom: i32,
    pub an_int_351: i16,
    pub animation: i32,
    pub an_int_401: u16,
    pub an_int_366: u16,
    // Type 4 (text)
    pub text_font: i32,
    pub text: String,
    pub an_int_404: u8,
    pub text_halign: u8,
    pub text_valign: u8,
    pub text_shadow: bool,
    // Type 3 (rect) / Type 9 (line)
    pub color: i32,
    pub fill: bool,
    pub line_width: u8,
    pub line_rtl: bool,
    // Active properties + opkeys
    pub active_properties: u32,
    pub opkeys: Vec<OpKey>,
    // Operations
    pub op_base: String,
    pub ops: Vec<String>,
    pub op_cursors: Vec<(u8, u16)>,
    pub op_cursor_count: u8,
    // Interaction
    pub pause_text: String,
    pub drag_deadzone: u8,
    pub drag_deadtime: u8,
    pub drag_renderbehavior: u8,
    pub target_verb: String,
    pub target_object_id: i32,
    pub target_enter_cursor: i32,
    pub target_leave_cursor: i32,
    // Hooks (20 total when var2 = -1)
    pub hooks: [Option<Vec<HookArg>>; 20],
    // Transmit lists (5)
    pub transmit_lists: [Option<Vec<i32>>; 5],
}

#[derive(Debug, Clone)]
pub struct OpKey {
    pub index: u8,
    pub rate_duration: i32,
    pub code: i8,
    pub mask: i8,
}

#[derive(Debug, Clone)]
pub enum HookArg {
    Int(i32),
    Str(String),
}

// ── Binary → structured ───────────────────────────────────────────────────

pub fn parse_component(data: &[u8], _interface_id: u32) -> Component {
    let mut pos = 0usize;
    let mut comp = Component::default();

    comp.version = read_u8(data, &mut pos);
    let mut raw_type = read_u8(data, &mut pos);

    let var2: i32 = if comp.version == 255 {
        -1
    } else {
        comp.version as i32
    };

    if raw_type & 0x80 != 0 {
        raw_type &= 0x7F;
        comp.name = Some(read_string(data, &mut pos));
    }
    comp.comp_type = raw_type;

    comp.client_code = read_u16(data, &mut pos);
    comp.x = read_i16(data, &mut pos);
    comp.y = read_i16(data, &mut pos);
    comp.width = read_u16(data, &mut pos);
    comp.height = read_u16(data, &mut pos);
    comp.width_mode = read_i8(data, &mut pos);
    comp.height_mode = read_i8(data, &mut pos);
    comp.x_mode = read_i8(data, &mut pos);
    comp.y_mode = read_i8(data, &mut pos);

    comp.parent_id = read_u16(data, &mut pos);
    comp.flags = read_u8(data, &mut pos);

    // Type 0: layer
    if comp.comp_type == 0 {
        comp.scroll_width = read_u16(data, &mut pos);
        comp.scroll_height = read_u16(data, &mut pos);
        if var2 < 0 {
            comp.no_click_through = read_u8(data, &mut pos) == 1;
        }
    }

    // Type 5: sprite
    if comp.comp_type == 5 {
        comp.graphic = read_i32(data, &mut pos);
        comp.angle_2d = read_u16(data, &mut pos);
        let sprite_flags = read_u8(data, &mut pos);
        comp.tiling = sprite_flags & 1 != 0;
        comp.alpha = sprite_flags & 2 != 0;
        comp.trans = read_u8(data, &mut pos);
        comp.outline = read_u8(data, &mut pos);
        comp.graphic_shadow = read_i32(data, &mut pos);
        comp.vflip = read_u8(data, &mut pos) == 1;
        comp.hflip = read_u8(data, &mut pos) == 1;
        comp.color = read_i32(data, &mut pos);
    }

    // Type 6: model
    if comp.comp_type == 6 {
        comp.model = read_u16(data, &mut pos) as i32;
        if comp.model == 65535 {
            comp.model = -1;
        }
        comp.model_flags = read_u8(data, &mut pos);
        let position_set = comp.model_flags & 1 != 0;
        let a_bool_19 = comp.model_flags & 2 != 0;
        if position_set {
            comp.model_origin_x = read_i16(data, &mut pos);
            comp.model_origin_y = read_i16(data, &mut pos);
            comp.model_xan = read_u16(data, &mut pos);
            comp.model_yan = read_u16(data, &mut pos);
            comp.model_zan = read_u16(data, &mut pos);
            comp.model_zoom = read_u16(data, &mut pos) as i32;
        } else if a_bool_19 {
            comp.model_origin_x = read_i16(data, &mut pos);
            comp.model_origin_y = read_i16(data, &mut pos);
            comp.an_int_351 = read_i16(data, &mut pos);
            comp.model_xan = read_u16(data, &mut pos);
            comp.model_yan = read_u16(data, &mut pos);
            comp.model_zan = read_u16(data, &mut pos);
            comp.model_zoom = read_i8(data, &mut pos) as i32;
        }
        comp.animation = read_u16(data, &mut pos) as i32;
        if comp.animation == 65535 {
            comp.animation = -1;
        }
        if comp.width_mode != 0 {
            comp.an_int_401 = read_u16(data, &mut pos);
        }
        if comp.height_mode != 0 {
            comp.an_int_366 = read_u16(data, &mut pos);
        }
    }

    // Type 4: text
    if comp.comp_type == 4 {
        comp.text_font = read_u16(data, &mut pos) as i32;
        if comp.text_font == 65535 {
            comp.text_font = -1;
        }
        comp.text = read_string(data, &mut pos);
        comp.an_int_404 = read_u8(data, &mut pos);
        comp.text_halign = read_u8(data, &mut pos);
        comp.text_valign = read_u8(data, &mut pos);
        comp.text_shadow = read_u8(data, &mut pos) == 1;
        comp.color = read_i32(data, &mut pos);
        comp.trans = read_u8(data, &mut pos);
    }

    // Type 3: rectangle
    if comp.comp_type == 3 {
        comp.color = read_i32(data, &mut pos);
        comp.fill = read_u8(data, &mut pos) == 1;
        comp.trans = read_u8(data, &mut pos);
    }

    // Type 9: line
    if comp.comp_type == 9 {
        comp.line_width = read_u8(data, &mut pos);
        comp.color = read_i32(data, &mut pos);
        comp.line_rtl = read_u8(data, &mut pos) == 1;
    }

    // Active properties
    comp.active_properties = read_u24(data, &mut pos);

    // Opkeys
    let mut opkey_header = read_u8(data, &mut pos);
    while opkey_header != 0 {
        let index = (opkey_header >> 4) - 1;
        let next_byte = read_u8(data, &mut pos);
        let mut rate = ((opkey_header as u32) << 8 | next_byte as u32) & 0xFFF;
        if rate == 4095 {
            rate = 0xFFFFFFFF_u32;
        } // -1 as i32
        let code = read_i8(data, &mut pos);
        let mask = read_i8(data, &mut pos);
        comp.opkeys.push(OpKey {
            index,
            rate_duration: rate as i32,
            code,
            mask,
        });
        opkey_header = read_u8(data, &mut pos);
    }

    // Operations
    comp.op_base = read_string(data, &mut pos);
    let ops_packed = read_u8(data, &mut pos);
    let op_count = ops_packed & 0xF;
    comp.op_cursor_count = ops_packed >> 4;
    for _ in 0..op_count {
        comp.ops.push(read_string(data, &mut pos));
    }
    if comp.op_cursor_count > 0 {
        let idx = read_u8(data, &mut pos);
        let cursor = read_u16(data, &mut pos);
        comp.op_cursors.push((idx, cursor));
    }
    if comp.op_cursor_count > 1 {
        let idx = read_u8(data, &mut pos);
        let cursor = read_u16(data, &mut pos);
        comp.op_cursors.push((idx, cursor));
    }

    // Interaction
    comp.pause_text = read_string(data, &mut pos);
    comp.drag_deadzone = read_u8(data, &mut pos);
    comp.drag_deadtime = read_u8(data, &mut pos);
    comp.drag_renderbehavior = read_u8(data, &mut pos);
    comp.target_verb = read_string(data, &mut pos);

    // Target cursors (conditional on active_properties bits 11-17)
    let target_mask = (comp.active_properties >> 11) & 0x7F;
    comp.target_object_id = -1;
    comp.target_enter_cursor = -1;
    comp.target_leave_cursor = -1;
    if target_mask != 0 {
        let v = read_u16(data, &mut pos);
        comp.target_object_id = if v == 65535 { -1 } else { v as i32 };
        let v = read_u16(data, &mut pos);
        comp.target_enter_cursor = if v == 65535 { -1 } else { v as i32 };
        let v = read_u16(data, &mut pos);
        comp.target_leave_cursor = if v == 65535 { -1 } else { v as i32 };
    }

    // Hooks (20 when var2 = -1: no onopt_hook)
    for i in 0..20 {
        comp.hooks[i] = parse_hook(data, &mut pos);
    }

    // Transmit lists (5)
    for i in 0..5 {
        comp.transmit_lists[i] = parse_transmitlist(data, &mut pos);
    }

    comp
}

fn parse_hook(data: &[u8], pos: &mut usize) -> Option<Vec<HookArg>> {
    let count = read_u8(data, pos);
    if count == 0 {
        return None;
    }
    let mut args = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let is_str = read_u8(data, pos);
        if is_str == 0 {
            args.push(HookArg::Int(read_i32(data, pos)));
        } else {
            args.push(HookArg::Str(read_string(data, pos)));
        }
    }
    Some(args)
}

fn parse_transmitlist(data: &[u8], pos: &mut usize) -> Option<Vec<i32>> {
    let count = read_u8(data, pos);
    if count == 0 {
        return None;
    }
    let mut ids = Vec::with_capacity(count as usize);
    for _ in 0..count {
        ids.push(read_i32(data, pos));
    }
    Some(ids)
}

// ── Structured → binary ───────────────────────────────────────────────────

pub fn encode_component(comp: &Component, _interface_id: u32) -> Vec<u8> {
    let mut out = Vec::new();

    out.push(comp.version);
    let mut type_byte = comp.comp_type;
    if comp.name.is_some() {
        type_byte |= 0x80;
    }
    out.push(type_byte);
    if let Some(ref name) = comp.name {
        write_string(&mut out, name);
    }

    write_u16(&mut out, comp.client_code);
    write_i16(&mut out, comp.x);
    write_i16(&mut out, comp.y);
    write_u16(&mut out, comp.width);
    write_u16(&mut out, comp.height);
    out.push(comp.width_mode as u8);
    out.push(comp.height_mode as u8);
    out.push(comp.x_mode as u8);
    out.push(comp.y_mode as u8);

    write_u16(&mut out, comp.parent_id);
    out.push(comp.flags);

    let var2: i32 = if comp.version == 255 {
        -1
    } else {
        comp.version as i32
    };

    if comp.comp_type == 0 {
        write_u16(&mut out, comp.scroll_width);
        write_u16(&mut out, comp.scroll_height);
        if var2 < 0 {
            out.push(if comp.no_click_through { 1 } else { 0 });
        }
    }

    if comp.comp_type == 5 {
        write_i32(&mut out, comp.graphic);
        write_u16(&mut out, comp.angle_2d);
        let sf = (if comp.tiling { 1u8 } else { 0 }) | (if comp.alpha { 2 } else { 0 });
        out.push(sf);
        out.push(comp.trans);
        out.push(comp.outline);
        write_i32(&mut out, comp.graphic_shadow);
        out.push(if comp.vflip { 1 } else { 0 });
        out.push(if comp.hflip { 1 } else { 0 });
        write_i32(&mut out, comp.color);
    }

    if comp.comp_type == 6 {
        let model_u16 = if comp.model == -1 {
            65535u16
        } else {
            comp.model as u16
        };
        write_u16(&mut out, model_u16);
        out.push(comp.model_flags);
        let position_set = comp.model_flags & 1 != 0;
        let a_bool_19 = comp.model_flags & 2 != 0;
        if position_set {
            write_i16(&mut out, comp.model_origin_x);
            write_i16(&mut out, comp.model_origin_y);
            write_u16(&mut out, comp.model_xan);
            write_u16(&mut out, comp.model_yan);
            write_u16(&mut out, comp.model_zan);
            write_u16(&mut out, comp.model_zoom as u16);
        } else if a_bool_19 {
            write_i16(&mut out, comp.model_origin_x);
            write_i16(&mut out, comp.model_origin_y);
            write_i16(&mut out, comp.an_int_351);
            write_u16(&mut out, comp.model_xan);
            write_u16(&mut out, comp.model_yan);
            write_u16(&mut out, comp.model_zan);
            out.push(comp.model_zoom as u8);
        }
        let anim_u16 = if comp.animation == -1 {
            65535u16
        } else {
            comp.animation as u16
        };
        write_u16(&mut out, anim_u16);
        if comp.width_mode != 0 {
            write_u16(&mut out, comp.an_int_401);
        }
        if comp.height_mode != 0 {
            write_u16(&mut out, comp.an_int_366);
        }
    }

    if comp.comp_type == 4 {
        let font_u16 = if comp.text_font == -1 {
            65535u16
        } else {
            comp.text_font as u16
        };
        write_u16(&mut out, font_u16);
        write_string(&mut out, &comp.text);
        out.push(comp.an_int_404);
        out.push(comp.text_halign);
        out.push(comp.text_valign);
        out.push(if comp.text_shadow { 1 } else { 0 });
        write_i32(&mut out, comp.color);
        out.push(comp.trans);
    }

    if comp.comp_type == 3 {
        write_i32(&mut out, comp.color);
        out.push(if comp.fill { 1 } else { 0 });
        out.push(comp.trans);
    }

    if comp.comp_type == 9 {
        out.push(comp.line_width);
        write_i32(&mut out, comp.color);
        out.push(if comp.line_rtl { 1 } else { 0 });
    }

    // Active properties (3 bytes)
    write_u24(&mut out, comp.active_properties);

    // Opkeys: header byte = (index+1)<<4 | rate_hi_nibble, next byte = rate_lo
    for opkey in &comp.opkeys {
        let rate = if opkey.rate_duration == -1 {
            4095u32
        } else {
            opkey.rate_duration as u32 & 0xFFF
        };
        let header = ((opkey.index + 1) << 4) | ((rate >> 8) as u8 & 0xF);
        let next = (rate & 0xFF) as u8;
        out.push(header);
        out.push(next);
        out.push(opkey.code as u8);
        out.push(opkey.mask as u8);
    }
    out.push(0); // opkey terminator

    // Operations
    write_string(&mut out, &comp.op_base);
    let ops_packed = (comp.ops.len() as u8 & 0xF) | (comp.op_cursor_count << 4);
    out.push(ops_packed);
    for op in &comp.ops {
        write_string(&mut out, op);
    }
    if comp.op_cursor_count > 0 && !comp.op_cursors.is_empty() {
        out.push(comp.op_cursors[0].0);
        write_u16(&mut out, comp.op_cursors[0].1);
    }
    if comp.op_cursor_count > 1 && comp.op_cursors.len() > 1 {
        out.push(comp.op_cursors[1].0);
        write_u16(&mut out, comp.op_cursors[1].1);
    }

    // Interaction
    write_string(&mut out, &comp.pause_text);
    out.push(comp.drag_deadzone);
    out.push(comp.drag_deadtime);
    out.push(comp.drag_renderbehavior);
    write_string(&mut out, &comp.target_verb);

    let target_mask = (comp.active_properties >> 11) & 0x7F;
    if target_mask != 0 {
        let v = if comp.target_object_id == -1 {
            65535u16
        } else {
            comp.target_object_id as u16
        };
        write_u16(&mut out, v);
        let v = if comp.target_enter_cursor == -1 {
            65535u16
        } else {
            comp.target_enter_cursor as u16
        };
        write_u16(&mut out, v);
        let v = if comp.target_leave_cursor == -1 {
            65535u16
        } else {
            comp.target_leave_cursor as u16
        };
        write_u16(&mut out, v);
    }

    // Hooks (20)
    for i in 0..20 {
        encode_hook(&mut out, &comp.hooks[i]);
    }

    // Transmit lists (5)
    for i in 0..5 {
        encode_transmitlist(&mut out, &comp.transmit_lists[i]);
    }

    out
}

fn encode_hook(out: &mut Vec<u8>, hook: &Option<Vec<HookArg>>) {
    match hook {
        None => out.push(0),
        Some(args) => {
            out.push(args.len() as u8);
            for arg in args {
                match arg {
                    HookArg::Int(v) => {
                        out.push(0);
                        write_i32(out, *v);
                    }
                    HookArg::Str(s) => {
                        out.push(1);
                        write_string(out, s);
                    }
                }
            }
        }
    }
}

fn encode_transmitlist(out: &mut Vec<u8>, list: &Option<Vec<i32>>) {
    match list {
        None => out.push(0),
        Some(ids) => {
            out.push(ids.len() as u8);
            for &id in ids {
                write_i32(out, id);
            }
        }
    }
}

// ── Text emitter/parser ────────────────────────────────────────────────────

pub fn emit_component_text(comp: &Component, comp_index: u32) -> String {
    let mut out = String::new();
    let type_name = match comp.comp_type {
        0 => "layer",
        3 => "rect",
        4 => "text",
        5 => "graphic",
        6 => "model",
        9 => "line",
        t => {
            out.push_str(&format!("[com_{}]\ntype={}\n", comp_index, t));
            return out;
        }
    };

    out.push_str(&format!("[com_{}]\n", comp_index));
    out.push_str(&format!("type={}\n", type_name));
    if let Some(ref name) = comp.name {
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

    match comp.comp_type {
        0 => {
            if comp.scroll_width != 0 {
                out.push_str(&format!("scroll_width={}\n", comp.scroll_width));
            }
            if comp.scroll_height != 0 {
                out.push_str(&format!("scroll_height={}\n", comp.scroll_height));
            }
            if comp.no_click_through {
                out.push_str("no_click_through=yes\n");
            }
        }
        5 => {
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
            if comp.color != 0 {
                out.push_str(&format!("color={}\n", comp.color));
            }
        }
        6 => {
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
            if comp.model_xan != 0 {
                out.push_str(&format!("model_xan={}\n", comp.model_xan));
            }
            if comp.model_yan != 0 {
                out.push_str(&format!("model_yan={}\n", comp.model_yan));
            }
            if comp.model_zan != 0 {
                out.push_str(&format!("model_zan={}\n", comp.model_zan));
            }
            if comp.model_zoom != 0 {
                out.push_str(&format!("model_zoom={}\n", comp.model_zoom));
            }
            if comp.an_int_351 != 0 {
                out.push_str(&format!("sprite_y_offset={}\n", comp.an_int_351));
            }
            if comp.animation != 0 {
                out.push_str(&format!("animation={}\n", comp.animation));
            }
            if comp.an_int_401 != 0 {
                out.push_str(&format!("dynamic_width={}\n", comp.an_int_401));
            }
            if comp.an_int_366 != 0 {
                out.push_str(&format!("dynamic_height={}\n", comp.an_int_366));
            }
        }
        4 => {
            out.push_str(&format!("text_font={}\n", comp.text_font));
            if !comp.text.is_empty() {
                out.push_str(&format!("text={}\n", comp.text));
            }
            if comp.an_int_404 != 0 {
                out.push_str(&format!("text_style={}\n", comp.an_int_404));
            }
            if comp.text_halign != 0 {
                out.push_str(&format!("text_halign={}\n", comp.text_halign));
            }
            if comp.text_valign != 0 {
                out.push_str(&format!("text_valign={}\n", comp.text_valign));
            }
            if comp.text_shadow {
                out.push_str("text_shadow=yes\n");
            }
            if comp.color != 0 {
                out.push_str(&format!("color={}\n", comp.color));
            }
            if comp.trans != 0 {
                out.push_str(&format!("trans={}\n", comp.trans));
            }
        }
        3 => {
            if comp.color != 0 {
                out.push_str(&format!("color={}\n", comp.color));
            }
            if comp.fill {
                out.push_str("fill=yes\n");
            }
            if comp.trans != 0 {
                out.push_str(&format!("trans={}\n", comp.trans));
            }
        }
        9 => {
            if comp.line_width != 0 {
                out.push_str(&format!("line_width={}\n", comp.line_width));
            }
            if comp.color != 0 {
                out.push_str(&format!("color={}\n", comp.color));
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

pub fn parse_interface_text(
    text: &str,
    comp_names: Option<&std::collections::HashMap<String, u32>>,
) -> Vec<(u32, Component)> {
    const HOOK_NAMES: [&str; 20] = [
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

    let mut components = Vec::new();
    let mut current: Option<(u32, Component)> = None;

    for line in text.lines() {
        let comp_id = if let Some(header) = line.strip_prefix("[com_") {
            let idx_str = header.strip_suffix(']').unwrap_or(header);
            idx_str.parse::<u32>().ok()
        } else if let Some(names) = comp_names {
            line.strip_prefix('[')
                .and_then(|s| s.strip_suffix(']'))
                .and_then(|name| names.get(name).copied())
        } else {
            None
        };

        if let Some(idx) = comp_id {
            if let Some(prev) = current.take() {
                components.push(prev);
            }
            let mut comp = Component::default();
            comp.version = 255;
            comp.parent_id = 65535;
            comp.target_object_id = -1;
            comp.target_enter_cursor = -1;
            comp.target_leave_cursor = -1;
            current = Some((idx, comp));
            continue;
        }

        let Some((_, ref mut comp)) = current else {
            continue;
        };
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();

        match key {
            "type" => {
                comp.comp_type = match val.trim() {
                    "layer" => 0,
                    "rect" => 3,
                    "text" => 4,
                    "graphic" => 5,
                    "model" => 6,
                    "line" => 9,
                    other => other.parse().unwrap_or(0),
                };
            }
            "name" => comp.name = Some(val.to_string()),
            "client_code" => comp.client_code = val.trim().parse().unwrap_or(0),
            "x" => comp.x = val.trim().parse().unwrap_or(0),
            "y" => comp.y = val.trim().parse().unwrap_or(0),
            "width" => comp.width = val.trim().parse().unwrap_or(0),
            "height" => comp.height = val.trim().parse().unwrap_or(0),
            "width_mode" => comp.width_mode = val.trim().parse().unwrap_or(0),
            "height_mode" => comp.height_mode = val.trim().parse().unwrap_or(0),
            "x_mode" => comp.x_mode = val.trim().parse().unwrap_or(0),
            "y_mode" => comp.y_mode = val.trim().parse().unwrap_or(0),
            "parent_id" => comp.parent_id = val.trim().parse().unwrap_or(65535),
            "flags" => comp.flags = val.trim().parse().unwrap_or(0),
            "scroll_width" => comp.scroll_width = val.trim().parse().unwrap_or(0),
            "scroll_height" => comp.scroll_height = val.trim().parse().unwrap_or(0),
            "no_click_through" => comp.no_click_through = val.trim() == "yes",
            "graphic" => comp.graphic = val.trim().parse().unwrap_or(0),
            "angle_2d" => comp.angle_2d = val.trim().parse().unwrap_or(0),
            "tiling" => comp.tiling = val.trim() == "yes",
            "alpha" => comp.alpha = val.trim() == "yes",
            "trans" => comp.trans = val.trim().parse().unwrap_or(0),
            "outline" => comp.outline = val.trim().parse().unwrap_or(0),
            "graphic_shadow" => comp.graphic_shadow = val.trim().parse().unwrap_or(0),
            "vflip" => comp.vflip = val.trim() == "yes",
            "hflip" => comp.hflip = val.trim() == "yes",
            "model" => comp.model = val.trim().parse().unwrap_or(-1),
            "model_flags" => comp.model_flags = val.trim().parse().unwrap_or(0),
            "model_origin_x" => comp.model_origin_x = val.trim().parse().unwrap_or(0),
            "model_origin_y" => comp.model_origin_y = val.trim().parse().unwrap_or(0),
            "model_xan" => comp.model_xan = val.trim().parse().unwrap_or(0),
            "model_yan" => comp.model_yan = val.trim().parse().unwrap_or(0),
            "model_zan" => comp.model_zan = val.trim().parse().unwrap_or(0),
            "model_zoom" => comp.model_zoom = val.trim().parse().unwrap_or(0),
            "sprite_y_offset" => comp.an_int_351 = val.trim().parse().unwrap_or(0),
            "animation" => comp.animation = val.trim().parse().unwrap_or(0),
            "dynamic_width" => comp.an_int_401 = val.trim().parse().unwrap_or(0),
            "dynamic_height" => comp.an_int_366 = val.trim().parse().unwrap_or(0),
            "text_font" => comp.text_font = val.trim().parse().unwrap_or(-1),
            "text" => comp.text = val.to_string(),
            "text_style" => comp.an_int_404 = val.trim().parse().unwrap_or(0),
            "text_halign" => comp.text_halign = val.trim().parse().unwrap_or(0),
            "text_valign" => comp.text_valign = val.trim().parse().unwrap_or(0),
            "text_shadow" => comp.text_shadow = val.trim() == "yes",
            "color" => comp.color = val.trim().parse().unwrap_or(0),
            "fill" => comp.fill = val.trim() == "yes",
            "line_width" => comp.line_width = val.trim().parse().unwrap_or(0),
            "line_rtl" => comp.line_rtl = val.trim() == "yes",
            "active_properties" => comp.active_properties = val.trim().parse().unwrap_or(0),
            "opkey" => {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 4 {
                    comp.opkeys.push(OpKey {
                        index: parts[0].trim().parse().unwrap_or(0),
                        rate_duration: parts[1].trim().parse().unwrap_or(0),
                        code: parts[2].trim().parse().unwrap_or(0),
                        mask: parts[3].trim().parse().unwrap_or(0),
                    });
                }
            }
            "op_base" => comp.op_base = val.to_string(),
            "op_cursor_count" => comp.op_cursor_count = val.trim().parse().unwrap_or(0),
            "op_cursor" => {
                let parts: Vec<&str> = val.split(',').collect();
                if parts.len() == 2 {
                    comp.op_cursors.push((
                        parts[0].trim().parse().unwrap_or(0),
                        parts[1].trim().parse().unwrap_or(0),
                    ));
                }
            }
            "pause_text" => comp.pause_text = val.to_string(),
            "drag_deadzone" => comp.drag_deadzone = val.trim().parse().unwrap_or(0),
            "drag_deadtime" => comp.drag_deadtime = val.trim().parse().unwrap_or(0),
            "drag_renderbehavior" => comp.drag_renderbehavior = val.trim().parse().unwrap_or(0),
            "target_verb" => comp.target_verb = val.to_string(),
            "target_object_id" => comp.target_object_id = val.trim().parse().unwrap_or(-1),
            "target_enter_cursor" => comp.target_enter_cursor = val.trim().parse().unwrap_or(-1),
            "target_leave_cursor" => comp.target_leave_cursor = val.trim().parse().unwrap_or(-1),
            "vartransmit_list" => {
                comp.transmit_lists[0] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "invtransmit_list" => {
                comp.transmit_lists[1] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "stattransmit_list" => {
                comp.transmit_lists[2] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "varctransmit_list" => {
                comp.transmit_lists[3] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            "varcstrtransmit_list" => {
                comp.transmit_lists[4] = Some(
                    val.split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect(),
                );
            }
            k => {
                if let Some(n_str) = k.strip_prefix("op") {
                    if !n_str.is_empty() && n_str.chars().all(|c| c.is_ascii_digit()) {
                        if let Ok(n) = n_str.parse::<usize>() {
                            if n >= 1 {
                                let idx = n - 1;
                                while comp.ops.len() <= idx {
                                    comp.ops.push(String::new());
                                }
                                comp.ops[idx] = val.to_string();
                            }
                        }
                    }
                } else if let Some(hi) = HOOK_NAMES.iter().position(|&name| name == k) {
                    comp.hooks[hi] = parse_hook_text(val);
                }
            }
        }
    }

    if let Some(prev) = current {
        components.push(prev);
    }

    components
}

fn parse_hook_text(val: &str) -> Option<Vec<HookArg>> {
    let val = val.trim();
    if val.is_empty() {
        return None;
    }
    let mut args = Vec::new();
    let mut chars = val.chars().peekable();

    while chars.peek().is_some() {
        while chars.peek() == Some(&',') {
            chars.next();
        }
        match chars.peek() {
            None => break,
            Some(&'i') => {
                chars.next();
                let num: String = chars.by_ref().take_while(|c| *c != ',').collect();
                if let Ok(v) = num.parse::<i32>() {
                    args.push(HookArg::Int(v));
                }
            }
            Some(&'s') => {
                chars.next();
                if chars.peek() == Some(&'"') {
                    chars.next();
                    let mut s = String::new();
                    loop {
                        match chars.next() {
                            None | Some('"') => break,
                            Some('\\') => {
                                if let Some(c) = chars.next() {
                                    s.push(c);
                                }
                            }
                            Some(c) => s.push(c),
                        }
                    }
                    args.push(HookArg::Str(s));
                } else {
                    let s: String = chars.by_ref().collect();
                    args.push(HookArg::Str(s));
                    break;
                }
            }
            Some(_) => {
                chars.next();
            }
        }
    }

    if args.is_empty() { None } else { Some(args) }
}

// ── Read/write primitives ──────────────────────────────────────────────────

fn read_u8(data: &[u8], pos: &mut usize) -> u8 {
    let v = data.get(*pos).copied().unwrap_or(0);
    *pos += 1;
    v
}

fn read_i8(data: &[u8], pos: &mut usize) -> i8 {
    read_u8(data, pos) as i8
}

fn read_u16(data: &[u8], pos: &mut usize) -> u16 {
    let hi = data.get(*pos).copied().unwrap_or(0) as u16;
    let lo = data.get(*pos + 1).copied().unwrap_or(0) as u16;
    *pos += 2;
    (hi << 8) | lo
}

fn read_i16(data: &[u8], pos: &mut usize) -> i16 {
    read_u16(data, pos) as i16
}

fn read_u24(data: &[u8], pos: &mut usize) -> u32 {
    let a = data.get(*pos).copied().unwrap_or(0) as u32;
    let b = data.get(*pos + 1).copied().unwrap_or(0) as u32;
    let c = data.get(*pos + 2).copied().unwrap_or(0) as u32;
    *pos += 3;
    (a << 16) | (b << 8) | c
}

fn read_i32(data: &[u8], pos: &mut usize) -> i32 {
    let a = data.get(*pos).copied().unwrap_or(0) as u32;
    let b = data.get(*pos + 1).copied().unwrap_or(0) as u32;
    let c = data.get(*pos + 2).copied().unwrap_or(0) as u32;
    let d = data.get(*pos + 3).copied().unwrap_or(0) as u32;
    *pos += 4;
    ((a << 24) | (b << 16) | (c << 8) | d) as i32
}

fn read_string(data: &[u8], pos: &mut usize) -> String {
    let start = *pos;
    while *pos < data.len() && data[*pos] != 0 {
        *pos += 1;
    }
    let s = rs_io::cp1252::decode_string(&data[start..*pos]);
    if *pos < data.len() {
        *pos += 1;
    }
    s
}

fn write_u16(out: &mut Vec<u8>, v: u16) {
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

fn write_i16(out: &mut Vec<u8>, v: i16) {
    write_u16(out, v as u16);
}

fn write_u24(out: &mut Vec<u8>, v: u32) {
    out.push((v >> 16) as u8);
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

fn write_i32(out: &mut Vec<u8>, v: i32) {
    let v = v as u32;
    out.push((v >> 24) as u8);
    out.push((v >> 16) as u8);
    out.push((v >> 8) as u8);
    out.push(v as u8);
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&rs_io::cp1252::encode_string(s));
    out.push(0);
}
