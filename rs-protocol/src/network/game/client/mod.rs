pub mod anticheat_cyclelogic1;
pub mod anticheat_cyclelogic2;
pub mod anticheat_cyclelogic3;
pub mod anticheat_cyclelogic4;
pub mod anticheat_cyclelogic5;
pub mod anticheat_cyclelogic6;
#[cfg(since_254)]
pub mod anticheat_cyclelogic7;
pub mod anticheat_oplogic1;
pub mod anticheat_oplogic2;
pub mod anticheat_oplogic3;
pub mod anticheat_oplogic4;
pub mod anticheat_oplogic5;
pub mod anticheat_oplogic6;
pub mod anticheat_oplogic7;
pub mod anticheat_oplogic8;
pub mod anticheat_oplogic9;
pub mod chat_setmode;
pub mod client_cheat;
pub mod close_modal;
#[cfg(since_254)]
pub mod event_applet_focus;
#[cfg(any(rev = "225", since_254))]
pub mod event_camera_position;
#[cfg(since_254)]
pub mod event_mouse_click;
#[cfg(since_254)]
pub mod event_mouse_move;
pub mod event_tracking;
pub mod friendlist_add;
pub mod friendlist_del;
pub mod idk_savedesign;
pub mod idle_timer;
pub mod if_button;
pub mod ignorelist_add;
pub mod ignorelist_del;
pub mod inv_button1;
pub mod inv_button2;
pub mod inv_button3;
pub mod inv_button4;
pub mod inv_button5;
pub mod inv_buttond;
#[cfg(since_254)]
pub mod map_build_complete;
pub mod message_private;
pub mod message_public;
pub mod move_gameclick;
pub mod move_minimapclick;
pub mod move_opclick;
pub mod no_timeout;
pub mod opheld1;
pub mod opheld2;
pub mod opheld3;
pub mod opheld4;
pub mod opheld5;
pub mod opheldt;
pub mod opheldu;
pub mod oploc1;
pub mod oploc2;
pub mod oploc3;
pub mod oploc4;
pub mod oploc5;
pub mod oploct;
pub mod oplocu;
pub mod opnpc1;
pub mod opnpc2;
pub mod opnpc3;
pub mod opnpc4;
pub mod opnpc5;
pub mod opnpct;
pub mod opnpcu;
pub mod opobj1;
pub mod opobj2;
pub mod opobj3;
pub mod opobj4;
pub mod opobj5;
pub mod opobjt;
pub mod opobju;
pub mod opplayer1;
pub mod opplayer2;
pub mod opplayer3;
pub mod opplayer4;
#[cfg(since_254)]
pub mod opplayer5;
pub mod opplayert;
pub mod opplayeru;
pub mod rebuild_get_maps;
pub mod resume_p_countdialog;
pub mod resume_pause_button;
pub mod send_snapshot;
pub mod tut_clickside;

pub use crate::network::game::client_prot_message::ClientProtMessage;

pub fn pack_coord(x: u16, z: u16) -> u32 {
    ((z & 0x3FFF) as u32) | (((x & 0x3FFF) as u32) << 14)
}

pub fn unpack_coord(packed: u32) -> (u16, u16) {
    let x = (packed >> 14) as u16;
    let z = (packed & 0x3FFF) as u16;
    (x, z)
}
