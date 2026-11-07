pub use crate::iterators::*;
use crate::pointer::{ScriptPointer, ScriptPointerSet};
use crate::subject::ScriptSubject;
use crate::trigger::ServerTriggerType;
use crate::{LocSerial, NpcUid, ObjSerial, PlayerUid, ScriptError};
use num_enum::TryFromPrimitive;
use rs_grid::CoordGrid;
use rs_pack::cache::script::Script;
use rs_pack::types::{LocAngle, LocLayer, LocShape};
use std::any::type_name;
use std::sync::Arc;
use std::time::Instant;

/// The full execution state of a single VM script invocation.
///
/// `ScriptState` carries everything the bytecode interpreter needs to run a
/// script to completion: the program counter, operand stacks, local variables,
/// call frames for `gosub`/`goto`, active entity references, iterator state,
/// and the current execution status.
///
/// Stacks (`int_stack`, `string_stack`) are pre-allocated to a fixed capacity
/// of 128 entries and accessed via unchecked pointer arithmetic for
/// performance, guarded only by `debug_assert!` bounds checks.
///
/// Entity reference fields (`active_player`, `active_npc`, etc.) come in
/// primary/secondary pairs. The primary slot holds the *subject* entity and
/// the secondary slot holds the *target*, unless the subject and target share
/// the same entity type, in which case the target is placed in the secondary
/// slot.
///
/// The [`pointers`](Self::pointers) bitset mirrors which entity fields are
/// `Some`, and is kept in sync by [`sync_pointers`](Self::sync_pointers).
#[derive(Debug, Clone)]
pub struct ScriptState {
    pub script: Arc<Script>,
    pub root_script_id: i32,
    pub(crate) pc: i32,
    pub(crate) opcount: i32,
    int_stack: Vec<i32>,
    pub(crate) isp: i32,
    string_stack: Vec<String>,
    pub(crate) ssp: i32,
    pub(crate) int_locals: Vec<i32>,
    pub(crate) string_locals: Vec<String>,
    pub(crate) int_base: usize,
    pub(crate) string_base: usize,
    pub(crate) gosub_frame_stack: Vec<GoSubFrame>,
    pub(crate) goto_frame_stack: Vec<GoToFrame>,
    pub execution: ExecutionState,
    pub trigger: Option<ServerTriggerType>,
    pub pointers: ScriptPointerSet,

    pub active_player: Option<PlayerUid>,
    pub active_player2: Option<PlayerUid>,
    pub active_npc: Option<NpcUid>,
    pub active_npc2: Option<NpcUid>,
    pub active_loc: Option<LocRef>,
    pub active_loc2: Option<LocRef>,
    pub active_obj: Option<ObjRef>,
    pub active_obj2: Option<ObjRef>,

    pub last_int: Option<i32>,
    /// Rev-647: the string carried by RESUME_P_STRINGDIALOG (or a display-name
    /// result), read back by the `last_string` command.
    #[cfg(since_647)]
    pub last_string: Option<String>,
    // The player whose script queued this one, when it runs from a queue: a
    // hit it paints is theirs (2011's hitsplats are bright for the hit's
    // dealer and receiver, darker for everyone else).
    #[cfg(since_647)]
    pub queue_source: Option<u16>,

    pub(crate) split_pages: Option<Vec<Vec<String>>>,
    pub(crate) split_mesanim: Option<u16>,

    pub(crate) db_table: Option<u16>,
    pub(crate) db_row: Option<u16>,
    pub(crate) db_row_query: Vec<u16>,

    pub(crate) npc_iterator: Option<NpcIteratorState>,
    pub(crate) loc_iterator: Option<LocIteratorState>,
    pub(crate) obj_iterator: Option<ObjIteratorState>,
    pub(crate) player_iterator: Option<PlayerIteratorState>,

    pub(crate) timespent: Option<Instant>,

    pub delay: i32,
}

impl ScriptState {
    pub const ACTIVE_NPC: [ScriptPointer; 2] =
        [ScriptPointer::ActiveNpc, ScriptPointer::ActiveNpc2];

    pub const ACTIVE_LOC: [ScriptPointer; 2] =
        [ScriptPointer::ActiveLoc, ScriptPointer::ActiveLoc2];

    pub const ACTIVE_OBJ: [ScriptPointer; 2] =
        [ScriptPointer::ActiveObj, ScriptPointer::ActiveObj2];

    pub const ACTIVE_PLAYER: [ScriptPointer; 2] =
        [ScriptPointer::ActivePlayer, ScriptPointer::ActivePlayer2];

    pub const PROTECTED_ACTIVE_PLAYER: [ScriptPointer; 2] = [
        ScriptPointer::ProtectedActivePlayer,
        ScriptPointer::ProtectedActivePlayer2,
    ];

