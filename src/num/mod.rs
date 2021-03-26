use core::ptr::NonNull;

pub trait PrimitiveInt:
    Copy +
    core::ops::Shl<u8, Output = Self> +
    core::ops::BitAnd<Output = Self> +
    core::ops::BitOr<Output = Self> +
    core::cmp::Eq +
    core::ops::Sub<Output = Self> +
    Sized {
    const SIZE: usize;
    const ZERO: Self;
    const ONE: Self;
    fn reinterpret_u8(v: u8) -> Self;
}

pub trait PrimitiveUInt: PrimitiveInt {}

impl PrimitiveInt for u8 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v }
}
impl PrimitiveUInt for u8 {}

impl PrimitiveInt for u16 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}
impl PrimitiveUInt for u16 {}

impl PrimitiveInt for u32 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}
impl PrimitiveUInt for u32 {}

impl PrimitiveInt for u64 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}
impl PrimitiveUInt for u64 {}

impl PrimitiveInt for usize {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}
impl PrimitiveUInt for usize {}

impl PrimitiveInt for i8 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}

impl PrimitiveInt for i16 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}

impl PrimitiveInt for i32 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}

impl PrimitiveInt for i64 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}

impl PrimitiveInt for isize {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
}


pub fn is_power_of_2<T: PrimitiveUInt> (n: T) -> bool {
    n != T::ZERO && (n & (n - T::ONE)) == T::ZERO
}

pub use core::num::NonZeroUsize;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Pow2Usize(NonZeroUsize);

impl Pow2Usize {

    pub fn new(n: usize) -> Option<Self> {
        if is_power_of_2(n) {
            Some(Pow2Usize(NonZeroUsize::new(n).unwrap()))
        } else {
            None
        }
    }

    pub fn get(self) -> usize {
        self.0.get()
    }

    pub fn one() -> Self {
        Pow2Usize::new(1).unwrap()
    }

    pub fn max() -> Self {
        Pow2Usize::new(!((!0) >> 1)).unwrap()
    }

    pub fn next(self) -> Option<Self> {
        Pow2Usize::new(self.get().wrapping_shl(1))
    }

    pub fn prev(self) -> Option<Self> {
        Pow2Usize::new(self.get().wrapping_shr(1))
    }

    pub fn shl(self, count: u32) -> Option<Self> {
        if count >= (core::mem::size_of::<usize>() as u32) * 8 {
            None
        } else {
            Pow2Usize::new(self.get().wrapping_shl(count))
        }
    }

    pub fn shr(self, count: u32) -> Option<Self> {
        if count >= (core::mem::size_of::<usize>() as u32) * 8 {
            None
        } else {
            Pow2Usize::new(self.get().wrapping_shr(count))
        }
    }

    pub fn from_smaller_or_equal_usize(n: usize) -> Option<Self> {
        let mut p = Self::one();
        while p.get() < n {
            match p.next() {
                Some(q) => p = q,
                None => return None
            }
        }
        Some(p)
    }

    pub fn rmask (&self) -> usize {
        self.0.get() - 1
    }

    pub fn lmask(&self) -> usize {
        !self.rmask()
    }

    pub fn is_aligned(&self, v: usize) -> bool {
        v & self.rmask() == 0
    }

    pub fn is_ptr_aligned<T>(&self, ptr: *const T) -> bool {
        self.is_aligned(ptr as usize)
    }

    pub fn is_non_null_ptr_aligned<T>(&self, nnptr: NonNull<T>) -> bool {
        self.is_ptr_aligned(nnptr.as_ptr())
    }
}

