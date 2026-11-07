use super::UnpackedPacks;
use super::param::char_to_type_name;
use rs_io::Packet;

pub(super) fn decode_varclansetting(
    buf: &mut Packet,
    packs: &UnpackedPacks,
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
            2 => {
                props.push((
                    "basevar".into(),
                    packs.reference("varclansetting", buf.g2()),
                ));
                props.push(("startbit".into(), buf.g1().to_string()));
                props.push(("endbit".into(), buf.g1().to_string()));
            }
            code => panic!("Unrecognized varclansetting config code: {code}"),
        }
    }
}