    /// Constructs a new `ScriptState` from a script and optional arguments.
    ///
    /// Initializes all stacks (int and string) with a fixed capacity of 128,
    /// pre-fills local variable vectors from the provided arguments, and pads
    /// any remaining local slots with defaults (`0` for ints, empty string for
    /// strings). The program counter starts at `-1` (pre-incremented by the
    /// interpreter loop before the first opcode fetch).
    ///
    /// # Arguments
    ///
    /// * `script` - The compiled script to execute, wrapped in an `Arc`.
    /// * `args` - Optional typed arguments whose values are placed into the
    ///   corresponding local variable slots (ints into `int_locals`, strings
    ///   into `string_locals`) in order.
    ///
    /// # Returns
    ///
    /// A fully initialized `ScriptState` in [`ExecutionState::Running`].
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`init`](Self::init).
    fn new(script: Arc<Script>, args: Option<Vec<ScriptArgument>>) -> Self {
        let mut int_locals = Vec::with_capacity(script.int_local_count as usize);
        let mut string_locals = Vec::with_capacity(script.string_local_count as usize);
        if let Some(args) = args {
            for arg in args {
                match arg {
                    ScriptArgument::Int(val) => int_locals.push(val),
                    ScriptArgument::String(val) => string_locals.push(val),
                }
            }
        }
        int_locals.resize(script.int_local_count as usize, 0);
        string_locals.resize(script.string_local_count as usize, String::new());
        let root_script_id = script.id;
        let trigger = ServerTriggerType::try_from_primitive((script.info.lookup & 0xFF) as u8).ok();
        Self {
            script,
            root_script_id,
            pc: -1,
            opcount: 0,
            int_stack: vec![0; 128],
            isp: 0,
            string_stack: vec![String::new(); 128],
            ssp: 0,
            int_locals,
            string_locals,
            int_base: 0,
            string_base: 0,
            gosub_frame_stack: Vec::new(),
            goto_frame_stack: Vec::new(),
            execution: ExecutionState::Running,
            trigger,
            pointers: ScriptPointerSet::new(),
            active_player: None,
            active_player2: None,
            active_npc: None,
            active_npc2: None,
            active_loc: None,
            active_loc2: None,
            active_obj: None,
            active_obj2: None,
            last_int: None,
            #[cfg(since_647)]
            last_string: None,
            #[cfg(since_647)]
            queue_source: None,
            split_pages: None,
            split_mesanim: None,
            db_table: None,
            db_row: None,
            db_row_query: Vec::new(),
            npc_iterator: None,
            loc_iterator: None,
            obj_iterator: None,
            player_iterator: None,
            timespent: None,
            delay: 0,
        }
    }

    /// Creates and fully initializes a `ScriptState` with subject and target
    /// entity bindings.
    ///
    /// This is the primary public constructor. It delegates to [`new`](Self::new)
    /// for base initialization, then binds the subject entity to the primary
    /// active slot and the target entity to either the primary or secondary
    /// slot depending on whether the subject already occupies the primary slot
    /// for that entity type. Finally, [`sync_pointers`](Self::sync_pointers)
    /// is called to update the pointer bitset.
    ///
    /// # Arguments
    ///
    /// * `script` - The compiled script to execute.
    /// * `subject` - The primary entity this script operates on (if any).
    /// * `target` - The secondary/target entity (if any). If `subject` and
    ///   `target` are the same entity type, `target` goes into the `2` slot.
    /// * `args` - Optional typed arguments forwarded to [`new`](Self::new).
    ///
    /// # Returns
    ///
    /// A fully initialized `ScriptState` ready for the interpreter loop.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`new`](Self::new), [`sync_pointers`](Self::sync_pointers).
    ///
    /// **Called by:** Engine script execution entry points (player handlers,
    /// NPC phases, world phases, trigger handlers).
    pub fn init(
        script: Arc<Script>,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        args: Option<Vec<ScriptArgument>>,
    ) -> Self {
        let mut state = Self::new(script, args);
        state.bind_subject_target(subject, target);
        state.sync_pointers();
        state
    }

    /// Resets an existing `ScriptState` in place, reusing heap allocations.
    ///
    /// Instead of allocating a new `ScriptState` via [`init`](Self::init),
    /// this method overwrites every field while preserving the backing
    /// memory of the fixed-size stacks (`int_stack`, `string_stack`) and
    /// frame stacks (`gosub_frame_stack`, `goto_frame_stack`). Only the
    /// variable-size local vectors are cleared and resized, which may
    /// reuse their existing capacity when the new script's local counts
    /// fit within the old allocation.
    ///
    /// # Motivation
    ///
    /// `ScriptState::init` allocates ~4 KB of heap memory per call
    /// (`vec![0; 128]` for `int_stack`, 128 empty `String`s for
    /// `string_stack`, plus frame stacks). With 20,000+ script
    /// invocations per tick this produces significant allocator pressure.
    /// `reset` eliminates all of those allocations for the common case
    /// where a reusable `ScriptState` is available.
    ///
    /// # Arguments
    ///
    /// * `script` -- The compiled script to execute.
    /// * `subject` -- The primary entity this script operates on (if any).
    /// * `target` -- The secondary/target entity (if any).
    /// * `args` -- Optional typed arguments placed into local variable slots.
    ///
    /// # Side Effects
    ///
    /// All fields are overwritten. The `int_stack` and `string_stack`
    /// backing buffers are *not* reallocated; stack pointers are simply
    /// reset to 0 (stale values are overwritten before they are read).
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`sync_pointers`](Self::sync_pointers).
    ///
    /// **Called by:** [`Engine::run_script_inner`] when a reusable state
    /// is available.
    pub fn reset(
        &mut self,
        script: Arc<Script>,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        args: Option<Vec<ScriptArgument>>,
    ) {
        // -- Locals: clear and repopulate from args --
        self.int_locals.clear();
        self.string_locals.clear();
        self.int_base = 0;
        self.string_base = 0;
        if let Some(args) = args {
            for arg in args {
                match arg {
                    ScriptArgument::Int(val) => self.int_locals.push(val),
                    ScriptArgument::String(val) => self.string_locals.push(val),
                }
            }
        }
        self.int_locals.resize(script.int_local_count as usize, 0);
        self.string_locals
            .resize(script.string_local_count as usize, String::new());

        // -- Script metadata --
        self.root_script_id = script.id;
        self.trigger =
            ServerTriggerType::try_from_primitive((script.info.lookup & 0xFF) as u8).ok();
        self.script = script;

        // -- Program counter and opcount --
        self.pc = -1;
        self.opcount = 0;

        // -- Stacks: just reset pointers. Values are overwritten before read. --
        self.isp = 0;
        self.ssp = 0;

        // -- Frame stacks: clear contents, retain capacity --
        self.gosub_frame_stack.clear();
        self.goto_frame_stack.clear();

        // -- Execution --
        self.execution = ExecutionState::Running;

        // -- Entity references --
        self.active_player = None;
        self.active_player2 = None;
        self.active_npc = None;
        self.active_npc2 = None;
        self.active_loc = None;
        self.active_loc2 = None;
        self.active_obj = None;
        self.active_obj2 = None;

        // -- Bind subject and target --
        self.bind_subject_target(subject, target);

        // -- Misc state --
        self.last_int = None;
        #[cfg(since_647)]
        {
            self.last_string = None;
        }
        self.split_pages = None;
        self.split_mesanim = None;
        self.db_table = None;
        self.db_row = None;
        self.db_row_query.clear();
        self.npc_iterator = None;
        self.loc_iterator = None;
        self.obj_iterator = None;
        self.player_iterator = None;
        self.timespent = None;
        self.delay = 0;

        // -- Rebuild pointer bitset --
        self.sync_pointers();
    }

    /// Binds the `subject` and `target` entities into the active-entity slots.
    ///
    /// The subject is placed in the primary slot for its entity type. The target
    /// is placed in the secondary (`2`) slot when it shares the subject's entity
    /// type, otherwise in the primary slot for its own type. Shared by
    /// [`init`](Self::init) and [`reset`](Self::reset).
    ///
    /// # Arguments
    ///
    /// * `subject` - The primary entity (if any).
    /// * `target` - The secondary/target entity (if any). If it matches the
    ///   subject's entity type, it goes into the `2` slot.
    fn bind_subject_target(
        &mut self,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
    ) {
        match subject {
            Some(ScriptSubject::Player(uid)) => self.active_player = Some(uid),
            Some(ScriptSubject::Npc(uid)) => self.active_npc = Some(uid),
            Some(ScriptSubject::Loc(loc)) => self.active_loc = Some(loc),
            Some(ScriptSubject::Obj(obj)) => self.active_obj = Some(obj),
            None => {}
        }

        match target {
            Some(ScriptSubject::Player(uid)) => {
                if matches!(&subject, Some(ScriptSubject::Player(_))) {
                    self.active_player2 = Some(uid);
                } else {
                    self.active_player = Some(uid);
                }
            }
            Some(ScriptSubject::Npc(uid)) => {
                if matches!(&subject, Some(ScriptSubject::Npc(_))) {
                    self.active_npc2 = Some(uid);
                } else {
                    self.active_npc = Some(uid);
                }
            }
            Some(ScriptSubject::Loc(loc)) => {
                if matches!(&subject, Some(ScriptSubject::Loc(_))) {
                    self.active_loc2 = Some(loc);
                } else {
                    self.active_loc = Some(loc);
                }
            }
            Some(ScriptSubject::Obj(obj)) => {
                if matches!(&subject, Some(ScriptSubject::Obj(_))) {
                    self.active_obj2 = Some(obj);
                } else {
                    self.active_obj = Some(obj);
                }
            }
            None => {}
        }
    }

    /// Rebuilds the [`pointers`](Self::pointers) bitset from the current
    /// entity reference fields.
    ///
    /// Clears the entire bitset, then iterates over all eight entity fields
    /// (`active_player` through `active_obj2`) and sets the corresponding
    /// [`ScriptPointer`] bit for each field that is `Some`.
    ///
    /// # Side Effects
    ///
    /// Replaces `self.pointers` with a freshly computed [`ScriptPointerSet`].
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`ScriptPointerSet::clear`], [`ScriptPointerSet::add`].
    ///
    /// **Called by:** [`init`](Self::init) and [`reset`](Self::reset) after
    /// entity binding.
    fn sync_pointers(&mut self) {
        let checks: &[(bool, ScriptPointer)] = &[
            (self.active_player.is_some(), ScriptPointer::ActivePlayer),
            (self.active_player2.is_some(), ScriptPointer::ActivePlayer2),
            (self.active_npc.is_some(), ScriptPointer::ActiveNpc),
            (self.active_npc2.is_some(), ScriptPointer::ActiveNpc2),
            (self.active_loc.is_some(), ScriptPointer::ActiveLoc),
            (self.active_loc2.is_some(), ScriptPointer::ActiveLoc2),
            (self.active_obj.is_some(), ScriptPointer::ActiveObj),
            (self.active_obj2.is_some(), ScriptPointer::ActiveObj2),
        ];

        self.pointers.clear();
        checks
            .iter()
            .filter(|&&(present, _)| present)
            .for_each(|&(_, pointer)| self.pointers.add(pointer));
    }

    /// Resets the execution context for a new script program (used by `goto`).
    ///
    /// Drops every frame's locals, since a `goto` has no return path, then
    /// sets up the new script's locals with [`push_locals`](Self::push_locals).
    /// The program counter is reset to `-1` so the interpreter loop begins at
    /// the first opcode.
    ///
    /// # Arguments
    ///
    /// * `script` - A static reference to the new script to execute.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Side Effects
    ///
    /// * Clears `int_locals` and `string_locals` and resets their bases to 0.
    /// * Pops argument values from `int_stack` / `string_stack`.
    /// * Resets `pc` to `-1` and replaces `self.script`.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`push_locals`](Self::push_locals).
    ///
    /// **Called by:** [`goto_frame`](Self::goto_frame).
    #[inline(always)]
    fn new_program(&mut self, script: &'static Arc<Script>) -> Result<(), ScriptError> {
        self.int_locals.clear();
        self.string_locals.clear();
        self.int_base = 0;
        self.string_base = 0;
        self.push_locals(script);
        self.pc = -1;
        self.script = Arc::clone(script);
        Ok(())
    }

    /// Appends `script`'s locals at the current bases and pops its arguments
    /// from the operand stacks into the first slots (in reverse order to
    /// preserve argument ordering).
    ///
    /// The locals buffers must end at the bases, which holds after
    /// [`new_program`](Self::new_program) clears them and after
    /// [`gosub_frame`](Self::gosub_frame) moves the bases to their ends.
    #[inline(always)]
    fn push_locals(&mut self, script: &Script) {
        self.int_locals
            .resize(self.int_base + script.int_local_count as usize, 0);
        for i in (0..script.int_arg_count as usize).rev() {
            self.int_locals[self.int_base + i] = self.pop_int();
        }
        self.string_locals.resize(
            self.string_base + script.string_local_count as usize,
            String::new(),
        );
        for i in (0..script.string_arg_count as usize).rev() {
            self.string_locals[self.string_base + i] = self.pop_string();
        }
    }

    /// Restores execution context from the most recent `gosub` call frame.
    ///
    /// Pops the top [`GoSubFrame`] from the gosub frame stack, drops the
    /// returning proc's locals off the end of the locals buffers, and restores
    /// the caller's script reference, program counter, and locals bases,
    /// effectively returning from a subroutine.
    ///
    /// # Panics
    ///
    /// Panics if the gosub frame stack is empty (via `unwrap()`).
    ///
    /// # Side Effects
    ///
    /// * Pops the [`GoToFrame`] that [`gosub_frame`](Self::gosub_frame)
    ///   pushed, so the backtrace only lists procs still on the call stack.
    /// * Truncates `int_locals` and `string_locals` to the returning proc's
    ///   bases, then replaces `self.script`, `self.pc`, `self.int_base`, and
    ///   `self.string_base` with the saved frame values.
    ///
    /// **Called by:** The `Return` opcode handler in `ops::core`.
    #[inline(always)]
    pub(crate) fn pop_frame(&mut self) {
        let frame = self.gosub_frame_stack.pop().unwrap();
        self.goto_frame_stack.pop();
        self.int_locals.truncate(self.int_base);
        self.string_locals.truncate(self.string_base);
        self.int_base = frame.int_base;
        self.string_base = frame.string_base;
        self.script = frame.script;
        self.pc = frame.pc;
    }

    /// Pushes a `gosub` call frame and enters a subroutine script.
    ///
    /// Saves the current script, program counter, and locals bases onto
    /// both the gosub and goto frame stacks, then appends the new script's
    /// locals after the caller's and pops its arguments into them. The caller's
    /// locals stay where they are, so a call allocates nothing once the
    /// buffers have grown. The current `script` is replaced without an extra
    /// `Arc::clone` by using `std::mem::replace`.
    ///
    /// # Arguments
    ///
    /// * `script` - A static reference to the subroutine script to enter.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Side Effects
    ///
    /// * Pushes a [`GoSubFrame`] onto `gosub_frame_stack`.
    /// * Pushes a [`GoToFrame`] onto `goto_frame_stack` for the backtrace;
    ///   [`pop_frame`](Self::pop_frame) pops it on return.
    /// * Moves `int_base` and `string_base` to the ends of the locals buffers.
    /// * Pops argument values from operand stacks into the new locals.
    /// * Resets `pc` to `-1`.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`push_locals`](Self::push_locals).
    ///
    /// **Called by:** The `GoSub` / `GoSubWithParams` opcode handlers in `ops::core`.
    #[inline(always)]
    pub(crate) fn gosub_frame(&mut self, script: &'static Arc<Script>) -> Result<(), ScriptError> {
        let old_script = std::mem::replace(&mut self.script, Arc::clone(script));
        self.goto_frame_stack.push(GoToFrame {
            script: Arc::clone(&old_script),
            pc: self.pc,
        });
        self.gosub_frame_stack.push(GoSubFrame {
            script: old_script, // move, no clone
            pc: self.pc,
            int_base: self.int_base,
            string_base: self.string_base,
        });
        self.int_base = self.int_locals.len();
        self.string_base = self.string_locals.len();
        self.push_locals(script);
        self.pc = -1;
        Ok(())
    }

    /// Pushes a `goto` frame and transfers execution to a new script.
    ///
    /// Unlike [`gosub_frame`](Self::gosub_frame), a `goto` is a one-way jump:
    /// the gosub frame stack is cleared (no return path), and execution
    /// continues in the target script via [`new_program`](Self::new_program).
    /// A [`GoToFrame`] is still pushed for stack-trace / debugging purposes.
    ///
    /// # Arguments
    ///
    /// * `script` - A static reference to the target script to jump to.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success.
    ///
    /// # Side Effects
    ///
    /// * Pushes a [`GoToFrame`] onto `goto_frame_stack`.
    /// * Clears `gosub_frame_stack`.
    /// * Delegates to [`new_program`](Self::new_program) to reset locals and PC.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`new_program`](Self::new_program).
    ///
    /// **Called by:** The `GoTo` / `GoToWithParams` opcode handlers in `ops::core`.
    #[inline(always)]
    pub(crate) fn goto_frame(&mut self, script: &'static Arc<Script>) -> Result<(), ScriptError> {
        self.goto_frame_stack.push(GoToFrame {
            script: Arc::clone(&self.script),
            pc: self.pc,
        });
        self.gosub_frame_stack.clear();
        self.new_program(script)
    }

    /// Reads the integer operand at the current program counter position.
    ///
    /// Uses unchecked pointer arithmetic for performance. The bounds are
    /// validated only via `debug_assert!` in debug builds.
    ///
    /// # Returns
    ///
    /// The `i32` operand at `self.script.int_operands[self.pc]`.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` pointer access into `int_operands`. The caller must
    /// ensure `self.pc` is a valid index. In release builds, an out-of-bounds
    /// `pc` causes undefined behavior.
    ///
    /// **Called by:** Opcode handlers across `ops::core`, `ops::player`,
    /// `ops::npc`, `ops::string`, etc., and VM utility macros.
    #[inline(always)]
    pub fn int_operand(&self) -> i32 {
        debug_assert!((self.pc as usize) < self.script.int_operands.len());
        unsafe { *self.script.int_operands.as_ptr().add(self.pc as usize) }
    }

    /// Pushes a 32-bit integer onto the integer operand stack.
    ///
    /// Uses unchecked pointer arithmetic for performance. Overflow is caught
    /// only by `debug_assert!` in debug builds.
    ///
    /// # Arguments
    ///
    /// * `value` - The integer value to push.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` mutable pointer write into `int_stack`. If `isp` exceeds
    /// the stack capacity (128), release builds will exhibit undefined behavior.
    ///
    /// # Panics
    ///
    /// In debug builds, panics with a stack overflow message including the
    /// script name if `isp >= 128`.
    ///
    /// # Side Effects
    ///
    /// Writes `value` at position `isp` and increments `isp`.
    ///
    /// **Called by:** Opcode handlers that produce integer results (arithmetic,
    /// variable loads, comparisons, etc.).
    #[inline(always)]
    pub fn push_int(&mut self, value: i32) {
        debug_assert!(
            (self.isp as usize) < self.int_stack.len(),
            "{}",
            format!("int stack overflow, script: {}", self.script.info.name)
        );
        unsafe { *self.int_stack.as_mut_ptr().add(self.isp as usize) = value };
        self.isp += 1;
    }

    /// Pops a 32-bit integer from the integer operand stack.
    ///
    /// Decrements `isp` first, then reads the value at the new position using
    /// unchecked pointer arithmetic.
    ///
    /// # Returns
    ///
    /// The `i32` value at the top of the integer stack.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` pointer read from `int_stack`. If the stack is empty,
    /// `isp` wraps negative and release builds will exhibit undefined behavior.
    ///
    /// # Panics
    ///
    /// In debug builds, panics with a stack underflow message including the
    /// script name if `isp` goes below `0`.
    ///
    /// # Side Effects
    ///
    /// Decrements `isp`.
    ///
    /// **Called by:** Opcode handlers that consume integer operands, and
    /// [`new_program`](Self::new_program) / [`gosub_frame`](Self::gosub_frame)
    /// when popping script arguments into locals.
    #[inline(always)]
    pub fn pop_int(&mut self) -> i32 {
        self.isp -= 1;
        debug_assert!(
            self.isp >= 0,
            "{}",
            format!("int stack underflow, script: {}", self.script.info.name)
        );
        unsafe { *self.int_stack.as_ptr().add(self.isp as usize) }
    }

    /// Pops a 32-bit integer from the stack and narrows it to `T`, range-checking the value.
    ///
    /// Returns [`ScriptError::Runtime`] if the popped value doesn't fit in `T`
    /// (e.g. a negative or >255 value popped as `u8`).
    ///
    /// # Returns
    ///
    /// The `T` value at the top of the integer stack.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` pointer read from `int_stack`. If the stack is empty,
    /// `isp` wraps negative and release builds will exhibit undefined behavior.
    ///
    /// # Panics
    ///
    /// In debug builds, panics with a stack underflow message including the
    /// script name if `isp` goes below `0`.
    ///
    /// # Side Effects
    ///
    /// Decrements `isp`.
    ///
    /// **Called by:** Opcode handlers that consume integer operands.
    #[inline]
    pub fn pop_int_as<T>(&mut self) -> Result<T, ScriptError>
    where
        T: TryFrom<i32>,
    {
        let value = self.pop_int();
        T::try_from(value).map_err(|_| {
            ScriptError::Runtime(format!(
                "value: {value} out of range for {}",
                type_name::<T>()
            ))
        })
    }

    /// Reads the string operand at the current program counter position.
    ///
    /// Uses unchecked pointer arithmetic for performance. The bounds are
    /// validated only via `debug_assert!` in debug builds.
    ///
    /// # Returns
    ///
    /// A `&str` reference to `self.script.strings[int_operand]`: a string
    /// push's int operand is its index into the script's string table.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` pointer access into `strings`. The caller must ensure
    /// `self.pc` is a string push. In release builds, any other `pc` causes
    /// undefined behavior.
    ///
    /// **Called by:** The `PushConstantString` opcode handler in `ops::core`.
    #[inline(always)]
    pub(crate) fn string_operand(&self) -> &str {
        let index = self.int_operand() as usize;
        debug_assert!(index < self.script.strings.len());
        unsafe { &*self.script.strings.as_ptr().add(index) }
    }

    /// Pushes a string slice onto the string operand stack.
    ///
    /// Instead of allocating a new `String`, this reuses the pre-allocated
    /// stack slot by clearing it and copying the contents of `value` via
    /// `push_str`. Uses unchecked indexing for performance.
    ///
    /// # Arguments
    ///
    /// * `value` - The string slice to push.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` `get_unchecked_mut` to access the stack slot. If `ssp`
    /// exceeds the stack capacity (128), release builds will exhibit undefined
    /// behavior.
    ///
    /// # Panics
    ///
    /// In debug builds, panics with a stack overflow message including the
    /// script name if `ssp >= 128`.
    ///
    /// # Side Effects
    ///
    /// Clears the slot at `ssp`, writes `value` into it, and increments `ssp`.
    ///
    /// **Called by:** Opcode handlers that produce string results (variable loads,
    /// string constants, db lookups, etc.).
    #[inline(always)]
    pub fn push_string(&mut self, value: &str) {
        debug_assert!(
            (self.ssp as usize) < self.string_stack.len(),
            "{}",
            format!("string stack overflow, script: {}", self.script.info.name)
        );
        let slot = unsafe { self.string_stack.get_unchecked_mut(self.ssp as usize) };
        slot.clear();
        slot.push_str(value);
        self.ssp += 1;
    }

    /// Pushes a copy of a string local variable onto the string operand stack.
    ///
    /// Copies from `string_locals[string_base + idx]` to the stack slot at
    /// `ssp`. Uses raw pointer indirection to avoid borrow checker conflicts
    /// (both `string_locals` and `string_stack` are fields of `self`).
    ///
    /// # Arguments
    ///
    /// * `idx` - The running script's string local to copy from.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` pointer reads and `get_unchecked_mut`. Both `idx` and
    /// `ssp` must be in bounds; only `debug_assert!` guards are present.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `idx` is out of bounds for `string_locals`
    /// or if `ssp` overflows the string stack.
    ///
    /// # Side Effects
    ///
    /// Clears the stack slot at `ssp`, copies the local's contents into it,
    /// and increments `ssp`.
    ///
    /// **Called by:** The `PushStringLocal` opcode handler in `ops::core`.
    #[inline(always)]
    pub(crate) fn push_string_local(&mut self, idx: usize) {
        let idx = self.string_base + idx;
        debug_assert!(idx < self.string_locals.len(), "string local out of bounds");
        debug_assert!(
            (self.ssp as usize) < self.string_stack.len(),
            "string stack overflow"
        );
        let ssp = self.ssp as usize;
        let src = self.string_locals[idx].as_str() as *const str;
        let slot = unsafe { self.string_stack.get_unchecked_mut(ssp) };
        slot.clear();
        slot.push_str(unsafe { &*src });
        self.ssp += 1;
    }

    /// Concatenates the top `count` strings on the string stack into one.
    ///
    /// The bottom-most of the `count` strings becomes the destination; all
    /// strings above it are appended in order, then the stack pointer is
    /// adjusted so only the concatenated result remains. If `count` is 0 or 1
    /// this is a no-op.
    ///
    /// Uses raw pointer indirection to simultaneously read from one stack
    /// slot and write to another without triggering borrow checker errors.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of strings from the top of the stack to join.
    ///
    /// # Safety
    ///
    /// Uses a raw pointer into the stack for disjoint source and destination slots.
    /// `count` must not exceed `ssp`; only `debug_assert!` guards are present.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `count` exceeds the current stack depth.
    ///
    /// # Side Effects
    ///
    /// Mutates the string at position `ssp - count` (appends subsequent
    /// strings) and sets `ssp` to `ssp - count + 1`.
    ///
    /// **Called by:** `JoinString` in `ops::core` and `APPEND` in `ops::string`.
    #[inline(always)]
    pub(crate) fn join_strings(&mut self, count: usize) {
        debug_assert!(
            count <= self.ssp as usize,
            "join_strings: count exceeds stack"
        );
        if count <= 1 {
            return;
        }
        let start = self.ssp as usize - count;
        let slots = self.string_stack.as_mut_ptr();
        let dst = unsafe { &mut *slots.add(start) };
        for i in (start + 1)..self.ssp as usize {
            let src = unsafe { &*slots.add(i) };
            dst.push_str(src);
        }
        self.ssp = (start + 1) as i32;
    }

    /// Peeks at a string on the stack without popping it.
    ///
    /// Returns a reference to the string at `ssp - 1 - offset`, where an
    /// `offset` of `0` is the top of the stack. Uses unchecked indexing for
    /// performance.
    ///
    /// # Arguments
    ///
    /// * `offset` - How many positions below the top of the stack to peek.
    ///   `0` is the top element, `1` is the one below it, etc.
    ///
    /// # Returns
    ///
    /// A `&str` reference to the string at the specified stack position.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` `get_unchecked`. The computed index must be in bounds;
    /// only `debug_assert!` guards are present.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if the computed index is out of bounds.
    ///
    /// **Called by:** String comparison and length opcode handlers in `ops::string`.
    #[inline(always)]
    pub(crate) fn peek_string(&self, offset: i32) -> &str {
        let idx = (self.ssp - 1 - offset) as usize;
        debug_assert!(idx < self.string_stack.len(), "peek_string out of bounds");
        unsafe { self.string_stack.get_unchecked(idx) }
    }

    /// Discards the top `count` strings from the string stack without
    /// returning them.
    ///
    /// Simply decrements `ssp` by `count`. The string data in the discarded
    /// slots is not cleared and will be overwritten by subsequent pushes.
    ///
    /// # Arguments
    ///
    /// * `count` - The number of string stack entries to discard.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `ssp` goes below `0` (stack underflow).
    ///
    /// # Side Effects
    ///
    /// Decrements `ssp` by `count`.
    ///
    /// **Called by:** String comparison and length opcode handlers in `ops::string`
    /// after peeking at operands.
    #[inline(always)]
    pub(crate) fn drop_strings(&mut self, count: i32) {
        self.ssp -= count;
        debug_assert!(self.ssp >= 0, "string stack underflow on drop");
    }

    /// Pops a string from the string stack, taking ownership of the value.
    ///
    /// Decrements `ssp` and uses `std::mem::take` to move the `String` out
    /// of the stack slot, leaving an empty string in its place (avoiding a
    /// clone). Uses unchecked indexing for performance.
    ///
    /// # Returns
    ///
    /// The owned `String` that was at the top of the string stack.
    ///
    /// # Safety
    ///
    /// Uses `unsafe` `get_unchecked_mut`. If the stack is empty, `ssp` wraps
    /// negative and release builds will exhibit undefined behavior.
    ///
    /// # Panics
    ///
    /// In debug builds, panics with a stack underflow message including the
    /// script name if `ssp` goes below `0`.
    ///
    /// # Side Effects
    ///
    /// Decrements `ssp` and replaces the stack slot with an empty `String`.
    ///
    /// **Called by:** Opcode handlers that consume string operands,
    /// [`new_program`](Self::new_program), [`gosub_frame`](Self::gosub_frame),
    /// and [`pop_script_args`](Self::pop_script_args).
    #[inline(always)]
    pub fn pop_string(&mut self) -> String {
        self.ssp -= 1;
        debug_assert!(
            self.ssp >= 0,
            "{}",
            format!("string stack underflow, script: {}", self.script.info.name)
        );
        std::mem::take(unsafe { self.string_stack.get_unchecked_mut(self.ssp as usize) })
    }

    /// Pops a sequence of typed script arguments from the operand stacks.
    ///
    /// First pops a type descriptor string from the string stack, where each
    /// character encodes one argument's type: `'s'` for string, anything else
    /// for integer. Then pops that many values from the corresponding stacks
    /// in reverse order (right-to-left) to preserve the original argument
    /// ordering.
    ///
    /// # Returns
    ///
    /// A `Vec<ScriptArgument>` with entries matching the type descriptor's
    /// order (left-to-right).
    ///
    /// # Side Effects
    ///
    /// * Pops one string (the type descriptor) via [`pop_string`](Self::pop_string).
    /// * Pops N additional values from the int and/or string stacks, where N
    ///   is the length of the type descriptor.
    ///
    /// # Call Stack
    ///
    /// **Calls:** [`pop_string`](Self::pop_string), [`pop_int`](Self::pop_int).
    ///
    /// **Called by:** Player opcode handlers (`ops::player`) for queue and
    /// timer operations that forward arguments to future script invocations.
    #[inline(always)]
    pub(crate) fn pop_script_args(&mut self) -> Vec<ScriptArgument> {
        // Count chars, not bytes. type chars such as dbrows `Ð` are two bytes,
        // and a byte count would shift every later argument one slot along.
        let types = self.pop_string();
        let mut args: Vec<ScriptArgument> = types
            .chars()
            .rev()
            .map(|c| match c {
                's' => ScriptArgument::String(self.pop_string()),
                _ => ScriptArgument::Int(self.pop_int()),
            })
            .collect();
        args.reverse();
        args
    }
}

