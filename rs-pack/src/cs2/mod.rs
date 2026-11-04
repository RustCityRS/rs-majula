//! CS2 (ClientScript2) decompiler — stack-machine bytecode → structured source.
//!
//! Pipeline: binary → `Script` → `Function` (IR) → `Construct` (CFG) → text.
//!
//! Port of RuneStar's Kotlin decompiler to Rust.

pub mod cfa;
pub mod codegen;
pub mod interpreter;
pub mod ir;
pub mod names;
pub mod opcodes;
pub mod script;
pub mod types;

use std::collections::HashMap;

use anyhow::Result;

/// Metadata about a script's parameter and return value counts.
#[derive(Debug, Clone, Default)]
pub struct ScriptMeta {
    pub int_params: u16,
    pub string_params: u16,
    pub long_params: u16,
    pub int_returns: u16,
    pub string_returns: u16,
    /// Semantic types for each int return position (e.g., Obj, Stat, Component).
    pub int_return_types: Vec<types::ArgType>,
}

/// Run the full decompile pipeline on raw script bytes.
pub fn decompile(data: &[u8], script_id: u32) -> Result<String> {
    decompile_with_names(data, script_id, None, None)
}

/// Run the full decompile pipeline with optional name/param lookups.
pub fn decompile_with_names(
    data: &[u8],
    script_id: u32,
    script_names: Option<&HashMap<u32, String>>,
    script_meta: Option<&HashMap<u32, ScriptMeta>>,
) -> Result<String> {
    let script = script::Script::decode(data)?;
    let function = interpreter::interpret_with_params(&script, script_id, script_meta);
    let construct = cfa::reconstruct(&function);
    let my_meta = script_meta.and_then(|m| m.get(&script_id));
    Ok(codegen::generate_with_names(
        &construct,
        &function,
        script_names,
        my_meta,
    ))
}
