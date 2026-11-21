use crate::script::LobbySession;
use rs_pack::cache::VarValue;
use rs_protocol::network::game::server::{
    varc_large::VarcLarge, varc_small::VarcSmall, varc_str_large::VarcStrLarge,
    varc_str_small::VarcStrSmall, varp_large::VarpLarge, varp_small::VarpSmall,
};
use rs_ver::TypeId;
use rs_vm::engine::ScriptVars;

impl ScriptVars for LobbySession<'_> {
    fn stored_varp(&self, id: u16) -> Option<&VarValue> {
        self.player.varps.get(&id)
    }

    fn set_var(&mut self, id: u16, value: VarValue, transmit: bool) {
        if transmit {
            let val = value.as_int();
            if VarpSmall::fits(val) {
                self.player.handle.write(VarpSmall { id, val: val as u8 });
            } else {
                self.player.handle.write(VarpLarge { id, val });
            }
        }
        self.player.varps.insert(id, value);
    }

    fn varc(&self, id: u16) -> i32 {
        self.player.varcs.get(&id).copied().unwrap_or_else(|| {
            self.assets
                .varcs
                .get_by_id(TypeId::new(id), false)
                .map_or(0, |varc| VarValue::default_for(varc.var_type).as_int())
        })
    }

    fn sendvarc(&mut self, id: u16, val: i32) {
        self.player.varcs.insert(id, val);
        if VarcSmall::fits(val) {
            self.player.handle.write(VarcSmall { id, val: val as u8 });
        } else {
            self.player.handle.write(VarcLarge { id, val });
        }
    }

    fn varc_str(&self, id: u16) -> &str {
        self.player.strings.get(&id).map_or("", String::as_str)
    }

    fn sendvarc_str(&mut self, id: u16, val: &str) {
        if VarcStrSmall::fits(val) {
            self.player.handle.write(VarcStrSmall { id, val });
        } else {
            self.player.handle.write(VarcStrLarge { id, val });
        }
        val.clone_into(self.player.strings.entry(id).or_default());
    }
}
