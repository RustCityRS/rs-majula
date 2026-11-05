//! Drives an npc conversation as the 2011 client does, for quest and town npc
//! tests.
//!
//! [`talk_to`] sends Talk-to (OPNPC1). [`converse`] then cycles the engine:
//! on each chat box it clicks "Click here to continue", and on each
//! multi-choice box the option whose text starts with the next of the given
//! choices. Each click is a RESUME_PAUSEBUTTON (`p2(slot) p4_alt2(com)`) on an
//! open interface, which is all the handler checks
//! (`handlers/resume_pause_button.rs`).
//!
//! The dialogue interfaces are content's (`interface_chat/scripts/chat.rs2`):
//! - player chat 64-67 (chat1-4);
//! - npc chat 241-244 (npcchat1-4);
//! - message boxes 210-214;
//! - objbox 519 and doubleobjbox 131;
//! - multi-choice multi2-5 (228, 230, 232, 234), whose options are
//!   components 2 onwards.

use rs_engine::Engine;
use rs_engine::harness::HarnessClient;
use rs_protocol::network::game::client_prot::ClientProt;
use rs_protocol::network::game::server_prot::ServerProt;
use std::collections::HashMap;

const CHAT_BOXES: &[u16] = &[
    64, 65, 66, 67, 131, 210, 211, 212, 213, 214, 241, 242, 243, 244, 519,
];
/// (interface, option count).
const MULTI: &[(u16, u16)] = &[(228, 2), (230, 3), (232, 4), (234, 5)];
const NO_SLOT: i32 = 0xFFFF;

/// What a conversation showed.
#[derive(Debug, Default)]
pub struct Conversation {
    pub texts: Vec<String>,
    pub messages: Vec<String>,
    pub unanswered: Option<Vec<String>>,
}

impl Conversation {
    /// Whether the texts set during it contain `needle`. A chat box sets each
    /// wrapped line as its own text, so the lines are joined with spaces first.
    pub fn said(&self, needle: &str) -> bool {
        self.texts.join(" ").contains(needle)
    }
}

/// Talk-to on npc `nid` (OPNPC1: `p1_alt2(ctrl) p2(nid)`).
pub fn talk_to(client: &mut HarnessClient, nid: u16) {
    let p = [128u8, (nid >> 8) as u8, nid as u8];
    client.send(ClientProt::OpNpc1, &p);
}

/// Each IF_SETTEXT in `packets`: (component hash, text).
pub fn settexts(packets: &[(u16, Vec<u8>)]) -> Vec<(u32, String)> {
    let opcode = ServerProt::IfSetText as u16;
    packets
        .iter()
        .filter(|(op, p)| *op == opcode && p.len() >= 4)
        .map(|(_, p)| {
            let com = u32::from_be_bytes([p[0], p[1], p[2], p[3]]);
            let end = p[4..]
                .iter()
                .position(|&b| b == 0)
                .map_or(p.len(), |i| 4 + i);
            (com, String::from_utf8_lossy(&p[4..end]).into_owned())
        })
        .collect()
}

fn resume(client: &mut HarnessClient, iface: u16, com: u16) {
    let v = ((iface as i32) << 16) | com as i32;
    let p = [
        (NO_SLOT >> 8) as u8,
        NO_SLOT as u8,
        (v >> 8) as u8,
        v as u8,
        (v >> 24) as u8,
        (v >> 16) as u8,
    ];
    client.send(ClientProt::ResumePauseButton, &p);
}

/// Cycles up to `max` ticks, answering every dialogue box: continue on a chat
/// box, and on a multi-choice box the option starting with the next of
/// `choices`. Once a dialogue has opened, it stops when none has been open for
/// 4 ticks (walking to the npc comes first). If a choice
/// box offers none of the wanted text, it stops and records the options in
/// `unanswered`.
pub fn converse(
    engine: &mut Engine,
    client: &mut HarnessClient,
    choices: &[&str],
    max: usize,
) -> Conversation {
    let mut out = Conversation::default();
    let mut latest: HashMap<u32, String> = HashMap::new();
    let mut next = 0;
    let mut idle = 0;
    let mut seen = false;
    for _ in 0..max {
        engine.cycle();
        let packets = client.packets();
        for (com, text) in settexts(&packets) {
            out.texts.push(text.clone());
            latest.insert(com, text);
        }
        out.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        let Some(player) = engine.harness_player(client.pid) else {
            break;
        };
        if let Some(&(iface, n)) = MULTI.iter().find(|(i, _)| player.is_interface_open(*i)) {
            idle = 0;
            seen = true;
            let options: Vec<(u16, String)> = (2..2 + n)
                .map(|c| {
                    let hash = (iface as u32) << 16 | c as u32;
                    (c, latest.get(&hash).cloned().unwrap_or_default())
                })
                .collect();
            let wanted = choices.get(next);
            match wanted.and_then(|w| options.iter().find(|(_, t)| t.trim().starts_with(w))) {
                Some(&(com, _)) => {
                    resume(client, iface, com);
                    next += 1;
                }
                None => {
                    out.unanswered = Some(options.into_iter().map(|(_, t)| t).collect());
                    break;
                }
            }
        } else if let Some(&iface) = CHAT_BOXES.iter().find(|i| player.is_interface_open(**i)) {
            idle = 0;
            seen = true;
            resume(client, iface, 0);
        } else if seen {
            idle += 1;
            if idle > 4 {
                break;
            }
        }
    }
    out
}

