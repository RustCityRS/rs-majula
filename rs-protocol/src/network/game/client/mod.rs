#[cfg(since_647)]
pub use crate::network::game::client_prot::{ClientProt, ClientProtInfo};
#[cfg(since_647)]
pub use crate::network::game::client_prot_category::ClientProtCategory;
pub use crate::network::game::client_prot_message::ClientProtMessage;

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
#[cfg(since_647)]
pub mod apcoordt;
pub mod chat_setmode;
#[cfg(since_647)]
pub mod clan_joinchat_leavechat;
#[cfg(since_647)]
pub mod clan_kickuser;
#[cfg(since_647)]
pub mod clanchannel_kickuser;
#[cfg(since_647)]
pub mod click_world_map;
pub mod client_cheat;
#[cfg(since_647)]
pub mod client_detailoptions_status;
pub mod close_modal;
#[cfg(since_647)]
pub mod create_check_name;
#[cfg(since_254)]
pub mod event_applet_focus;
#[cfg(any(rev = "225", since_254))]
pub mod event_camera_position;
#[cfg(since_647)]
pub mod event_keyboard;
#[cfg(since_254)]
pub mod event_mouse_click;
#[cfg(since_254)]
pub mod event_mouse_move;
#[cfg(before_274)]
pub mod event_tracking;
#[cfg(since_647)]
pub mod face_square;
#[cfg(since_647)]
pub mod friend_setrank;
pub mod friendlist_add;
pub mod friendlist_del;
pub mod idk_savedesign;
pub mod idle_timer;
pub mod if_button;
#[cfg(since_647)]
pub mod if_button1;
#[cfg(since_647)]
pub mod if_button10;
#[cfg(since_647)]
pub mod if_button2;
#[cfg(since_647)]
pub mod if_button3;
#[cfg(since_647)]
pub mod if_button4;
#[cfg(since_647)]
pub mod if_button5;
#[cfg(since_647)]
pub mod if_button6;
#[cfg(since_647)]
pub mod if_button7;
#[cfg(since_647)]
pub mod if_button8;
#[cfg(since_647)]
pub mod if_button9;
#[cfg(since_647)]
pub mod if_buttond;
#[cfg(since_647)]
pub mod if_buttont;
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
#[cfg(since_647)]
pub mod map_build_loading;
pub mod message_private;
pub mod message_public;
#[cfg(since_647)]
pub mod message_quickchat_private;
#[cfg(since_647)]
pub mod message_quickchat_public;
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
#[cfg(since_647)]
pub mod oploc6;
pub mod oploct;
pub mod oplocu;
pub mod opnpc1;
pub mod opnpc2;
pub mod opnpc3;
pub mod opnpc4;
pub mod opnpc5;
#[cfg(since_647)]
pub mod opnpc6;
pub mod opnpct;
pub mod opnpcu;
pub mod opobj1;
pub mod opobj2;
pub mod opobj3;
pub mod opobj4;
pub mod opobj5;
#[cfg(since_647)]
pub mod opobj6;
pub mod opobjt;
pub mod opobju;
pub mod opplayer1;
#[cfg(since_647)]
pub mod opplayer10;
pub mod opplayer2;
pub mod opplayer3;
pub mod opplayer4;
#[cfg(since_254)]
pub mod opplayer5;
#[cfg(since_647)]
pub mod opplayer6;
#[cfg(since_647)]
pub mod opplayer7;
#[cfg(since_647)]
pub mod opplayer8;
#[cfg(since_647)]
pub mod opplayer9;
pub mod opplayert;
pub mod opplayeru;
pub mod rebuild_get_maps;
#[cfg(since_647)]
pub mod reflection_check_reply;
#[cfg(since_647)]
pub mod resume_p_clanforumqfcdialog;
pub mod resume_p_countdialog;
#[cfg(since_647)]
pub mod resume_p_hsldialog;
#[cfg(since_647)]
pub mod resume_p_namedialog;
#[cfg(since_647)]
pub mod resume_p_objdialog;
#[cfg(since_647)]
pub mod resume_p_stringdialog;
pub mod resume_pause_button;
pub mod send_snapshot;
#[cfg(since_647)]
pub mod set_chatfiltersettings;
#[cfg(since_647)]
pub mod sound_song_preloaded;
#[cfg(since_647)]
pub mod transmitvar_verifyid;
pub mod tut_clickside;
#[cfg(since_647)]
pub mod url_request;
#[cfg(since_647)]
pub mod video_end;
#[cfg(since_647)]
pub mod window_status;
#[cfg(since_647)]
pub mod world_latency;
#[cfg(since_647)]
pub mod worldlist_fetch;

pub fn pack_coord(x: u16, z: u16) -> u32 {
    ((z & 0x3FFF) as u32) | (((x & 0x3FFF) as u32) << 14)
}

pub fn unpack_coord(packed: u32) -> (u16, u16) {
    let x = (packed >> 14) as u16;
    let z = (packed & 0x3FFF) as u16;
    (x, z)
}
