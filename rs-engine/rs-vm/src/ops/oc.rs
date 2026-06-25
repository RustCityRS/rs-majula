use crate::register::OpsRegistry;
use crate::util::{cert, pop_obj, pop_param, uncert};
use crate::{ScriptError, handlers, none};
use rs_pack::ParamValue;
use rs_pack::cache::script::*;

/// Registers object config (OC) opcodes for querying static item definitions
/// from the cache, such as cost, name, equipment slots, and parameters.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Identity:** `OC_NAME`, `OC_DEBUGNAME`, `OC_DESC`, `OC_CATEGORY`
/// - **Properties:** `OC_COST`, `OC_STACKABLE`, `OC_TRADEABLE`, `OC_MEMBERS`
/// - **Equipment:** `OC_WEARPOS`, `OC_WEARPOS2`, `OC_WEARPOS3`
/// - **Certificates:** `OC_CERT`, `OC_UNCERT`
/// - **Params:** `OC_PARAM`
/// - **Interactions:** `OC_IOP` (stub)
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::oc::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 4200
        none!(m, OC_CATEGORY => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.category.map(|x| x as i32).unwrap_or(-1));
        });

        // 4201
        none!(m, OC_CERT => |s| {
            let obj = pop_obj(s)?;
            s.push_int(cert(obj) as i32);
        });

        // 4202
        none!(m, OC_COST => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.cost);
        });

        // 4203
        none!(m, OC_DEBUGNAME => |s| {
            let obj = pop_obj(s)?;
            s.push_string(obj.debugname().unwrap_or("null"));
        });

        // 4204
        none!(m, OC_DESC => |s| {
            let obj = pop_obj(s)?;
            s.push_string(obj.desc.as_deref().unwrap_or("null"));
        });

        // 4205
        none!(m, OC_IOP => |_s| {
            Err(ScriptError::Runtime("Unimplemented.".to_string()))?;
        });

        // 4206
        none!(m, OC_MEMBERS => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.members as i32);
        });

        // 4207
        none!(m, OC_NAME => |s| {
            let obj = pop_obj(s)?;
            s.push_string(obj.name.as_deref().unwrap_or(obj.debugname().unwrap_or("null")));
        });

        // 4209
        none!(m, OC_PARAM => |s| {
            let param = pop_param(s)?;
            let obj = pop_obj(s)?;
            let value = obj.params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });

        // 4210
        none!(m, OC_STACKABLE => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.stackable as i32);
        });

        // 4211
        none!(m, OC_TRADEABLE => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.tradeable as i32);
        });

        // 4212
        none!(m, OC_UNCERT => |s| {
            let obj = pop_obj(s)?;
            s.push_int(uncert(obj) as i32);
        });

        // 4213
        none!(m, OC_WEARPOS => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.wearpos.map_or(-1, |v| v as i32));
        });

        // 4214
        none!(m, OC_WEARPOS2 => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.wearpos2.map_or(-1, |v| v as i32));
        });

        // 4215
        none!(m, OC_WEARPOS3 => |s| {
            let obj = pop_obj(s)?;
            s.push_int(obj.wearpos3.map_or(-1, |v| v as i32));
        });
    }
}
