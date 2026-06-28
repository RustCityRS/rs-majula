use crate::engine::{ScriptEngine, ScriptPlayer, cache};
use crate::register::OpsRegistry;
use crate::util::pop_font;
use crate::{ScriptError, active_player, handlers, none};
use itoa;
use rs_pack::cache::script::*;

/// Registers string manipulation opcodes for concatenation, conversion,
/// comparison, searching, and text pagination.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Concatenation:** `APPEND`, `APPEND_NUM`, `APPEND_SIGNNUM`, `APPEND_CHAR`
/// - **Conversion:** `TOSTRING`, `LOWERCASE`
/// - **Comparison:** `COMPARE`
/// - **Searching:** `STRING_INDEXOF_CHAR`, `STRING_INDEXOF_STRING`
/// - **Substrings:** `SUBSTRING`, `STRING_LENGTH`
/// - **Conditional:** `TEXT_SWITCH`, `TEXT_GENDER`
/// - **Text pagination:** `SPLIT_INIT`, `SPLIT_GET`, `SPLIT_GETANIM`,
///   `SPLIT_LINECOUNT`, `SPLIT_PAGECOUNT`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::string::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 4500
        none!(m, APPEND_NUM => |s| {
            let b = s.pop_int();
            let mut a = s.pop_string();
            let mut buf = itoa::Buffer::new();
            a.push_str(buf.format(b));
            s.push_string(&a);
        });

        // 4501
        none!(m, APPEND => |s| {
            let b = s.pop_string();
            let mut a = s.pop_string();
            a.push_str(&b);
            s.push_string(&a);
        });

        // 4502
        none!(m, APPEND_SIGNNUM => |s| {
            let b = s.pop_int();
            let mut a = s.pop_string();
            if b >= 0 {
                a.push('+');
            }
            let mut buf = itoa::Buffer::new();
            a.push_str(buf.format(b));
            s.push_string(&a);
        });

        // 4503
        none!(m, LOWERCASE => |s| {
            let a = s.pop_string();
            s.push_string(&a.to_lowercase());
        });

        // 4504
        active_player!(m, TEXT_GENDER => |s, player| {
            let female = s.pop_string();
            let male = s.pop_string();
            if player.gender() == 0 {
                s.push_string(&male);
            } else {
                s.push_string(&female);
            }
        });

        // 4505
        none!(m, TOSTRING => |s| {
            let a = s.pop_int();
            s.push_string(itoa::Buffer::new().format(a));
        });

        // 4506
        none!(m, COMPARE => |s| {
            let b_ptr = s.peek_string(0) as *const str;
            let a_ptr = s.peek_string(1) as *const str;
            let result = unsafe { (&*a_ptr).cmp(&*b_ptr) as i32 };
            s.drop_strings(2);
            s.push_int(result);
        });

        // 4507
        none!(m, TEXT_SWITCH => |s| {
            let c = s.pop_int();
            let b = s.pop_string();
            let a = s.pop_string();
            s.push_string(if c == 1 { &a } else { &b });
        });

        // 4508
        none!(m, APPEND_CHAR => |s| {
            let a = s.pop_int();
            let mut b = s.pop_string();
            if a == -1 {
                return Err(ScriptError::Runtime("null char".to_string()));
            }
            let Some(char) = std::char::from_u32((a & 0xFFFF) as u32) else {
                return Err(ScriptError::Runtime("bad char".to_string()));
            };
            b.push(char);
            s.push_string(&b);
        });

        // 4509
        none!(m, STRING_LENGTH => |s| {
            let len = s.peek_string(0).len();
            s.drop_strings(1);
            s.push_int(len as i32);
        });

        // 4510
        none!(m, SUBSTRING => |s| {
            let c = s.pop_string();
            let b = s.pop_int() as usize;
            let a = s.pop_int() as usize;
            s.push_string(&c[a..b]);
        });

        // 4511
        none!(m, STRING_INDEXOF_CHAR => |s| {
            let b = s.pop_string();
            let a = s.pop_int();
            if a == -1 {
                return Err(ScriptError::Runtime("null char".to_string()));
            }
            let Some(char) = std::char::from_u32((a & 0xFFFF) as u32) else {
                return Err(ScriptError::Runtime("bad char".to_string()));
            };
            s.push_int(b.chars().position(|c| c == char).map_or(-1, |index| index as i32));
        });

        // 4512
        none!(m, STRING_INDEXOF_STRING => |s| {
            let b = s.pop_string();
            let a = s.pop_string();
            s.push_int(b.find(&a).map_or(-1, |index| index as i32)); // return -1 if not found.
        });


        // 4513
        none!(m, SPLIT_GET => |s| {
            let line = s.pop_int();
            let page = s.pop_int();
            let text = s
                .split_pages
                .as_deref()
                .and_then(|pages| pages.get(page as usize))
                .and_then(|page| page.get(line as usize))
                .cloned()
                .unwrap_or_default();
            s.push_string(&text);
        });

        // 4514
        none!(m, SPLIT_GETANIM => |s| {
            let page = s.pop_int();
            match s.split_mesanim {
                None => s.push_int(-1),
                Some(v) => {
                    let mesanim = cache()
                        .mesanims
                        .get_by_id(v)
                        .ok_or(ScriptError::MesanimNotFound(v as i32))?;
                    let line_count = s.split_pages
                        .as_ref()
                        .and_then(|pages| pages.get(page as usize))
                        .map(|p| p.len())
                        .unwrap_or(0);
                    if line_count == 0 {
                        s.push_int(-1);
                    } else {
                        s.push_int(mesanim.len[line_count - 1].map(|v| v as i32).unwrap_or(-1));
                    }
                }
            }
        });

        // 4515
        none!(m, SPLIT_INIT => |s| {
            let font = pop_font(s)?;
            let lines = s.pop_int();
            let width = s.pop_int();
            let mut text = s.pop_string();
            if text.starts_with("<p,") && let Some(end) = text.find(">") {
                let name = &text[3..end];
                let mesanim = cache()
                    .mesanims
                    .get_by_debugname(name)
                    .ok_or(ScriptError::MesanimNotFoundName(name.into()))?;
                s.split_mesanim = Some(mesanim.id);
                text = text[end + 1..].to_string();
            } else {
                s.split_mesanim = None;
            }
            s.split_pages = Some(font.split(&text, width as u16)
              .chunks(lines as usize)
              .map(|chunk| chunk.to_vec())
              .collect());
        });

        // 4516
        none!(m, SPLIT_LINECOUNT => |s| {
            let page = s.pop_int();
            let pages = s.split_pages
                .as_deref()
                .ok_or(ScriptError::Runtime("Split pages not found!".to_string()))?;
            let page = pages
                .get(page as usize)
                .ok_or(ScriptError::Runtime(format!("Split page {} not found!", page)))?;
            s.push_int(page.len() as i32);
        });

        // 4517
        none!(m, SPLIT_PAGECOUNT => |s| {
            let pages = s.split_pages
                .as_deref()
                .ok_or(ScriptError::Runtime("Split pages not found!".to_string()))?;
            s.push_int(pages.len() as i32);
        });
    }
}
