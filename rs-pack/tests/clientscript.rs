#![cfg(since_647)]

use rs_io::js5::Js5Store;
use rs_pack::archive;
use rs_pack::cs2::{script::Script, source};
use rs_pack::pack::names::AssetNames;
use runec::clientscript::{self, Source, format::Format};
use runec::types::Type;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[test]
#[ignore = "requires the 647 reference cache and generated sources"]
fn sources_without_metadata_match_every_reference_clientscript() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let pack = root.join("content/647/pack");
    let dir = root.join("content/647/scripts/clientscript");
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let sources = clientscript::load_sources(&dir).unwrap();
    let ids = clientscript::script_ids(&pack).unwrap();
    let mut registry = clientscript::registry(&dir, &pack).unwrap();
    for source in &sources {
        assert!(
            !fs::read_to_string(&source.path)
                .unwrap()
                .contains("// @cs2"),
            "{}",
            source.path.display()
        );
        clientscript::register(&source.file, &ids, &mut registry).unwrap();
    }
    let mut compiled = HashMap::new();
    for source in &sources {
        for (id, bytes) in source.compile(&registry, &format).unwrap() {
            assert!(compiled.insert(id as u32, bytes).is_none());
        }
    }
    let mut store = Js5Store::open(root.join("expected")).unwrap();
    let groups = store
        .reference_table(archive::CLIENTSCRIPT)
        .unwrap()
        .groups
        .clone();
    assert!(!groups.is_empty());
    assert_eq!(compiled.len(), groups.len());
    for group in groups {
        let original = store
            .read_group_raw_decompressed(archive::CLIENTSCRIPT, group.id)
            .unwrap()
            .unwrap();
        assert_eq!(
            compiled.remove(&group.id).unwrap(),
            original,
            "clientscript {}",
            group.id
        );
    }
}

#[test]
fn editable_source_preserves_bytes_and_compiles_changes() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("example.cs2");
    let text = "[proc,clientscript_100000](int $input, long $minimum)(int,string,long)\n\
        def_int $items(4);\n\
        $items(2) = 7;\n\
        def_int $i = 0;\n\
        while ($i < 3) { $i = calc($i + 1); }\n\
        if ($minimum < 3000000000L | $input = 2) { mes(r\"<col=ff0000>£€</col>\"); }\n\
        switch_int ($input) {\n\
            case 2: return($items(2), r\"<text>\", 9223372036854775807L);\n\
            case default: return($i, r\"fallback\", -3000000000L);\n\
        }";
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100000, Script::decode(&original, &format).unwrap())]);
    let signatures = source::signatures(&scripts);
    let generated = source::decompile(
        &scripts[&100000],
        100000,
        &names,
        &signatures,
        &HashMap::new(),
        &commands,
        &registry,
    )
    .unwrap();
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
    let edited = Source::parse(&text.replace("= 7;", "= 19;"), path).unwrap();
    let changed = edited.compile(&registry, &format).unwrap().remove(0).1;
    assert_ne!(changed, original);
    assert!(
        Script::decode(&changed, &format)
            .unwrap()
            .int_operands
            .contains(&19)
    );
    for end in 0..original.len() {
        assert!(
            Script::decode(&original[..end], &format).is_err(),
            "Accepted truncation at {end}"
        );
    }
    let mut corrupt = original.clone();
    let len = corrupt.len();
    corrupt[len - 2..].copy_from_slice(&[255, 255]);
    assert!(Script::decode(&corrupt, &format).is_err());
}

#[test]
fn authored_source_reports_type_and_symbol_errors() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    for body in ["mes(1);", "unknown_command();", "return(r\"wrong\");"] {
        let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
        let source = Source::parse(
            &format!("[proc,clientscript_100001]() {body}"),
            Path::new("invalid.cs2"),
        )
        .unwrap();
        clientscript::register(&source.file, &HashMap::new(), &mut registry).unwrap();
        assert!(
            source.compile(&registry, &format).is_err(),
            "Accepted {body}"
        );
    }
}

#[test]
fn readable_declarations_strings_conditions_and_implicit_returns_preserve_code() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("readable.cs2");
    let text = r#"[proc,clientscript_100002](boolean $enabled)
        def_string $message = "null";
        if ($enabled = true) { $message = "Accept aid<br>(currently off)"; }
        mes($message);
        mes("\<unformatted>");"#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100002, Script::decode(&original, &format).unwrap())]);
    let generated = source::decompile(
        &scripts[&100002],
        100002,
        &names,
        &source::signatures(&scripts),
        &HashMap::new(),
        &commands,
        &registry,
    )
    .unwrap();
    assert!(generated.contains("def_string $string = \"null\";"));
    assert!(generated.contains("\"Accept aid<br>(currently off)\""));
    assert!(generated.contains("if ($int = true)"));
    assert!(!generated.contains("@cs2"));
    assert!(!generated.trim_end().ends_with("return;"));
    assert!(!generated.contains("__join"));
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
}

