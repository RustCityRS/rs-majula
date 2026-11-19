use super::{LobbySession, LobbyVm};
use crate::player::LobbyPlayer;
use rs_crypto::isaac::IsaacPair;
use rs_engine::{ClientIO, create_io};
use rs_pack::cache::r#if::IfTypeProvider;
use rs_pack::cache::provider::{CacheType, TypeProvider};
use rs_pack::cache::script::ScriptProvider;
use rs_pack::cache::varc::VarClientTypeProvider;
use rs_pack::cache::varp::VarPlayerTypeRaw;
use rs_pack::cache::varpbit::VarpBitType;
use rs_pack::pack::pack_registry::PackFile;
use rs_pack::pack::packed_data::PackedData;
use rs_pack::startup::LobbyAssets;
use rs_pack::types::{IfEvents, ScriptVarType};
use rs_protocol::network::game::server::if_setcolour::IfSetColour;
use rs_ver::TypeId;
use rs_vm::ScriptError;
use rs_vm::engine::ScriptVars;
use rs_vm::trigger::ServerTriggerType;
use runec::opcodes::{
    CONSOLE, ERROR, IF_SETCOLOUR, IF_SETEVENTS, IF_SETRESUMEBUTTONS, P_LOGOUT, PUSH_VARC_INT,
};
use runec::project::{Options, Project, Target};
use runec::sources::{Scope, Visibility};
use runec::symbol::SymbolRegistry;
use runec::types::Type;
use runec::writer::encode_bundle;
use std::net::Ipv4Addr;
use std::sync::Arc;

fn assets(source: &str) -> LobbyAssets {
    let mut symbols = SymbolRegistry::new();
    for (name, opcode, arity) in [
        ("if_setcolour", IF_SETCOLOUR, 2),
        ("if_setevents", IF_SETEVENTS, 8),
        ("if_setresumebuttons", IF_SETRESUMEBUTTONS, 5),
    ] {
        symbols.register_command(
            name.into(),
            i32::from(opcode),
            vec![Type::Int; arity],
            vec![],
        );
    }
    let mut visibility = Visibility::default();
    for (name, ty, id, category) in [
        ("base", Type::Int, 0, "varp"),
        ("bits", Type::Int, 0, "varpbit"),
        ("snapshot", Type::Int, 0, "varc"),
        ("message", Type::String, 0, "varcstr"),
        ("copy", Type::String, 1, "varcstr"),
    ] {
        symbols.register_game_var(name.into(), ty, id, category.into());
        visibility.declare(category, name, Scope::Lobby);
    }
    symbols.set_visibility(visibility);
    let mut project = Project::new(Target::Lobby, symbols, Options::default());
    project.open("test.ls2", Scope::Lobby, 1, source.to_owned());
    let result = project.compile();
    assert!(
        !result.analysis.diagnostics.has_errors(),
        "{:?}",
        result.analysis.diagnostics
    );
    let mut files = Vec::new();
    for script in result.artifacts {
        let id = script.id as usize;
        if id >= files.len() {
            files.resize_with(id + 1, Vec::new);
        }
        files[id] = script.bytes;
    }
    let (dat, idx) = encode_bundle(&files);
    let mut bit = VarpBitType::new(TypeId::new(0));
    bit.start_bit = 1;
    bit.end_bit = 2;
    let mut varcs = PackedData::new(2);
    for var_type in [ScriptVarType::Int, ScriptVarType::Obj] {
        varcs.start_entry();
        varcs.p1(1);
        varcs.p1(var_type as u8);
        varcs.finish_entry();
    }
    LobbyAssets {
        commands: PackFile::parse(include_str!(concat!(
            "../../../content/",
            env!("REV"),
            "/pack/ls2_command.pack"
        ))),
        seq_count: 0,
        scripts: ScriptProvider::from_bytes(&dat, &idx),
        varps: TypeProvider::from_types(vec![VarPlayerTypeRaw::new(TypeId::new(0))], ()),
        varpbits: TypeProvider::from_types(vec![bit], ()),
        varcs: VarClientTypeProvider::from_indexed_bytes(&varcs, &PackedData::new(0)).unwrap(),
        interfaces: IfTypeProvider {
            names: Default::default(),
            types: Default::default(),
        },
        layout: Default::default(),
        toplevel: 0,
    }
}

