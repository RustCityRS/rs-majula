use crate::register::OpsRegistry;
use crate::util::{pop_loc, pop_param};
use crate::{ScriptError, handlers, none};
use rs_pack::ParamValue;
use rs_pack::cache::script::*;

pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 4100
        none!(m, LC_CATEGORY => |s| {
            let loc = pop_loc(s)?;
            s.push_int(loc.category.map(|x| x as i32).unwrap_or(-1));
        });

        // 4101
        none!(m, LC_DEBUGNAME => |s| {
            let loc = pop_loc(s)?;
            s.push_string(loc.debugname().unwrap_or("null"));
        });

        // 4102
        none!(m, LC_DESC => |s| {
            let loc = pop_loc(s)?;
            s.push_string(loc.desc.as_deref().unwrap_or("null"));
        });

        // 4103
        none!(m, LC_LENGTH => |s| {
            let loc = pop_loc(s)?;
            s.push_int(loc.length as i32);
        });

        // 4104
        none!(m, LC_NAME => |s| {
            let loc = pop_loc(s)?;
            s.push_string(loc.name.as_deref().unwrap_or(loc.debugname().unwrap_or("null")));
        });

        // 4105
        none!(m, LC_OP => |_s| {
            Err(ScriptError::Runtime("Unimplemented.".to_string()))?;
        });

        // 4106
        none!(m, LC_PARAM => |s| {
            let param = pop_param(s)?;
            let loc = pop_loc(s)?;
            let value = loc.params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });

        // 4107
        none!(m, LC_WIDTH => |s| {
            let loc = pop_loc(s)?;
            s.push_int(loc.width as i32);
        });
    }
}