/// The current execution status of a [`ScriptState`].
///
/// Controls the VM main loop: only [`Running`](Self::Running) scripts are
/// actively interpreted. All other states cause the interpreter to yield
/// control back to the engine, which may resume the script later (for
/// suspended states) or discard it (for terminal states).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionState {
    /// The script was terminated due to an error or forced abort.
    Aborted,
    /// The script is actively executing bytecode.
    Running,
    /// The script completed normally and should be cleaned up.
    Finished,
    /// Suspended to wait for a player movement or interaction to complete.
    Suspended,
    /// Suspended because the player pressed the pause button during a message dialog.
    PauseButton,
    /// Suspended while waiting for the player to enter a number in a count dialog.
    CountDialog,
    /// Rev-647: suspended while waiting for the player to type a string
    /// (`p_stringdialog`), resumed by RESUME_P_STRINGDIALOG.
    #[cfg(since_647)]
    StringDialog,
    /// Rev-647: suspended while waiting for the display-name DB check
    /// requested by `p_setdisplayname`.
    #[cfg(since_647)]
    DisplayNamePending,
    /// Suspended to wait for an NPC movement or interaction to complete.
    NpcSuspended,
    /// Suspended to wait for a world-level event to complete.
    WorldSuspended,
}

/// A typed argument passed to a script invocation.
///
/// Scripts accept a mixed sequence of integer and string arguments. The
/// argument types are encoded in a type descriptor string (e.g. `"sis"` for
/// string-int-string) which is used by [`ScriptState::pop_script_args`] to
/// pop the correct values from the operand stacks.
#[derive(Debug, Clone)]
pub enum ScriptArgument {
    /// A 32-bit signed integer argument.
    Int(i32),
    /// A heap-allocated string argument.
    String(String),
}

