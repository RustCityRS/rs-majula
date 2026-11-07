use super::client_config_table;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_number;
use anyhow::Result;
use std::collections::HashMap;

fn parts(value: &str) -> impl Iterator<Item = &str> {
    value.split(',').map(str::trim)
}

fn write_u8(client: &mut PackedData, code: u8, value: &str) {
    parse_number(value, |v: u8| {
        client.p1(code);
        client.p1(v);
    });
}

fn write_i8(client: &mut PackedData, code: u8, value: &str) {
    parse_number(value, |v: i8| {
        client.p1(code);
        client.p1(v as u8);
    });
}

fn write_u16(client: &mut PackedData, code: u8, value: &str) {
    parse_number(value, |v: u16| {
        client.p1(code);
        client.p2(v);
    });
}

fn write_u16s(client: &mut PackedData, code: u8, value: &str) {
    client.p1(code);
    for part in parts(value) {
        parse_number(part, |v: u16| client.p2(v));
    }
}

fn write_i32s(client: &mut PackedData, code: u8, value: &str) {
    client.p1(code);
    for part in parts(value) {
        parse_number(part, |v: i32| client.p4(v));
    }
}

fn write_list(client: &mut PackedData, code: u8, value: &str) {
    let ids: Vec<_> = parts(value).filter(|part| !part.is_empty()).collect();
    client.p1(code);
    client.p1(u8::try_from(ids.len()).expect("Too many effectors"));
    for id in ids {
        parse_number(id, |v: u16| client.p2(v));
    }
}

pub fn pack_particleemitters(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    client_config_table(
        file_cache,
        "particleemitter",
        &registry.particleemitter,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "coneangle" => write_u16s(client, 1, value),

                    // 2
                    "unknown2" => write_u8(client, 2, value),

                    // 3
                    "speed" => write_i32s(client, 3, value),

                    // 4
                    "speedcontrol" => {
                        let (kind, change) = value
                            .split_once(',')
                            .unwrap_or_else(|| panic!("Invalid speedcontrol: {value}"));
                        client.p1(4);
                        parse_number(kind.trim(), |v: u8| client.p1(v));
                        parse_number(change.trim(), |v: i8| client.p1(v as u8));
                    }

                    // 5 is one size, 31 a range
                    "size" if value.contains(',') => write_u16s(client, 31, value),
                    "size" => write_u16(client, 5, value),

                    // 6
                    "rgba" => write_i32s(client, 6, value),

                    // 7
                    "lifetime" => write_u16s(client, 7, value),

                    // 8
                    "emissionrate" => write_u16s(client, 8, value),

                    // 9
                    "worldeffectors" => write_list(client, 9, value),

                    // 10
                    "localeffectors" => write_list(client, 10, value),

                    // 12
                    "killbelowlevel" => write_i8(client, 12, value),

                    // 13
                    "killabovelevel" => write_i8(client, 13, value),

                    // 14
                    "unknown14" => write_u16(client, 14, value),

                    // 15
                    "material" => write_u16(client, 15, value),

                    // 16
                    "unknown16" => {
                        let values: Vec<_> = parts(value).collect();
                        let [start, offset, duration, repeat] = values[..] else {
                            panic!("Invalid unknown16: {value}");
                        };
                        client.p1(16);
                        parse_number(start, |v: u8| client.p1(v));
                        parse_number(offset, |v: u16| client.p2(v));
                        parse_number(duration, |v: u16| client.p2(v));
                        parse_number(repeat, |v: u8| client.p1(v));
                    }

                    // 17
                    "unknown17" => write_u16(client, 17, value),

                    // 18
                    "rgbatarget" => write_i32s(client, 18, value),

                    // 19
                    "unknown19" => write_u8(client, 19, value),

                    // 20
                    "rgbtransitionpercent" => write_u8(client, 20, value),

                    // 21
                    "alphatransitionpercent" => write_u8(client, 21, value),

                    // 22
                    "speedtarget" => write_i32s(client, 22, value),

                    // 23
                    "speedtargetpercent" => write_u8(client, 23, value),

                    // 24
                    "linkcolourchannels" => client.p1(24),

                    // 25
                    "unknown25" => write_list(client, 25, value),

                    // 26
                    "enabled" => client.p1(26),

                    // 27
                    "sizetarget" => write_u16(client, 27, value),

                    // 28
                    "sizetargetpercent" => write_u8(client, 28, value),

                    // 29
                    "unknown29" => parse_number(value, |v: i16| {
                        client.p1(29);
                        client.p2(v as u16);
                    }),

                    // 30
                    "unknown30" => client.p1(30),

                    // 32
                    "lighting" => client.p1(32),

                    // 33
                    "unknown33" => client.p1(33),

                    // 34
                    "unknown34" => client.p1(34),

                    // not found
                    _ => panic!("Unrecognized particleemitter config key: {key}"),
                }
            }
        },
    )
}
