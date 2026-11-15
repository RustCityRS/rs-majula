use super::UnpackedPacks;
use super::param::char_to_type_name;
use rs_io::Packet;
use rs_ver::TypeId;

/// A clan setting, or from 742 a clan var, whose bit range comes on `bitcode`.
pub(super) fn decode_clan_var(
    buf: &mut Packet,
    packs: &UnpackedPacks,
    kind: &str,
    bitcode: u8,
) -> Vec<(String, String)> {
    let mut props = Vec::new();
    loop {
        match buf.g1() {
            0 => return props,
            1 => {
                let ty = match buf.g1() {
                    0x01 => "bit".to_owned(),
                    0xa7 => "clanforumqfc".to_owned(),
                    other => char_to_type_name(other),
                };
                props.push(("type".into(), ty));
            }
            code if code == bitcode => {
                props.push((
                    "basevar".into(),
                    packs.reference(kind, TypeId::from_g2(buf.g2())),
                ));
                props.push(("startbit".into(), buf.g1().to_string()));
                props.push(("endbit".into(), buf.g1().to_string()));
            }
            code => panic!("Unrecognized {kind} config code: {code}"),
        }
    }
}