/// One way through an npc's dialogue: the option taken at each choice box
/// (by index), each choice box met (its options), and what was said.
#[derive(Debug)]
pub struct Branch {
    pub path: Vec<usize>,
    pub menus: Vec<Vec<String>>,
    pub conversation: Conversation,
}

/// Like [`converse`], taking options by index: `path[i]` at the i-th choice
/// box, then the first option, except at a box this talk has already been
/// through, where it takes the last (the way out, as a rule) instead of going
/// round a loop again.
pub fn converse_path(
    engine: &mut Engine,
    client: &mut HarnessClient,
    path: &[usize],
    max: usize,
) -> (Conversation, Vec<Vec<String>>, Vec<usize>) {
    let mut out = Conversation::default();
    let mut menus: Vec<Vec<String>> = Vec::new();
    let mut picks: Vec<usize> = Vec::new();
    let mut latest: HashMap<u32, String> = HashMap::new();
    let mut idle = 0;
    let mut seen = false;
    for _ in 0..max {
        engine.cycle();
        let packets = client.packets();
        for (com, text) in settexts(&packets) {
            out.texts.push(text.clone());
            latest.insert(com, text);
        }
        out.messages.extend(
            HarnessClient::game_messages(&packets)
                .into_iter()
                .map(|(_, t)| t),
        );
        let Some(player) = engine.harness_player(client.pid) else {
            break;
        };
        if let Some(&(iface, n)) = MULTI.iter().find(|(i, _)| player.is_interface_open(*i)) {
            idle = 0;
            seen = true;
            let options: Vec<String> = (2..2 + n)
                .map(|c| {
                    let hash = (iface as u32) << 16 | c as u32;
                    latest.get(&hash).cloned().unwrap_or_default()
                })
                .collect();
            let pick = match path.get(menus.len()) {
                Some(&p) => p.min(options.len() - 1),
                None if menus.contains(&options) => options.len() - 1,
                None => 0,
            };
            menus.push(options);
            picks.push(pick);
            resume(client, iface, 2 + pick as u16);
        } else if let Some(&iface) = CHAT_BOXES.iter().find(|i| player.is_interface_open(**i)) {
            idle = 0;
            seen = true;
            resume(client, iface, 0);
        } else if seen {
            idle += 1;
            if idle > 4 {
                break;
            }
        }
    }
    (out, menus, picks)
}

/// Every way through a dialogue, depth first, up to `max_paths`. `start` puts
/// the world back and opens the talk for each one, returning the client. A
/// branch's siblings are queued for each choice box past its given path, but
/// not for a box the talk had already been through.
pub fn explore(
    engine: &mut Engine,
    mut start: impl FnMut(&mut Engine) -> HarnessClient,
    max_paths: usize,
) -> Vec<Branch> {
    let mut todo: Vec<Vec<usize>> = vec![Vec::new()];
    let mut done = Vec::new();
    while let Some(path) = todo.pop() {
        if done.len() >= max_paths {
            break;
        }
        let mut client = start(engine);
        let (conversation, menus, picks) = converse_path(engine, &mut client, &path, 200);
        for depth in path.len()..menus.len() {
            if menus[..depth].contains(&menus[depth]) {
                continue;
            }
            for alt in (0..menus[depth].len()).rev() {
                if alt != picks[depth] {
                    let mut next = picks[..depth].to_vec();
                    next.push(alt);
                    todo.push(next);
                }
            }
        }
        done.push(Branch {
            path: picks,
            menus,
            conversation,
        });
    }
    done
}