/// A saved call frame for a `gosub` instruction.
///
/// When the VM executes a `gosub`, the current script reference, program
/// counter, and where its locals start are captured into a `GoSubFrame` and
/// pushed onto `gosub_frame_stack`. The locals themselves stay in the shared
/// buffers. When the subroutine returns, the frame is popped by
/// [`ScriptState::pop_frame`] to restore execution context.
#[derive(Debug, Clone)]
pub(crate) struct GoSubFrame {
    pub script: Arc<Script>,
    pub pc: i32,
    pub int_base: usize,
    pub string_base: usize,
}

/// A backtrace entry for a `goto` (jump) or `gosub` instruction.
///
/// Unlike [`GoSubFrame`], it does not preserve local variables. Only the
/// script reference and program counter are saved, for stack traces. A
/// `goto` entry stays for the rest of the run; a `gosub` entry is popped by
/// [`ScriptState::pop_frame`] when the proc returns.
#[derive(Debug, Clone)]
pub(crate) struct GoToFrame {
    pub script: Arc<Script>,
    pub pc: i32,
}

/// Priority level for a [`QueuedScript`], controlling execution ordering and
/// pre-emption behavior in the engine's script queue.
///
/// Higher-priority scripts may interrupt or replace lower-priority ones
/// depending on the engine's scheduling rules.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QueuePriority {
    /// Default priority for player-initiated actions.
    Normal,
    /// Extended priority for actions that span multiple ticks (e.g. multi-step interactions).
    Long,
    /// Priority reserved for engine-internal scripts (e.g. login, zone entry).
    Engine,
    /// Low priority that can be displaced by any stronger queue entry.
    Weak,
    /// High priority that displaces weaker queue entries.
    Strong,
    /// Soft priority that runs only if no stronger script is queued.
    Soft,
}

