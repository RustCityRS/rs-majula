use super::UnpackedPacks;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use std::collections::HashMap;

pub(super) fn decode_varbit(
    buf: &mut Packet,
    _: u16,
    _: &HashMap<u16, u16>,
    _packs: &mut UnpackedPacks,
) -> Vec<(String, String)> {
    let mut props = Vec::new();
    while buf.remaining() > 0 {
        let code = buf.g1();
        if code == 0 {
            break;
        }
        match code {
            1 => {
                let basevar = buf.g2();
                let start = buf.g1();
                let end = buf.g1();
                if cfg!(before_647) || basevar != 0 || start != 0 || end != 0 {
                    #[cfg(before_647)]
                    let name = format!("varp_{basevar}");
                    #[cfg(since_647)]
                    let name = _packs
                        .config_name("varp", basevar as u32)
                        .map(str::to_string)
                        .unwrap_or_else(|| format!("varp_{basevar}"));
                    props.push(("basevar".into(), name));
                    props.push(("startbit".into(), start.to_string()));
                    props.push(("endbit".into(), end.to_string()));
                }
            }
            #[cfg(before_647)]
            10 => props.push(("debugname".into(), buf.gjstr(STRING_TERMINATOR))),
            250 => {
                buf.gjstr(STRING_TERMINATOR);
            }
            _ => panic!("Unrecognized varbit config code: {code}"),
        }
    }
    props
}