#[test]
fn command_arguments_use_typed_names_without_changing_bytes() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    registry.register_entity_id("options_boxes,1".to_owned(), Type::Obj, 99999);
    let path = Path::new("names.cs2");
    let text = r#"[proc,clientscript_100003]()
        if_setgraphic(762, 17104903);
        if_setonmouserepeat(38, -2147483645, 17104916, "Accept aid", 25, 180, "IIsii", 17104903);
        if_settext("options_boxes,1", 17104903);
        if_sethide(0, 11862017);
        cc_setonmouseover(2140, -2147483645, -2147483643, 200, "Iii");"#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100003, Script::decode(&original, &format).unwrap())]);
    let mut signatures = source::signatures(&scripts);
    signatures.insert(38, source::Signature::default());
    signatures.insert(2140, source::Signature::default());
    let generated = source::decompile(
        &scripts[&100003],
        100003,
        &names,
        &signatures,
        &HashMap::new(),
        &commands,
        &registry,
    )
    .unwrap();
    assert!(
        generated.contains("if_setgraphic(\"options_boxes,1\", options:com7);"),
        "{generated}"
    );
    assert!(generated.contains("if_setonmouserepeat(addtooltip, ^event_com, options:com20, \"Accept aid\", 25, 180, \"IIsii\", options:com7);"), "{generated}");
    assert!(generated.contains("if_settext(\"options_boxes,1\", options:com7);"));
    assert!(generated.contains("if_sethide(false, \"well of voyage_shift_click:com1\");"));
    assert!(
        generated
            .contains("cc_setonmouseover(cc_settrans, ^event_com, ^event_subid, 200, \"Iii\");")
    );
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
}

#[test]
fn structured_flow_and_typed_procedures_need_no_stack_helpers() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("structured.cs2");
    let text = r#"
        [proc,clientscript_100004](int $component, int $graphic, int $colour)
        if_setgraphic($graphic, $component);
        if_setcolour($colour, $component);

        [proc,clientscript_100005](int $state)
        def_int $a = 0;
        def_int $b = 0;
        $a, $b = 11, 22;
        def_component $components();
        def_component $components(2);
        $components(0) = options:com7;
        def_component $components(3);
        if ($state = false) {
            ~clientscript_100004(17104903, 762, 14483456);
        } else if ($state = true) {
            ~clientscript_100004($components(0), 763, 0xFFFFFF);
        }
        if ($state = 2 | $a > 0 & $b > 0) {
            while ($a < 20) { $a = calc($a + 1); }
        }
        switch_int ($state) { case 4: mes("four"); }
        ~clientscript_100006();

        [proc,clientscript_100006]()(int,string)
        return(1,"value");
    "#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original: HashMap<_, _> = parsed
        .compile(&registry, &format)
        .unwrap()
        .into_iter()
        .map(|(id, bytes)| (id as u32, bytes))
        .collect();
    let scripts = original
        .iter()
        .map(|(&id, bytes)| (id, Script::decode(bytes, &format).unwrap()))
        .collect();
    let signatures =
        source::infer_signatures(&scripts, &names, &HashMap::new(), &commands, &mut registry)
            .unwrap();
    assert_eq!(
        signatures[&100004].param_types,
        [Type::Component, Type::Graphic, Type::Colour]
    );
    for (&id, script) in &scripts {
        let generated = source::decompile(
            script,
            id,
            &names,
            &signatures,
            &HashMap::new(),
            &commands,
            &registry,
        )
        .unwrap();
        assert!(!generated.contains("__"), "{generated}");
        if id == 100005 {
            assert!(generated.contains("else if ($int0 = true)"), "{generated}");
            assert!(
                generated.contains("options:com7, \"options_boxes,1\", 0xdd0000"),
                "{generated}"
            );
            assert!(generated.contains("~clientscript_100006;"));
            assert!(generated.contains("while ("));
        }
        let parsed = Source::parse(&generated, path).unwrap();
        clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
        assert_eq!(
            parsed.compile(&registry, &format).unwrap()[0].1,
            original[&id]
        );
    }
}