/// A script that has been enqueued for future execution on an entity.
///
/// Queued scripts are held in the engine's per-entity script queue and
/// executed when their `delay` reaches zero. The [`priority`](Self::priority)
/// determines whether this entry can be displaced by or displace other
/// queued scripts.
#[derive(Debug)]
pub struct QueuedScript {
    pub priority: QueuePriority,
    pub script_id: i32,
    pub delay: u16,
    pub args: Option<Vec<ScriptArgument>>,
    // The pid of the player whose script queued it (ScriptState::queue_source).
    #[cfg(since_647)]
    pub source: Option<u16>,
}

/// Priority level for a [`TimedScript`], controlling how the timer interacts
/// with the engine's scheduling.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TimerPriority {
    /// Standard timer priority; executes on schedule regardless of other activity.
    Normal,
    /// Soft timer priority; may be skipped if a higher-priority action is pending.
    Soft,
}

/// A script scheduled to execute at regular intervals on a game clock.
///
/// Unlike [`QueuedScript`], a `TimedScript` fires repeatedly every
/// [`interval`](Self::interval) ticks, tracked against the engine's
/// [`clock`](Self::clock). The timer persists until explicitly cancelled.
#[derive(Debug)]
pub struct TimedScript {
    pub clock: u32,
    pub args: Option<Vec<ScriptArgument>>,
    pub script_id: i32,
    pub interval: u16,
    pub priority: TimerPriority,
}

