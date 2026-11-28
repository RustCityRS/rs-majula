use rs_io::Packet;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

/// A two-byte id, where 65535 is null.
pub(super) fn read_id(buf: &mut Packet) -> TypeId {
    TypeId::from_g2(buf.g2())
}

/// A model, seq or graphic id, which the client reads as a big smart from
/// 742 on.
pub(super) fn read_big_id(buf: &mut Packet) -> TypeId {
    #[cfg(before_742)]
    {
        read_id(buf)
    }
    #[cfg(since_742)]
    {
        TypeId::from_int(buf.gsmart2or4s())
    }
}

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
pub(super) fn read_transforms(
    buf: &mut Packet,
    default: TypeId,
    read: fn(&mut Packet) -> TypeId,
) -> Box<[TypeId]> {
    let count = buf.g1() as usize + 1;
    (0..count).map(|_| read(buf)).chain([default]).collect()
}

#[cfg(since_530)]
pub(super) fn read_optional_id(buf: &mut Packet) -> Option<TypeId> {
    read_id(buf).non_null()
}
