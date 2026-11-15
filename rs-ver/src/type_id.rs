use std::fmt;

#[cfg(before_742)]
type Repr = u16;
#[cfg(since_742)]
type Repr = i32;

/// The id of a cache type: a config, model, seq, graphic or any other record.
/// A `u16` until 742, whose client reads more locs and models than that as ints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct TypeId(Repr);

impl TypeId {
    /// No type, as the client stores it: 65535, and -1 from 742 on.
    #[cfg(before_742)]
    pub const NULL: Self = Self(u16::MAX);
    #[cfg(since_742)]
    pub const NULL: Self = Self(-1);

    pub const fn new(id: u16) -> Self {
        Self(id as Repr)
    }

    /// A two-byte id, where 65535 is null.
    pub const fn from_g2(id: u16) -> Self {
        if id == u16::MAX {
            Self::NULL
        } else {
            Self::new(id)
        }
    }

    /// An int id, where -1 is null.
    #[cfg(before_742)]
    pub const fn from_int(id: i32) -> Self {
        Self(id as u16)
    }

    #[cfg(since_742)]
    pub const fn from_int(id: i32) -> Self {
        Self(id)
    }

    /// A script value: `None` when it is null or out of range.
    pub fn from_script(value: i32) -> Option<Self> {
        if value < 0 {
            return None;
        }
        Repr::try_from(value)
            .ok()
            .map(Self)
            .filter(|id| !id.is_null())
    }

    pub const fn is_null(self) -> bool {
        self.0 == Self::NULL.0
    }

    /// `None` for null.
    pub const fn non_null(self) -> Option<Self> {
        if self.is_null() { None } else { Some(self) }
    }

    /// The position in a type table; null is past every table.
    pub const fn index(self) -> usize {
        self.raw() as usize
    }

    /// The id as a client int: null is -1.
    #[cfg(before_742)]
    pub const fn int(self) -> i32 {
        if self.is_null() { -1 } else { self.0 as i32 }
    }

    #[cfg(since_742)]
    pub const fn int(self) -> i32 {
        self.0
    }

    /// The id in two bytes: null is 65535.
    #[cfg(before_742)]
    pub const fn g2(self) -> u16 {
        self.0
    }

    #[cfg(since_742)]
    pub const fn g2(self) -> u16 {
        self.0 as u16
    }

    /// The unsigned id: null is all ones.
    pub const fn raw(self) -> u32 {
        self.0 as u32
    }
}

impl From<u16> for TypeId {
    fn from(id: u16) -> Self {
        Self::new(id)
    }
}

impl TryFrom<u32> for TypeId {
    type Error = std::num::TryFromIntError;

    fn try_from(id: u32) -> Result<Self, Self::Error> {
        Repr::try_from(id).map(Self)
    }
}

impl TryFrom<i32> for TypeId {
    type Error = std::num::TryFromIntError;

    /// A non-negative id that fits.
    fn try_from(id: i32) -> Result<Self, Self::Error> {
        u32::try_from(id).and_then(Self::try_from)
    }
}

impl TryFrom<usize> for TypeId {
    type Error = std::num::TryFromIntError;

    fn try_from(id: usize) -> Result<Self, Self::Error> {
        Repr::try_from(id).map(Self)
    }
}

impl fmt::Display for TypeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.int().fmt(f)
    }
}