fn connection() -> ClientIO {
    create_io(IsaacPair::new(&[0; 4], &[0; 4]))
}

#[test]
fn lobby_variables_and_signed_event_slots_survive_shared_dispatch() {
    let assets = assets(
        r#"[login,_]
        %base = 16;
        %bits = 3;
        %snapshot = %base;
        %bits = 8;
        %message = "ready";
        %copy = %message;
        if_setevents(17, -1, 0, 3, 4, 2, 1, 0);
    "#,
    );
    let mut player = LobbyPlayer::new(connection().handle, 1, Ipv4Addr::LOCALHOST.into(), 1, 0);
    assert!(
        player
            .run_trigger(
                &assets,
                &LobbyVm::new(&assets),
                ServerTriggerType::Login,
                None
            )
            .unwrap()
    );
    assert_eq!(player.varcs[&0], 22);
    assert_eq!(player.varps[&0].as_int(), 16);
    assert_eq!(player.strings[&1], "ready");
    let settings = IfEvents::new(3, 4, 2, true, false);
    assert_eq!(player.events[&(17, u16::MAX)], settings);
    assert_eq!(player.events[&(17, 0)], settings);
    assert_eq!(player.events.len(), 2);
}

#[test]
fn unset_varcs_read_their_types_default() {
    let assets = assets("[login,_]");
    let mut player = LobbyPlayer::new(connection().handle, 1, Ipv4Addr::LOCALHOST.into(), 1, 0);
    let session = LobbySession {
        player: &mut player,
        assets: &assets,
    };
    assert_eq!(session.varc(0), 0);
    assert_eq!(session.varc(1), -1);
    assert_eq!(session.varc(2), 0);
}

#[test]
fn compiled_login_calls_lobby_proc_and_sends_rgb15() {
    let assets = assets(
        "[login,_]\n~paint(17);\n[proc,paint](int $com)\nif_setcolour($com, 16711680);\nif_setresumebuttons(1, 2, 3, 4, 5);",
    );
    let vm = LobbyVm::new(&assets);
    let mut conn = connection();
    let mut player = LobbyPlayer::new(conn.handle, 1, Ipv4Addr::LOCALHOST.into(), 1, 0);
    assert!(
        player
            .run_trigger(&assets, &vm, ServerTriggerType::Login, None)
            .unwrap()
    );
    assert_eq!(player.resume_buttons, [1, 2, 3, 4, 5]);
    player.handle.flush();

    let mut expected = connection();
    expected.handle.write(IfSetColour {
        com: 17,
        colour: 0x7c00,
    });
    expected.handle.flush();
    assert_eq!(
        conn.bytes_rx.try_recv().unwrap(),
        expected.bytes_rx.try_recv().unwrap()
    );
    assert!(
        !player
            .run_trigger(&assets, &vm, ServerTriggerType::Logout, None)
            .unwrap()
    );
}

#[test]
fn lobby_rejects_world_opcodes_and_secondary_player_variables() {
    let mut assets = assets("[login,_]");
    let id = assets
        .scripts
        .get_by_lookup(ServerTriggerType::Login as i32)
        .unwrap()
        .id as usize;
    let vm = LobbyVm::new(&assets);
    let mut player = LobbyPlayer::new(connection().handle, 1, Ipv4Addr::LOCALHOST.into(), 1, 0);
    assert!(vm.ops.contains(CONSOLE));
    assert!(vm.ops.contains(IF_SETRESUMEBUTTONS));
    for (opcode, operand) in [(P_LOGOUT, 0), (ERROR, 0), (PUSH_VARC_INT, 1 << 16)] {
        let script = Arc::get_mut(assets.scripts.scripts[id].as_mut().unwrap()).unwrap();
        script.opcodes[0] = opcode;
        script.int_operands[0] = operand;
        let err = player
            .run_trigger(&assets, &vm, ServerTriggerType::Login, None)
            .unwrap_err();
        match opcode {
            P_LOGOUT | ERROR => {
                assert!(matches!(err, ScriptError::UnknownOpcode(id) if id == opcode))
            }
            _ => assert!(matches!(err, ScriptError::NoSubject)),
        }
    }
}
