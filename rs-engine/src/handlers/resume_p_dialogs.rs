use crate::active_player::ActivePlayer;
use crate::core::{engine, engine_mut};
use crate::handlers::ClientGameHandler;
use rs_pack::types::DummyItem;
use rs_protocol::network::game::client_prot::{
    ResumePClanforumqfcDialog, ResumePHslDialog, ResumePNameDialog, ResumePObjDialog,
};
use rs_vm::ScriptError;
use rs_vm::engine::ScriptEngine;
use rs_vm::state::{ExecutionState, ScriptState};
use rs_vm::subject::ScriptSubject;

/// Handles the `ResumePNameDialog` client protocol message.
///
/// The name typed into `meslayer_mode8` (at most 12 printable characters)
/// becomes `last_string` for the script waiting in `p_namedialog`.
impl ClientGameHandler for ResumePNameDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.name.chars().count() > 12 || !self.name.chars().all(printable) {
            return Err(ScriptError::Client(format!("Invalid name: {}", self.name)));
        }
        resume(active, ExecutionState::NameDialog, |state| {
            state.last_string = Some(self.name);
        })
    }
}

/// Handles the `ResumePObjDialog` client protocol message.
///
/// The obj picked in the stockmarket search becomes `last_int` for the script
/// waiting in `p_objdialog`. The search (`oc_find` with stockmarket only)
/// lists no cert, lent, bought or dummy obj.
impl ClientGameHandler for ResumePObjDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        let listed = engine().objs().get_by_id(self.id).is_some_and(|obj| {
            obj.stockmarket
                && obj.certtemplate.is_none()
                && obj.lenttemplate.is_none()
                && obj.boughttemplate.is_none()
                && obj.dummyitem == DummyItem::None
        });
        if !listed {
            return Err(ScriptError::Client(format!(
                "Obj is not in the stockmarket search: {}",
                self.id
            )));
        }
        resume(active, ExecutionState::ObjDialog, |state| {
            state.last_int = Some(self.id as i32);
        })
    }
}

/// Handles the `ResumePHslDialog` client protocol message.
///
/// The colour picked in a clan or skillcape colour picker, any 16-bit hsl,
/// becomes `last_int` for the script waiting in `p_hsldialog`.
impl ClientGameHandler for ResumePHslDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        resume(active, ExecutionState::HslDialog, |state| {
            state.last_int = Some(self.hsl as i32);
        })
    }
}

/// Handles the `ResumePClanforumqfcDialog` client protocol message.
///
/// The text typed or pasted into `clan_qfc_input` is at most 13 characters.
/// The script waiting in `p_clanforumqfcdialog` gets the quick find code it
/// spells as `last_string`, or "" for an empty entry or one that is no code.
impl ClientGameHandler for ResumePClanforumqfcDialog {
    fn handle(self, active: &mut ActivePlayer) -> Result<(), ScriptError> {
        if self.text.chars().count() > 13 {
            return Err(ScriptError::Client(format!(
                "Invalid quick find code: {}",
                self.text
            )));
        }
        let code = clanforumqfc(&self.text).unwrap_or_default();
        resume(active, ExecutionState::ClanforumqfcDialog, |state| {
            state.last_string = Some(code);
        })
    }
}

/// Resumes the player's script waiting in `dialog`, after `answer` writes the
/// client's reply into it.
fn resume(
    active: &mut ActivePlayer,
    dialog: ExecutionState,
    answer: impl FnOnce(&mut ScriptState),
) -> Result<(), ScriptError> {
    let Some(state) = &mut active.player.state.active_script else {
        return Err(ScriptError::Client(
            "Player does not have an active script!".to_string(),
        ));
    };
    if state.execution != dialog {
        return Err(ScriptError::Client(format!(
            "Player does not have an active {dialog:?}!"
        )));
    }
    answer(state);
    engine_mut().run_script_by_state(
        (**state).clone(),
        Some(ScriptSubject::Player(active.player.uid)),
        Some(true),
        Some(true),
    )
}

/// Whether the client's text input takes `c` (its `char_isprintable`).
fn printable(c: char) -> bool {
    matches!(c, ' '..='~' | '\u{a0}'..='\u{ff}' | '€' | 'Œ' | '—' | 'œ' | 'Ÿ')
}

/// The clan-forum quick find code `text` spells: a base-36 long, as
/// `clanforumqfc_tostring` shows it (upper case, no leading zeros).
fn clanforumqfc(text: &str) -> Option<String> {
    if !text.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    i64::from_str_radix(text, 36).ok()?;
    let digits = text.trim_start_matches('0');
    Some(if digits.is_empty() {
        "0".to_string()
    } else {
        digits.to_ascii_uppercase()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_find_codes_are_base_36_longs() {
        assert_eq!(
            clanforumqfc("1y2p0ij32e8e7").as_deref(),
            Some("1Y2P0IJ32E8E7")
        );
        assert_eq!(clanforumqfc("00ab").as_deref(), Some("AB"));
        assert_eq!(clanforumqfc("000").as_deref(), Some("0"));
        assert_eq!(clanforumqfc("1y2p0ij32e8e8"), None);
        assert_eq!(clanforumqfc(""), None);
        assert_eq!(clanforumqfc("-1"), None);
        assert_eq!(clanforumqfc("+1"), None);
        assert_eq!(clanforumqfc("ab cd"), None);
        assert_eq!(clanforumqfc("ab\n"), None);
    }

    #[test]
    fn printable_is_the_clients_set() {
        assert!(" ~a\u{a0}\u{ff}€Œ—œŸ".chars().all(printable));
        assert!(!"\n\u{7f}\u{9f}‚™\u{100}".chars().any(printable));
    }
}