#[test]
fn dump_formatting_preserves_local_slots_grouping_constants_and_bare_calls() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("formatting.cs2");
    let text = r#"[proc,clientscript_100008](int $parent)
        def_int $child = $parent;
        if_setsize(calc(10 - (3 - 1)), calc((2 + 3) * 4), 0, 1, $child);
        if_setposition(0, 0, 1, 2, $child);
        cc_create($child, 3, 0);
        cc_settextalign(0, 1, 0);
        cc_setcolour(0);
        cc_setcolour(14483456);
        def_int $x;
        def_int $y;
        $x, $y = op6205();
        op6205();
        if_setsize(op6205(), 0, 0, $child);
        cc_setoninvtransmit(38, $parent, $child, "text", 25, 180, 93, 1, "IIsiiY");"#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100008, Script::decode(&original, &format).unwrap())]);
    let mut signatures =
        source::infer_signatures(&scripts, &names, &HashMap::new(), &commands, &mut registry)
            .unwrap();
    signatures.insert(38, source::Signature::default());
    let generated = source::decompile(
        &scripts[&100008],
        100008,
        &names,
        &signatures,
        &HashMap::new(),
        &commands,
        &registry,
    )
    .unwrap();
    assert!(generated.contains("component $component0"), "{generated}");
    assert!(
        generated.contains("def_component $component1 = $component0;"),
        "{generated}"
    );
    assert!(generated.contains("calc(10 - (3 - 1))"), "{generated}");
    assert!(generated.contains("calc((2 + 3) * 4)"), "{generated}");
    assert!(generated.contains("^iftype_rectangle"));
    assert!(generated.contains("^setpos_abs_centre, ^setpos_abs_bottom"));
    assert!(generated.contains("cc_setcolour(^black);"));
    assert!(generated.contains("cc_setcolour(0xdd0000);"));
    assert!(
        generated.contains("if_setsize(op6205, ^setsize_abs, ^setsize_abs, $component1);"),
        "{generated}"
    );
    assert!(generated.contains("93, 1, \"IIsiiY\""), "{generated}");
    assert!(!generated.contains("op6205()"));
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
}

#[test]
fn dynamic_return_types_use_normal_command_names() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    registry.register_param_type(99999, Type::String);
    let path = Path::new("dynamic.cs2");
    let text = r#"[proc,clientscript_100007]()
        mes(enum('i', 's', 0, 1));
        mes(struct_param(0, 99999));
        enum('i', 's', 0, 2);
        struct_param(0, 99999);"#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100007, Script::decode(&original, &format).unwrap())]);
    let generated = source::decompile(
        &scripts[&100007],
        100007,
        &names,
        &source::signatures(&scripts),
        &HashMap::from([(99999, true)]),
        &commands,
        &registry,
    )
    .unwrap();
    assert!(!generated.contains("__"), "{generated}");
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
}

#[test]
fn integer_and_boolean_guards_preserve_bytes_and_behavior_without_metadata() {
    fn run(bytes: &[u8], input: i32, format: &Format) -> String {
        let script = Script::decode(bytes, format).unwrap();
        let mut ints = Vec::new();
        let mut strings = Vec::new();
        let mut pc = 0usize;
        loop {
            let operand = script.int_operands[pc];
            let jump = match script.opcodes[pc] {
                0 => {
                    ints.push(operand);
                    false
                }
                3 => {
                    strings.push(script.string_operands[pc].clone().unwrap());
                    false
                }
                33 => {
                    assert_eq!(operand, 0);
                    ints.push(input);
                    false
                }
                35 => {
                    strings.push(if operand == 0 { "single" } else { "plural" }.to_owned());
                    false
                }
                6 => true,
                op @ (7 | 8) => {
                    let rhs = ints.pop().unwrap();
                    let lhs = ints.pop().unwrap();
                    (lhs == rhs) == (op == 8)
                }
                86 => ints.pop().unwrap() == 1,
                87 => ints.pop().unwrap() == 0,
                21 => return strings.pop().unwrap(),
                opcode => panic!("Unexpected opcode {opcode}"),
            };
            pc = (pc as i64 + 1 + if jump { operand as i64 } else { 0 }) as usize;
        }
    }
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("plural.cs2");
    for literal in ["true", "1"] {
        let text = format!(
            "[proc,clientscript_100009](int $count, string $single, string $plural)(string)\nif ($count = {literal}) {{ return($single); }}\nreturn($plural);"
        );
        let parsed = Source::parse(&text, path).unwrap();
        clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
        let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
        let scripts = HashMap::from([(100009, Script::decode(&original, &format).unwrap())]);
        let generated = source::decompile(
            &scripts[&100009],
            100009,
            &names,
            &source::signatures(&scripts),
            &HashMap::new(),
            &commands,
            &registry,
        )
        .unwrap();
        assert!(generated.contains(&format!("(int $int, string $string0, string $string1)(string)\nif ($int = {literal}) {{\n    return($string0);\n}}\nreturn($string1);")), "{generated}");
        assert!(!generated.contains("@cs2"));
        assert!(!generated.contains("return(\"\");"));
        let parsed = Source::parse(&generated, path).unwrap();
        clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
        let roundtrip = parsed.compile(&registry, &format).unwrap().remove(0).1;
        assert_eq!(roundtrip, original);
        let edited = Source::parse(&generated.replace(&format!("= {literal}"), "= 2"), path)
            .unwrap()
            .compile(&registry, &format)
            .unwrap()
            .remove(0)
            .1;
        let inequality = Source::parse(
            &text.replace("= true", "! true").replace("= 1", "! 1"),
            path,
        )
        .unwrap();
        clientscript::register(&inequality.file, &HashMap::new(), &mut registry).unwrap();
        let inequality = inequality.compile(&registry, &format).unwrap().remove(0).1;
        for input in [-1, 0, 1, 2] {
            assert_eq!(
                run(&roundtrip, input, &format),
                if input == 1 { "single" } else { "plural" }
            );
            assert_eq!(
                run(&edited, input, &format),
                if input == 2 { "single" } else { "plural" }
            );
            assert_eq!(
                run(&inequality, input, &format),
                if input != 1 { "single" } else { "plural" }
            );
        }
    }
}

