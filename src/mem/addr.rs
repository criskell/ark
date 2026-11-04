use core::fmt;
use core::ops::{Add, AddAssign, Sub};

macro_rules! addr_type {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(u64);

        impl $name {
            pub const fn new(addr: u64) -> Self {
                Self(addr)
            }

            pub const fn as_u64(self) -> u64 {
                self.0
            }

            pub fn is_aligned(self, align: u64) -> bool {
                self.0.is_multiple_of(align)
            }

            pub fn align_down(self, align: u64) -> Self {
                debug_assert!(align.is_power_of_two());

                Self(self.0 & !(align - 1))
            }

            pub fn align_up(self, align: u64) -> Self {
                Self(self.0.next_multiple_of(align))
            }
        }

        impl Add<u64> for $name {
            type Output = Self;

            fn add(self, rhs: u64) -> Self {
                Self(self.0 + rhs)
            }
        }

        impl AddAssign<u64> for $name {
            fn add_assign(&mut self, rhs: u64) {
                self.0 += rhs;
            }
        }

        impl Sub<u64> for $name {
            type Output = Self;

            fn sub(self, rhs: u64) -> Self {
                Self(self.0 - rhs)
            }
        }

        impl Sub for $name {
            type Output = u64;

            fn sub(self, rhs: Self) -> u64 {
                self.0 - rhs.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({:#x})"), self.0)
            }
        }

        impl fmt::LowerHex for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::LowerHex::fmt(&self.0, f)
            }
        }
    };
}

addr_type!(PhysicalAddr);
addr_type!(VirtualAddr);

impl VirtualAddr {
    pub fn as_ptr<T>(self) -> *const T {
        self.0 as *const T
    }

    pub fn as_mut_ptr<T>(self) -> *mut T {
        self.0 as *mut T
    }
}