/// A snapshot reference to an NPC entity in the game world.
///
/// Stored in [`ScriptState::active_npc`] / [`ScriptState::active_npc2`] to
/// give the running script access to a specific NPC.
#[derive(Debug, Clone, Copy)]
pub struct NpcRef {
    pub nid: u16,
    pub id: u16,
    pub coord: CoordGrid,
}

/// A reference to a location (interactable world object) entity.
///
/// Stored in [`ScriptState::active_loc`] / [`ScriptState::active_loc2`] to
/// give the running script access to a specific location on the map.
///
/// It names one loc by its tile, layer and the serial its zone gave it, so it
/// keeps finding that loc as it changes, is removed or respawns, and finds
/// nothing once a loc a script added is gone, even if another lands on the
/// tile. `id`, `shape` and `angle` are the loc as last seen; ops that read
/// them get the loc as it is now through `ScriptEngine::live_loc`.
#[derive(Debug, Clone, Copy)]
pub struct LocRef {
    pub coord: CoordGrid,
    pub id: u16,
    pub shape: LocShape,
    pub angle: LocAngle,
    pub layer: LocLayer,
    pub serial: LocSerial,
}

/// A reference to a ground object (item) entity in the game world.
///
/// Stored in [`ScriptState::active_obj`] / [`ScriptState::active_obj2`] to
/// give the running script access to a specific ground item.
///
/// It names one obj by the serial its zone gave it, so it keeps finding that
/// obj as it restacks, is revealed or respawns, and finds nothing once the obj
/// is gone, even if another of the same id lands on the tile. Ops that need
/// the count or the obj itself look it up through the serial.
#[derive(Debug, Clone, Copy)]
pub struct ObjRef {
    pub coord: CoordGrid,
    pub id: u16,
    pub serial: ObjSerial,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rs_pack::cache::script::ScriptInfo;

