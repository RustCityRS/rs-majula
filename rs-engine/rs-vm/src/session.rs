use crate::engine::{ScriptEngine, ScriptVars, engine};
#[cfg(since_530)]
use crate::state::ScriptArgument;
use crate::state::ScriptState;
use crate::util::{get_active_player_mut, require_active_player};
use crate::{Result, ScriptError};
use rs_pack::cache::script::Script;
use rs_pack::cache::varp::VarPlayerTypeProvider;
#[cfg(since_254)]
use rs_pack::cache::varpbit::VarpBitTypeProvider;
#[cfg(since_530)]
use rs_pack::types::IfEvents;
use rs_ver::TypeId;
use std::marker::PhantomData;
use std::sync::Arc;

/// Script resources and the player bound to this execution.
pub trait ScriptHost {
    type Interface: ScriptInterface;
    type Vars: ScriptVars;

    fn get_script(&self, id: i32) -> Option<&Arc<Script>>;
    fn seq_count(&self) -> usize;
    fn interface(&mut self, state: &ScriptState, secondary: bool) -> Result<&mut Self::Interface>;
    fn vars(&mut self, state: &ScriptState, secondary: bool) -> Result<PlayerVars<'_, Self::Vars>>;
}

/// Names a host type for any borrow, so one registry serves hosts that borrow
/// per execution.
pub trait HostKind {
    type Host<'a>;
}

pub struct PlayerVars<'a, V> {
    pub player: &'a mut V,
    pub varps: &'a VarPlayerTypeProvider,
    #[cfg(since_254)]
    pub bits: &'a VarpBitTypeProvider,
}

pub(crate) fn active_interface<'a, H: ScriptHost>(
    host: &'a mut H,
    state: &ScriptState,
) -> Result<&'a mut H::Interface> {
    require_active_player(state)?;
    host.interface(state, state.int_operand() != 0)
}

pub(crate) fn active_vars<'a, H: ScriptHost>(
    host: &'a mut H,
    state: &ScriptState,
) -> Result<(u16, PlayerVars<'a, H::Vars>)> {
    let operand = state.int_operand();
    let slot = operand >> 16;
    if !(0..=1).contains(&slot) {
        return Err(ScriptError::NoSubject);
    }
    Ok((operand as u16, host.vars(state, slot != 0)?))
}

pub struct EngineHost<E>(pub(crate) PhantomData<fn() -> E>);

impl<E> HostKind for EngineHost<E> {
    type Host<'a> = Self;
}

impl<E: ScriptEngine + 'static> ScriptHost for EngineHost<E> {
    type Interface = E::Player;
    type Vars = E::Player;

    fn get_script(&self, id: i32) -> Option<&Arc<Script>> {
        engine::<E>().get_script(id)
    }

    fn seq_count(&self) -> usize {
        engine::<E>().seqs().count()
    }

    fn interface(&mut self, state: &ScriptState, secondary: bool) -> Result<&mut E::Player> {
        get_active_player_mut::<E>(state, secondary)
    }

    fn vars(&mut self, state: &ScriptState, secondary: bool) -> Result<PlayerVars<'_, E::Player>> {
        let varps = engine::<E>().varps();
        #[cfg(since_254)]
        let bits = engine::<E>().varpbits();
        Ok(PlayerVars {
            player: get_active_player_mut::<E>(state, secondary)?,
            varps,
            #[cfg(since_254)]
            bits,
        })
    }
}

/// Interface updates that scripts can apply to a connected player.
pub trait ScriptInterface {
    /// Configures which interface buttons are valid resume targets for a
    /// paused script dialog.
    ///
    /// # Arguments
    /// * `buttons` - `Some(Vec<i32>)` with the allowed component IDs, or
    ///   `None` to clear the resume button set.
    #[cfg(any(before_254, since_530))]
    fn if_setresumebuttons(&mut self, buttons: Option<Vec<i32>>);

    /// Appends a single component ID to the player's resume button set, lazily
    /// creating the set if it does not yet exist.
    ///
    /// This is the incremental counterpart to `if_setresumebuttons`: the resume
    /// set is reset whenever a suspended dialog script is cleared, so a dialog
    /// accumulates its buttons one `if_addresumebutton` call at a time.
    ///
    /// # Arguments
    /// * `button` - The component ID to add as a valid resume target.
    #[cfg(since_254)]
    fn if_addresumebutton(&mut self, button: i32);

    /// Rev-647: opens a toplevel (root) interface, with IF_OPENTOP's mode
    /// byte (`2` rebuilds the client's scene behind a loading screen).
    #[cfg(since_530)]
    fn if_opentop(&mut self, com: u16, mode: u8);

    /// Rev-647: mounts a sub-interface at a parent component hash
    /// (`interface << 16 | component`), as the type of the sub that holds it.
    #[cfg(since_530)]
    fn if_opensub(&mut self, sub_id: u16, parent_hash: i32);

