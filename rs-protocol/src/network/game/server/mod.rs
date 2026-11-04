#[cfg(since_647)]
pub mod appearance;
#[cfg(since_647)]
pub mod cam_forceangle;
#[cfg(before_647)]
pub mod cam_look_at;
#[cfg(since_647)]
pub mod cam_lookat;
#[cfg(before_647)]
pub mod cam_move_to;
#[cfg(since_647)]
pub mod cam_moveto;
#[cfg(since_647)]
pub mod cam_removeroof;
pub mod cam_reset;
pub mod cam_shake;
#[cfg(since_647)]
pub mod cam_smoothreset;
pub mod chat_filter_settings;
#[cfg(since_647)]
pub mod chat_filter_settings_privatechat;
#[cfg(since_647)]
pub mod clanchannel_delta;
#[cfg(since_647)]
pub mod clanchannel_full;
#[cfg(since_647)]
pub mod clansettings_delta;
#[cfg(since_647)]
pub mod clansettings_full;
#[cfg(since_647)]
pub mod create_check_name;
#[cfg(rev = "225")]
pub mod data_land;
#[cfg(rev = "225")]
pub mod data_land_done;
#[cfg(rev = "225")]
pub mod data_loc;
#[cfg(rev = "225")]
pub mod data_loc_done;
#[cfg(before_274)]
pub mod enable_tracking;
#[cfg(before_274)]
pub mod finish_tracking;
#[cfg(since_254)]
pub mod friendlist_loaded;
pub mod hint_arrow;
#[cfg(before_647)]
pub mod if_close;
#[cfg(since_647)]
pub mod if_closesub;
#[cfg(since_647)]
pub mod if_movesub;
#[cfg(before_647)]
pub mod if_openchat;
#[cfg(before_647)]
pub mod if_openmain;
#[cfg(before_647)]
pub mod if_openmain_side;
#[cfg(all(since_244, before_647))]
pub mod if_openoverlay;
#[cfg(before_647)]
pub mod if_openside;
#[cfg(since_647)]
pub mod if_opensub;
#[cfg(since_647)]
pub mod if_opentop;
#[cfg(since_647)]
pub mod if_setangle;
pub mod if_setanim;
pub mod if_setcolour;
#[cfg(since_647)]
pub mod if_setevents;
#[cfg(since_647)]
pub mod if_setgraphic;
pub mod if_sethide;
pub mod if_setmodel;
pub mod if_setnpchead;
pub mod if_setobject;
pub mod if_setplayerhead;
pub mod if_setposition;
#[cfg(any(before_245_2, since_647))]
pub mod if_setrecol;
#[cfg(since_647)]
pub mod if_setretex;
#[cfg(since_245_2)]
pub mod if_setscrollpos;
#[cfg(before_647)]
pub mod if_settab;
#[cfg(before_647)]
pub mod if_settab_active;
#[cfg(since_647)]
pub mod if_settargetparam;
pub mod if_settext;
#[cfg(since_647)]
pub mod if_setvideo;
#[cfg(since_647)]
pub mod jcoins_update;
pub mod last_login_info;
pub mod loc_add_change;
pub mod loc_anim;
#[cfg(since_647)]
pub mod loc_anim_specific;
pub mod loc_del;
#[cfg(before_647)]
pub mod loc_merge;
#[cfg(before_647)]
pub mod logout;
pub mod map_anim;
pub mod map_projanim;
#[cfg(since_647)]
pub mod message_clanchannel;
#[cfg(since_647)]
pub mod message_friendchat;
pub mod message_game;
pub mod message_private;
#[cfg(since_647)]
pub mod message_private_echo;
#[cfg(since_647)]
pub mod message_public;
#[cfg(since_647)]
pub mod message_quickchat_clanchannel;
#[cfg(since_647)]
pub mod message_quickchat_friendchat;
#[cfg(since_647)]
pub mod message_quickchat_private;
#[cfg(since_647)]
pub mod message_quickchat_private_echo;
pub mod midi_jingle;
pub mod midi_song;
#[cfg(all(since_274, before_647))]
pub mod minimap_toggle;
#[cfg(since_647)]
pub mod npc_anim_specific;
pub mod npc_info;
pub mod obj_add;
pub mod obj_count;
pub mod obj_del;
pub mod obj_reveal;
#[cfg(before_647)]
pub mod p_countdialog;
pub mod player_info;
pub mod rebuild_normal;
#[cfg(since_647)]
pub mod rebuild_region;
#[cfg(before_647)]
pub mod reset_anims;
#[cfg(before_647)]
pub mod reset_client_varcache;
#[cfg(since_647)]
pub mod run_client_script;
#[cfg(since_647)]
pub mod set_map_flag;
#[cfg(since_647)]
pub mod set_moveaction;
#[cfg(before_647)]
pub mod set_multiway;
#[cfg(since_254)]
pub mod set_player_op;
#[cfg(since_647)]
pub mod setdraworder;
#[cfg(since_289)]
pub mod sound_area;
#[cfg(since_647)]
pub mod sound_vorbis_area;
#[cfg(since_647)]
pub mod spotanim_specific;
pub mod synth_sound;
#[cfg(before_647)]
pub mod tut_flash;
#[cfg(before_647)]
pub mod tut_open;
#[cfg(before_647)]
pub mod unset_map_flag;
#[cfg(since_647)]
pub mod update_friendchat_channel_full;
#[cfg(since_647)]
pub mod update_friendchat_channel_singleuser;
pub mod update_friendlist;
#[cfg(before_647)]
pub mod update_ignorelist;
#[cfg(since_647)]
pub mod update_ignorelist_full;
pub mod update_inv_full;
#[cfg(before_647)]
pub mod update_inv_partial;
#[cfg(before_647)]
pub mod update_inv_stop_transmit;
#[cfg(before_647)]
pub mod update_pid;
pub mod update_reboot_timer;
pub mod update_runenergy;
#[cfg(before_647)]
pub mod update_runweight;
pub mod update_stat;
pub mod update_zone_full_follows;
pub mod update_zone_partial_enclosed;
pub mod update_zone_partial_follows;
#[cfg(since_647)]
pub mod varbit_large;
#[cfg(since_647)]
pub mod varbit_small;
#[cfg(since_647)]
pub mod varc_large;
#[cfg(since_647)]
pub mod varc_small;
#[cfg(since_647)]
pub mod varc_str_small;
pub mod varp_large;
pub mod varp_small;
#[cfg(since_647)]
pub mod vorbis_sound;

pub use crate::network::game::server_prot_message::ServerProtMessage;

#[cfg(since_647)]
pub use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
pub use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_647)]
pub use crate::network::game::server_prot_priority::ServerProtPriority;