    fn state() -> ScriptState {
        let script = Script {
            id: 0,
            int_arg_count: 0,
            string_arg_count: 0,
            int_local_count: 0,
            string_local_count: 0,
            opcodes: Box::default(),
            int_operands: Box::default(),
            strings: Box::default(),
            switch_tables: Box::default(),
            info: ScriptInfo {
                name: "test".into(),
                path: "test".into(),
                lookup: -1,
                params: Box::default(),
                pcs: Box::default(),
                lines: Box::default(),
            },
        };
        ScriptState::new(Arc::new(script), None)
    }

    /// The values in `args`, in order.
    fn describe(args: &[ScriptArgument]) -> Vec<String> {
        args.iter()
            .map(|arg| match arg {
                ScriptArgument::Int(v) => v.to_string(),
                ScriptArgument::String(v) => format!("{v:?}"),
            })
            .collect()
    }

    #[test]
    fn string_operand_reads_the_string_table() {
        let script = Script {
            opcodes: Box::new([3, 0, 3]),
            int_operands: Box::new([0, 7, 1]),
            strings: Box::new(["hi".into(), "yo".into()]),
            ..Arc::into_inner(state().script).unwrap()
        };
        let mut s = ScriptState::new(Arc::new(script), None);
        s.pc = 2;
        assert_eq!(s.string_operand(), "yo");
        s.pc = 0;
        assert_eq!(s.string_operand(), "hi");
    }