    /// Rev-647: closes whatever sub-interface is mounted at the parent
    /// component hash.
    #[cfg(since_530)]
    fn if_closesub(&mut self, component_hash: i32);

    /// Rev-647: moves the sub-interface mounted at `from` onto `to`, closing
    /// whatever `to` held, without reloading it.
    #[cfg(since_530)]
    fn if_movesub(&mut self, from: i32, to: i32);

    /// Rev-647: sets the client-side event mask for a component slot range.
    #[cfg(since_530)]
    fn if_setevents(&mut self, com: i32, slot1: i32, slot2: i32, settings: IfEvents);

    /// Rev-647: invokes a clientscript (cs2) with no parameters.
    #[cfg(since_530)]
    fn runclientscript(&mut self, script_id: i32);

    /// Rev-647: invokes a clientscript (cs2) with typed int/string parameters
    /// (`runclientscript*`).
    #[cfg(since_530)]
    fn runclientscript_with_params(&mut self, script_id: i32, args: Vec<ScriptArgument>);

    /// Closes any open modal interface.
    ///
    /// # Arguments
    /// * `clear` - Whether to also clear the interface state on the client.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// close triggers an error.
    fn if_close(&mut self, clear: bool) -> Result<()>;

    /// Closes every open modal, as [`if_close`](Self::if_close) does, unless
    /// the only one open is `com`.
    ///
    /// # Arguments
    /// * `com` - The interface component ID about to be opened.
    ///
    /// # Returns
    /// `Ok(())` on success, or a [`ScriptError`](crate::ScriptError) if the
    /// close triggers an error.
    fn if_close_others(&mut self, com: u16) -> Result<()>;

    /// Opens a chatbox interface (e.g. a dialog).
    ///
    /// # Arguments
    /// * `id` - The interface component ID to open in the chatbox area.
    fn if_openchat(&mut self, id: u16);

    /// Opens a main-area interface alongside a side-panel interface.
    ///
    /// # Arguments
    /// * `com` - The main interface component ID.
    /// * `side` - The side-panel interface component ID.
    fn if_openmain_side(&mut self, com: u16, side: u16);

    /// Opens a full-screen main-area interface.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_openmain(&mut self, com: u16);

    /// Opens a side-panel interface.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_openside(&mut self, com: u16);

    /// Sets the animation displayed on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `seq` - The sequence (animation) ID to display.
    fn if_setanim(&mut self, com: i32, seq: TypeId);

    /// Sets the colour of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `colour` - The RGB15 colour value.
    fn if_setcolour(&mut self, com: i32, colour: u16);

    /// Shows or hides an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `hide` - `true` to hide, `false` to show.
    fn if_sethide(&mut self, com: i32, hide: bool);

    /// Sets the model displayed on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `model` - The model ID to display.
    fn if_setmodel(&mut self, com: i32, model: TypeId);

    /// Sets the graphic (a sprite) displayed on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `graphic` - The sprite ID to display.
    #[cfg(since_647)]
    fn if_setgraphic(&mut self, com: i32, graphic: TypeId);

    /// Sets an NPC chathead model on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `npc` - The NPC type ID whose head model to display.
    fn if_setnpchead(&mut self, com: i32, npc: TypeId);

    /// Displays an object model on an interface component at the given zoom level.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `obj` - The object (item) type ID.
    /// * `scale` - The zoom level for the model display.
    fn if_setobject(&mut self, com: i32, obj: TypeId, scale: i32);

    /// Sets the player's own chathead model on an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    fn if_setplayerhead(&mut self, com: i32);

    /// Sets the position of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `x` - The X position in interface coordinates.
    /// * `y` - The Y position in interface coordinates.
    fn if_setposition(&mut self, com: i32, x: u16, y: u16);

    /// Sets the scrollbar position on an interface component.
    #[cfg(since_245_2)]
    fn if_setscrollpos(&mut self, com: i32, y: u16);

    /// Opens a modal overlay interface component.
    #[cfg(since_244)]
    fn if_openoverlay(&mut self, com: u16);

    /// Recolors an interface component model, remapping one color to another.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `src` - The source color to replace.
    /// * `dst` - The destination color to apply.
    #[cfg(before_244)]
    fn if_setrecol(&mut self, com: u16, src: u16, dst: u16);

    /// Assigns an interface component to a tab slot.
    ///
    /// # Arguments
    /// * `interface` - The interface ID to place in the tab.
    /// * `tab` - The tab index.
    fn if_settab(&mut self, interface: u16, tab: u8);

    /// Switches the client's currently selected (active) tab.
    ///
    /// # Arguments
    /// * `tab` - The tab index to make active.
    fn if_settabactive(&mut self, tab: u8);

    /// Sets the text content of an interface component.
    ///
    /// # Arguments
    /// * `com` - The interface component ID.
    /// * `text` - The text string to display.
    fn if_settext(&mut self, com: i32, text: &str);
}
