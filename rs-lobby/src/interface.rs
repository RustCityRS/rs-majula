use crate::player::LobbyPlayer;
use crate::script::LobbySession;
use rs_pack::startup::InterfaceLayout;
use rs_pack::types::IfEvents;
use rs_protocol::network::game::server::{
    if_movesub::IfMovesub,
    if_opentop::IfOpentop,
    if_setanim::IfSetAnim,
    if_setcolour::IfSetColour,
    if_setevents::IfSetEvents,
    if_setgraphic::IfSetGraphic,
    if_sethide::IfSetHide,
    if_setmodel::IfSetModel,
    if_setnpchead::IfSetNpcHead,
    if_setobject::IfSetObject,
    if_setplayerhead::IfSetPlayerHead,
    if_setposition::IfSetPosition,
    if_setscrollpos::IfSetScrollPos,
    if_settext::IfSetText,
    run_client_script::{RunClientScript, ScriptParam},
};
use rs_ver::TypeId;
use rs_vm::Result;
use rs_vm::engine::ScriptVars;
use rs_vm::session::ScriptInterface;
use rs_vm::state::ScriptArgument;

impl ScriptInterface for LobbySession<'_> {
    fn if_close(&mut self, _clear: bool) -> Result<()> {
        self.player.close_modals();
        Ok(())
    }

    fn if_close_others(&mut self, com: u16) -> Result<()> {
        if self
            .player
            .subs
            .values()
            .any(|&(id, kind)| kind == 0 && id != com)
        {
            self.player.close_modals();
        }
        Ok(())
    }

    fn if_openchat(&mut self, id: u16) {
        let com = self.assets.layout.dialogue_slot(id);
        if com != 0 {
            self.player.open_sub(com, id, 0);
        }
    }

    fn if_openmain_side(&mut self, id: u16, side: u16) {
        self.if_openmain(id);
        if let Some(&slot) = self.assets.layout.overlay_tab.get(&self.player.top) {
            let com = (i32::from(self.player.top) << 16) | i32::from(slot);
            self.player.open_sub(com, side, 1);
        }
    }

    fn if_openmain(&mut self, id: u16) {
        self.player.open_main(id, &self.assets.layout);
    }

    fn if_openside(&mut self, id: u16) {
        self.player
            .open_slot(self.assets.layout.overlay_tab.get(&self.player.top), id, 1);
    }

    fn if_setanim(&mut self, com: i32, seq: TypeId) {
        self.player.handle.write(IfSetAnim { com, seq });
    }

    fn if_setcolour(&mut self, com: i32, colour: u16) {
        self.player.handle.write(IfSetColour { com, colour });
    }

    fn if_sethide(&mut self, com: i32, hidden: bool) {
        self.player.handle.write(IfSetHide { com, hidden });
    }

    fn if_setmodel(&mut self, com: i32, model: TypeId) {
        self.player.handle.write(IfSetModel { com, model });
    }

    fn if_setnpchead(&mut self, com: i32, npc: TypeId) {
        self.player.handle.write(IfSetNpcHead { com, npc });
    }

    fn if_setobject(&mut self, com: i32, obj: TypeId, scale: i32) {
        self.player.handle.write(IfSetObject { com, obj, scale });
    }

    fn if_setplayerhead(&mut self, com: i32) {
        self.player.handle.write(IfSetPlayerHead { com });
    }

    fn if_setposition(&mut self, com: i32, x: u16, y: u16) {
        self.player.handle.write(IfSetPosition { com, x, y });
    }

    fn if_setresumebuttons(&mut self, buttons: Option<Vec<i32>>) {
        self.player.resume_buttons = buttons.unwrap_or_default();
    }

    fn if_settab(&mut self, interface: u16, tab: u8) {
        self.player.tabs.insert(tab, interface);
    }

    fn if_settabactive(&mut self, tab: u8) {
        self.sendvarc(self.assets.layout.switch_side, tab.into());
    }

    fn if_settext(&mut self, com: i32, text: &str) {
        self.player.handle.write(IfSetText { com, text });
    }

    fn if_setscrollpos(&mut self, com: i32, y: u16) {
        self.player.handle.write(IfSetScrollPos { com, y });
    }

    fn if_openoverlay(&mut self, id: u16) {
        self.player
            .open_slot(self.assets.layout.overlay.get(&self.player.top), id, 1);
    }

    fn if_addresumebutton(&mut self, button: i32) {
        self.player.resume_buttons.push(button);
    }

    fn if_closesub(&mut self, com: i32) {
        self.player.close_sub(com);
    }

    fn if_movesub(&mut self, from: i32, to: i32) {
        let moving = self.player.subs.remove(&from);
        self.player.close_children(to);
        if let Some(sub) = moving {
            self.player.subs.insert(to, sub);
        }
        self.player.handle.write(IfMovesub {
            com: from,
            com2: to,
        });
    }

    fn if_opensub(&mut self, interface: u16, parent: i32) {
        let holder = (parent as u32 >> 16) as u16;
        let kind = self
            .player
            .subs
            .values()
            .find(|&&(id, _)| id == holder)
            .map_or(1, |&(_, kind)| kind);
        self.player.open_sub(parent, interface, kind);
    }

    fn if_opentop(&mut self, interface: u16, mode: u8) {
        self.player.top = interface;
        self.player.handle.write(IfOpentop {
            interface,
            mode,
            #[cfg(since_742)]
            xtea_keys: [0; 4],
        });
    }

    fn if_setevents(&mut self, com: i32, slot1: i32, slot2: i32, settings: IfEvents) {
        for slot in slot1..=slot2 {
            self.player.events.insert((com, slot as u16), settings);
        }
        self.player.handle.write(IfSetEvents {
            com,
            slot1: slot1 as u16,
            slot2: slot2 as u16,
            settings: settings.0 as i32,
        });
    }

    fn if_setgraphic(&mut self, com: i32, graphic: TypeId) {
        self.player.handle.write(IfSetGraphic { com, graphic });
    }

    fn runclientscript(&mut self, id: i32) {
        self.runclientscript_with_params(id, Vec::new());
    }

    fn runclientscript_with_params(&mut self, id: i32, args: Vec<ScriptArgument>) {
        let params = args
            .into_iter()
            .map(|arg| match arg {
                ScriptArgument::Int(value) => ScriptParam::Int(value),
                ScriptArgument::String(value) => ScriptParam::Str(value),
            })
            .collect();
        self.player.handle.write(RunClientScript {
            script_id: id,
            params,
        });
    }
}

impl LobbyPlayer {
    fn open_main(&mut self, id: u16, layout: &InterfaceLayout) {
        let slots = if layout.wide_screen_ifaces.contains(&id) {
            &layout.wide_screen
        } else {
            &layout.main_screen
        };
        self.open_slot(slots.get(&self.top), id, 0);
    }

    fn open_slot(&mut self, slot: Option<&u16>, id: u16, kind: u8) {
        if let Some(&slot) = slot {
            let com = (i32::from(self.top) << 16) | i32::from(slot);
            if id == u16::MAX {
                self.close_sub(com);
            } else {
                self.open_sub(com, id, kind);
            }
        }
    }
}
