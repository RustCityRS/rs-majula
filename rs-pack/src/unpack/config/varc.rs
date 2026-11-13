use super::param::char_to_type_name;
use rs_io::Packet;

pub(super) fn decode_varc(buf: &mut Packet) -> Vec<(String, String)> {
    let mut props = Vec::new();
    loop {
        match buf.g1() {
            0 => return props,
            1 => props.push(("type".into(), char_to_type_name(buf.g1()))),
            2 => props.push(("scope".into(), "perm".into())),
            code => panic!("Unrecognized varc config code: {code}"),
        }
    }
}

pub(super) fn decode_varc_string(buf: &mut Packet) -> Vec<(String, String)> {
    let mut props = vec![("type".to_owned(), "string".to_owned())];
    props.extend(decode_varc(buf));
    props
}