use core::num::Wrapping;
pub fn usize_align_up (n: usize, align: Pow2Usize) -> Option<usize> {
    let mask = Wrapping(align.get()) - Wrapping(1usize);
    let aligned = (Wrapping(n) + mask).0 & !mask.0;
    if aligned < n { None } else { Some(aligned) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_power_of_2_checks() {
        assert!(!is_power_of_2(0u32));
        assert!(is_power_of_2(1u8));
        assert!(is_power_of_2(2u16));
        assert!(!is_power_of_2(3u64));
        assert!(is_power_of_2(4usize));
    }

    #[test]
    fn pow2usize_max() {
        assert_eq!(Pow2Usize::max().get().swap_bytes(), 0x80usize);
    }

    #[test]
    fn pow2usize_max_next() {
        assert!(Pow2Usize::max().next().is_none());
    }

    #[test]
    fn pow2usize_max_prev() {
        assert_eq!(Pow2Usize::max().prev().unwrap().get().swap_bytes(),
            0x40usize);
    }

    #[test]
    fn pow2usize_1_next() {
        assert_eq!(Pow2Usize::one().next().unwrap().get(), 2usize);
    }

    #[test]
    fn pow2usize_1_prev() {
        assert!(Pow2Usize::one().prev().is_none());
    }

    #[test]
    fn pow2usize_1_shl_non_overflowing() {
        assert_eq!(Pow2Usize::one().shl(2).unwrap().get(), 4usize);
    }

    #[test]
    fn pow2usize_1_shr_underflow_value() {
        assert!(Pow2Usize::one().shr(2).is_none());
    }

    #[test]
    fn pow2usize_1_shl_overflow_counter() {
        assert!(Pow2Usize::one().shl(0x81).is_none());
    }

    #[test]
    fn pow2usize_1_shr_overflow_counter() {
        assert!(Pow2Usize::new(2).unwrap().shr(0x80).is_none());
    }

    #[test]
    fn from_smaller_or_equal_usize_0() {
        assert_eq!(Pow2Usize::from_smaller_or_equal_usize(0).unwrap().get(), 1);
    }

    #[test]
    fn from_smaller_or_equal_usize_1() {
        assert_eq!(Pow2Usize::from_smaller_or_equal_usize(1).unwrap().get(), 1);
    }

    #[test]
    fn from_smaller_equal_usize_3() {
        assert_eq!(Pow2Usize::from_smaller_or_equal_usize(3).unwrap().get(), 4);
    }

    #[test]
    fn from_smaller_or_equal_usize_max_pow2() {
        let m = Pow2Usize::max().get();
        assert_eq!(Pow2Usize::from_smaller_or_equal_usize(m).unwrap().get(), m);
    }

    #[test]
    fn from_smaller_or_equal_usize_over_max_pow2() {
        let m = Pow2Usize::max().get() + 1;
        assert!(Pow2Usize::from_smaller_or_equal_usize(m).is_none());
    }

    #[test]
    fn lmask_1() {
        assert_eq!(Pow2Usize::one().lmask(), usize::MAX);
    }

    #[test]
    fn rmask_1() {
        assert_eq!(Pow2Usize::one().rmask(), 0);
    }

    #[test] fn u8_reinterpret_u8() { assert_eq!(u8::reinterpret_u8(0xAB), 0xAB_u8); }
    #[test] fn i8_reinterpret_u8() { assert_eq!(i8::reinterpret_u8(0x80), -0x80_i8); }
    #[test] fn u16_reinterpret_u8() { assert_eq!(u16::reinterpret_u8(0xAB), 0xAB_u16); }
    #[test] fn i16_reinterpret_u8() { assert_eq!(i16::reinterpret_u8(0x80), 0x80_i16); }
    #[test] fn u32_reinterpret_u8() { assert_eq!(u32::reinterpret_u8(0xAB), 0xAB_u32); }
    #[test] fn i32_reinterpret_u8() { assert_eq!(i32::reinterpret_u8(0x80), 0x80_i32); }
    #[test] fn u64_reinterpret_u8() { assert_eq!(u64::reinterpret_u8(0xAB), 0xAB_u64); }
    #[test] fn i64_reinterpret_u8() { assert_eq!(i64::reinterpret_u8(0x80), 0x80_i64); }
    #[test] fn usize_reinterpret_u8() { assert_eq!(usize::reinterpret_u8(0xAB), 0xAB_usize); }
    #[test] fn isize_reinterpret_u8() { assert_eq!(isize::reinterpret_u8(0x80), 0x80_isize); }
}

