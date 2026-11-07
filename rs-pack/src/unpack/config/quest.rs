use super::UnpackedPacks;
use crate::cache::quest::{QuestType, QuestVarRequirement};
use std::collections::HashMap;

fn var_requirement(requirement: &QuestVarRequirement, kind: &str, packs: &UnpackedPacks) -> String {
    let var = u16::try_from(requirement.var).map_or_else(
        |_| requirement.var.to_string(),
        |id| packs.reference(kind, id),
    );
    format!(
        "{var},{},{},{}",
        requirement.min, requirement.max, requirement.description
    )
}

pub fn emit_quest_props(
    def: &QuestType,
    packs: &mut UnpackedPacks,
    _reverse_hsl: &HashMap<u16, u16>,
) -> Vec<(String, String)> {
    let has = |code| def.opcode_order.contains(&code);
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if has(1) {
        p.push((1, "name".into(), def.name.clone()));
    }
    if has(2) {
        p.push((2, "sortname".into(), def.sortname.clone()));
    }
    for (var, start, end) in &def.progressvarp {
        let var = packs.reference("varp", *var);
        p.push((3, "masterquestvarp".into(), format!("{var},{start},{end}")));
    }
    for (var, start, end) in &def.progressvarbit {
        let var = packs.reference("varbit", *var);
        p.push((
            4,
            "masterquestvarbit".into(),
            format!("{var},{start},{end}"),
        ));
    }
    if has(5) {
        p.push((5, "parent".into(), packs.reference("quest", def.op5)));
    }
    if has(6) {
        let ty = match def.quest_type {
            0 => "normal".to_owned(),
            1 => "seasonal".to_owned(),
            other => other.to_string(),
        };
        p.push((6, "type".into(), ty));
    }
    if has(7) {
        let difficulty = match def.difficulty {
            0 => "novice".to_owned(),
            1 => "intermediate".to_owned(),
            2 => "experienced".to_owned(),
            3 => "master".to_owned(),
            4 => "grandmaster".to_owned(),
            other => other.to_string(),
        };
        p.push((7, "difficulty".into(), difficulty));
    }
    if def.members {
        p.push((8, "members".into(), "true".into()));
    }
    if has(9) {
        p.push((9, "questpoints".into(), def.points.to_string()));
    }
    if has(10) {
        let values: Vec<_> = def
            .op10
            .iter()
            .map(|&coord| packs.typed_value("coord", coord))
            .collect();
        p.push((10, "mainstartcoord".into(), values.join(",")));
    }
    if has(12) {
        p.push((12, "unknown12".into(), def.op12.to_string()));
    }
    for quest in &def.questrequirement {
        p.push((13, "questreq".into(), packs.reference("quest", *quest)));
    }
    for (stat, level) in &def.statrequirement {
        let stat = packs.reference("stat", *stat as u16);
        p.push((14, "statreq".into(), format!("{stat},{level}")));
    }
    if has(15) {
        p.push((15, "pointsreq".into(), def.pointsrequirement.to_string()));
    }
    if has(17) {
        p.push((
            17,
            "icon".into(),
            packs
                .reference("sprites", def.graphic)
                .trim_matches('"')
                .to_owned(),
        ));
    }
    for requirement in &def.varprequirement {
        let value = var_requirement(requirement, "varp", packs);
        p.push((18, "varpreq".into(), value));
    }
    for requirement in &def.varbitrequirement {
        let value = var_requirement(requirement, "varbit", packs);
        p.push((19, "varbitreq".into(), value));
    }
    p.extend(super::emit_param_list(&def.params, packs));
    super::reorder_by_opcode(&def.opcode_order, p)
}
