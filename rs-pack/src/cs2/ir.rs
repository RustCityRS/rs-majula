//! Intermediate representation types for CS2 decompilation.
//!
//! The IR sits between the raw stack-machine bytecode and the structured
//! control-flow constructs produced by CFA.

use std::fmt;

/// Stack types in CS2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackType {
    Int,
    String,
    Long,
}

/// An IR variable reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Variable {
    /// Local int variable (index into local int table).
    LocalInt(u32),
    /// Local string variable (index into local string table).
    LocalString(u32),
    /// Local long variable (index into local long table).
    LocalLong(u32),
    /// Stack int temporary.
    StackInt(u32),
    /// Stack string temporary.
    StackString(u32),
    /// Player variable (varp).
    Varp(u32),
    /// Variable bit (varbit).
    Varbit(u32),
    /// Client int variable (varc_int).
    VarcInt(u32),
    /// Client string variable (varc_string).
    VarcString(u32),
    /// Array variable.
    Array(u32),
}

impl fmt::Display for Variable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variable::LocalInt(i) => write!(f, "$int{i}"),
            Variable::LocalString(i) => write!(f, "$str{i}"),
            Variable::LocalLong(i) => write!(f, "$long{i}"),
            Variable::StackInt(i) => write!(f, "_si{i}"),
            Variable::StackString(i) => write!(f, "_ss{i}"),
            Variable::Varp(id) => write!(f, "%varp{id}"),
            Variable::Varbit(id) => write!(f, "%varbit{id}"),
            Variable::VarcInt(id) => write!(f, "%varc_int{id}"),
            Variable::VarcString(id) => write!(f, "%varc_str{id}"),
            Variable::Array(id) => write!(f, "%array{id}"),
        }
    }
}

/// An expression in the IR.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Integer literal.
    IntConstant(i32),
    /// String literal.
    StringConstant(String),
    /// Long literal.
    LongConstant(i64),
    /// Variable read.
    Var(Variable),
    /// Generic opcode operation: `opname(args...)`.
    Operation { opcode: u16, args: Vec<Expr> },
    /// Procedure call: `~script_id(args...)`.
    Proc { script_id: i32, args: Vec<Expr> },
    /// String concatenation (join_string).
    StringConcat(Vec<Expr>),
    /// A compound expression wrapping multiple sub-expressions (e.g. for
    /// multi-return gosub results).
    Compound(Vec<Expr>),
}

/// An IR instruction (one per logical operation after stack simulation).
#[derive(Debug, Clone)]
pub enum Instruction {
    /// A label marking a branch target.
    Label(u32),
    /// Assignment: `targets = expr`.
    Assignment { targets: Vec<Variable>, expr: Expr },
    /// Return statement with captured stack values.
    Return {
        int_values: Vec<Expr>,
        string_values: Vec<Expr>,
    },
    /// Conditional branch: if `condition` then goto `target`.
    Branch { condition: Expr, target: u32 },
    /// Unconditional goto.
    Goto(u32),
    /// Switch dispatch.
    Switch { expr: Expr, cases: Vec<(i32, u32)> },
    /// Expression evaluated for side effects (no assignment target).
    Expression(Expr),
}

/// A decompiled function (one CS2 script).
pub struct Function {
    pub id: u32,
    pub name: String,
    pub int_param_count: u16,
    pub string_param_count: u16,
    pub long_param_count: u16,
    pub local_int_count: u16,
    pub local_string_count: u16,
    pub local_long_count: u16,
    pub instructions: Vec<Instruction>,
}
