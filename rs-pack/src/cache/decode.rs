use rs_io::Packet;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

pub(super) fn read_array<T>(buf: &mut Packet, mut read: impl FnMut(&mut Packet) -> T) -> Box<[T]> {
    (0..buf.g1()).map(|_| read(buf)).collect()
}

#[cfg(since_647)]
pub(super) fn read_versioned_str(buf: &mut Packet) -> String {
    let version = buf.g1();
    assert_eq!(version, 0, "Unknown versioned string: {version}");
    buf.gjstr(STRING_TERMINATOR)
}

pub(super) fn read_pairs(
    buf: &mut Packet,
    first: &mut Option<Box<[u16]>>,
    second: &mut Option<Box<[u16]>>,
) {
    let (src, dst): (Vec<_>, Vec<_>) = (0..buf.g1()).map(|_| (buf.g2(), buf.g2())).unzip();
    *first = Some(src.into_boxed_slice());
    *second = Some(dst.into_boxed_slice());
}

#[cfg(since_289)]
pub(super) fn read_id(buf: &mut Packet) -> i32 {
    let id = buf.g2();
    if id == u16::MAX { -1 } else { id as i32 }
}

#[cfg(since_289)]
pub(super) fn read_transforms(buf: &mut Packet, default: Option<i32>) -> Box<[i32]> {
    let count = buf.g1() as usize + 1;
    let mut ids = Vec::with_capacity(count + usize::from(default.is_some()));
    ids.extend((0..count).map(|_| read_id(buf)));
    ids.extend(default);
    ids.into_boxed_slice()
}