    #[test]
    fn multibyte_type_chars_keep_args_in_place() {
        let mut s = state();
        for v in [7, 1129, 5, 1741, 1, 14] {
            s.push_int(v);
        }
        s.push_string("Ðoioii");
        assert_eq!(
            describe(&s.pop_script_args()),
            ["7", "1129", "5", "1741", "1", "14"]
        );
        assert_eq!((s.isp, s.ssp), (0, 0));
    }

    #[test]
    fn mixed_int_and_string_args() {
        let mut s = state();
        s.push_int(3);
        s.push_string("a");
        s.push_int(9);
        s.push_string("b");
        s.push_string("þsis");
        assert_eq!(describe(&s.pop_script_args()), ["3", "\"a\"", "9", "\"b\""]);
        assert_eq!((s.isp, s.ssp), (0, 0));
    }

    /// A script with these local and arg counts, leaked so it can be gosub'd.
    fn proc(ints: u16, int_args: u16, strings: u16, string_args: u16) -> &'static Arc<Script> {
        Box::leak(Box::new(Arc::new(Script {
            int_local_count: ints,
            int_arg_count: int_args,
            string_local_count: strings,
            string_arg_count: string_args,
            ..Arc::into_inner(state().script).unwrap()
        })))
    }

    #[test]
    fn gosub_keeps_each_frames_locals() {
        let mut s = ScriptState::new(Arc::clone(proc(2, 0, 1, 0)), None);
        s.int_locals[1] = 7;
        s.string_locals[0] = "root".into();

        s.push_int(5);
        s.push_string("arg");
        s.gosub_frame(proc(3, 1, 1, 1)).unwrap();
        s.int_locals[s.int_base + 2] = 9;
        assert_eq!(s.int_locals[s.int_base..], [5, 0, 9]);
        assert_eq!(s.string_locals[s.string_base..], ["arg"]);

        s.gosub_frame(proc(1, 0, 0, 0)).unwrap();
        assert_eq!(s.int_locals[s.int_base..], [0]);
        s.pop_frame();
        assert_eq!(s.int_locals[s.int_base..], [5, 0, 9]);
        s.pop_frame();

        assert_eq!((s.int_base, s.string_base), (0, 0));
        assert_eq!(s.int_locals, [0, 7]);
        assert_eq!(s.string_locals, ["root"]);
    }

    #[test]
    fn reused_state_overwrites_discarded_and_joined_strings() {
        let script = proc(0, 0, 1, 1);
        let mut s = ScriptState::new(Arc::clone(script), None);
        for text in ["long stale contents", "", "rune ü 水", "x"] {
            s.push_string("discarded");
            s.drop_strings(1);
            s.push_string("old left");
            s.push_string("old right");
            s.join_strings(2);

            s.reset(
                Arc::clone(script),
                None,
                None,
                Some(vec![ScriptArgument::String(text.into())]),
            );

            assert_eq!(s.ssp, 0);
            s.push_string("new ");
            s.push_string_local(0);
            s.join_strings(2);
            assert_eq!(s.peek_string(0), format!("new {text}"));
            s.drop_strings(1);
            s.push_string(text);
            assert_eq!(s.pop_string(), text);
            assert_eq!(s.ssp, 0);
        }
    }
}