#[test]
fn typed_values_use_names_in_declarations_assignments_and_comparisons() {
    let pack = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/pack");
    let source_dir = pack.with_file_name("scripts").join("clientscript");
    let names = AssetNames::load(&pack).unwrap();
    let commands = clientscript::load_commands(
        &pack.join("cs2_command.pack"),
        &source_dir.join("engine.cs2"),
    )
    .unwrap();
    let format = Format::load(&pack.join("cs2_command.pack")).unwrap();
    let mut registry = clientscript::registry(&source_dir, &pack).unwrap();
    let path = Path::new("typed.cs2");
    let text = r#"[proc,clientscript_100010](int $flag)(string)
        def_int $component = 39059508;
        if_sethide($flag, $component);
        if ($flag = 1) { $component = 63897654; }
        if ($component = 39059508 | $component = 63897654) { if_setmodel(41312, $component); }
        def_int $item = 10010;
        if ($item = 10010) { $item = 11259; }
        mes(oc_name($item));
        switch_obj ($item) { case 10010: mes("net"); case -1: mes("empty"); }
        def_component $components(1);
        $components(0) = 39059508;
        if ($flag = true) { if_sethide(false, $components(0)); }
        def_int $position = 60298074;
        cam_lookat($position, 100, 1, 2);
        $position = 0;
        if ($position = 60298074) { $position = 1073741823; }
        cam_moveto(coordpack(60298074, 1, 0, 0), 100, 1, 2);
        cam_addwaypoint(1, 2, 60298074, 400, 60298073, 400, 0);
        cam_followcoord(-1);
        coordx(1073741824);
        coordz(-1073741824);
        if ($flag = true) { return("enabled"); }
        return("disabled");
    "#;
    let parsed = Source::parse(text, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    let original = parsed.compile(&registry, &format).unwrap().remove(0).1;
    let scripts = HashMap::from([(100010, Script::decode(&original, &format).unwrap())]);
    let signatures =
        source::infer_signatures(&scripts, &names, &HashMap::new(), &commands, &mut registry)
            .unwrap();
    let generated = source::decompile(
        &scripts[&100010],
        100010,
        &names,
        &signatures,
        &HashMap::new(),
        &commands,
        &registry,
    )
    .unwrap();
    for expected in [
        "def_component $component1 = login:com52;",
        "$component1 = south_progress9:com54;",
        "if ($component1 = login:com52 | $component1 = south_progress9:com54)",
        "if_setmodel(model_41312, $component1);",
        "def_obj $obj2 = butterfly_net;",
        "if ($obj2 = butterfly_net)",
        "$obj2 = magic_butterfly_net;",
        "case butterfly_net:",
        "case null:",
        "$array0(0) = login:com52;",
        "def_coord $coord3 = 0_57_77_32_26;",
        "$coord3 = 0_0_0_0_0;",
        "if ($coord3 = 0_57_77_32_26)",
        "$coord3 = 3_255_255_63_63;",
        "cam_moveto(coordpack(0_57_77_32_26, 1, 0, 0), 100, 1, 2);",
        "cam_addwaypoint(1, 2, 0_57_77_32_26, 400, 0_57_77_32_25, 400, 0);",
        "cam_followcoord(-1);",
        "coordx(1073741824);",
        "coordz(-1073741824);",
    ] {
        assert!(
            generated.contains(expected),
            "Missing {expected}:\n{generated}"
        );
    }
    assert_eq!(
        generated.matches("if ($boolean0 = true)").count(),
        2,
        "{generated}"
    );
    let parsed = Source::parse(&generated, path).unwrap();
    clientscript::register(&parsed.file, &HashMap::new(), &mut registry).unwrap();
    assert_eq!(parsed.compile(&registry, &format).unwrap()[0].1, original);
}
