use core::ptr::NonNull;

pub mod fmt;

pub const BITS_PER_BYTE: usize = 8;

pub trait PrimitiveInt:
    Copy +
    core::ops::Shl<u8, Output = Self> +
    core::ops::Shl<usize, Output = Self> +
    core::ops::BitAnd<Output = Self> +
    core::ops::BitOr<Output = Self> +
    core::ops::Not<Output = Self> +
    core::cmp::PartialEq +
    core::cmp::PartialOrd +
    core::cmp::Eq +
    core::ops::Sub<Output = Self> +
    core::ops::Div<Output = Self> +
    core::ops::Rem<Output = Self> +
    Sized
where
    Self::SameSizeUInt: PrimitiveUInt,
    Self::SameSizeSInt: PrimitiveSInt,
{
    const SIZE: usize;
    const ZERO: Self;
    const ONE: Self;
    type SameSizeUInt;
    type SameSizeSInt;

    fn reinterpret_u8(v: u8) -> Self;
    fn trunc_to_u8(self) -> u8;
    fn lsb_mask_checked(n: usize) -> Option<Self> {
        let bit_count = Self::SIZE * BITS_PER_BYTE;
        if n < bit_count {
            Some((Self::ONE << n) - Self::ONE)
        } else if n == bit_count {
            Some(!Self::ZERO)
        } else {
            None
        }
    }
    fn lsb_mask(n: usize) -> Self {
        Self::lsb_mask_checked(n).unwrap()
    }
    fn msb_mask_checked(n: usize) -> Option<Self> {
        Self::lsb_mask_checked(n).map(|x| !x)
    }
    fn msb_mask(n: usize) -> Self {
        Self::msb_mask_checked(n).unwrap()
    }
    fn incl_bit_range_mask_checked(pos: usize, count: usize) -> Option<Self> {
        pos.checked_add(count)
            .and_then(|end| Self::lsb_mask_checked(end))
            .and_then(|em| Self::msb_mask_checked(pos).map(|pm| em & pm))
    }
    fn excl_bit_range_mask_checked(pos: usize, count: usize) -> Option<Self> {
        Self::incl_bit_range_mask_checked(pos, count).map(|x| !x)
    }
    fn incl_bit_range_mask(pos: usize, count: usize) -> Self {
        Self::incl_bit_range_mask_checked(pos, count).unwrap()
    }
    fn excl_bit_range_mask(pos: usize, count: usize) -> Self {
        !Self::incl_bit_range_mask(pos, count)
    }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt;
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt;
    fn neg_wrapping(self) -> Self;
    fn abs_uint(self) -> Self::SameSizeUInt {
        let p =
            if self >= Self::ZERO {
                self
            } else {
                self.neg_wrapping()
            };
        p.reinterpret_as_uint()
    }
}

pub trait PrimitiveUInt: PrimitiveInt { }
pub trait PrimitiveSInt: PrimitiveInt { }

impl PrimitiveInt for u8 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u8;
    type SameSizeSInt = i8;
    fn reinterpret_u8(v: u8) -> Self { v }
    fn trunc_to_u8(self) -> u8 { self }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveUInt for u8 {}

impl PrimitiveInt for u16 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u16;
    type SameSizeSInt = i16;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveUInt for u16 {}

impl PrimitiveInt for u32 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u32;
    type SameSizeSInt = i32;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveUInt for u32 {}

impl PrimitiveInt for u64 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u64;
    type SameSizeSInt = i64;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveUInt for u64 {}

impl PrimitiveInt for usize {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = usize;
    type SameSizeSInt = isize;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveUInt for usize {}

impl PrimitiveInt for i8 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u8;
    type SameSizeSInt = i8;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveSInt for i8 {}

impl PrimitiveInt for i16 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u16;
    type SameSizeSInt = i16;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveSInt for i16 {}

impl PrimitiveInt for i32 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u32;
    type SameSizeSInt = i32;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveSInt for i32 {}

impl PrimitiveInt for i64 {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = u64;
    type SameSizeSInt = i64;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveSInt for i64 {}

impl PrimitiveInt for isize {
    const SIZE: usize = core::mem::size_of::<Self>();
    const ZERO: Self = 0;
    const ONE: Self = 1;
    type SameSizeUInt = usize;
    type SameSizeSInt = isize;
    fn reinterpret_u8(v: u8) -> Self { v as Self }
    fn trunc_to_u8(self) -> u8 { self as u8 }
    fn reinterpret_as_uint(self) -> Self::SameSizeUInt {
        self as Self::SameSizeUInt
    }
    fn reinterpret_as_sint(self) -> Self::SameSizeSInt {
        self as Self::SameSizeSInt
    }
    fn neg_wrapping(self) -> Self { self.wrapping_neg() }
}
impl PrimitiveSInt for isize {}

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

    pub fn align_up(&self, value: usize) -> Option<usize> {
        usize_align_up(value, *self)
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

    #[test] fn u8_trunc_to_u8() { assert_eq!(u8::trunc_to_u8(0xAB), 0xAB_u8) }
    #[test] fn i8_trunc_to_u8() { assert_eq!(i8::trunc_to_u8(-0x55), 0xAB_u8) }
    #[test] fn u16_trunc_to_u8() { assert_eq!(u16::trunc_to_u8(0x88AB), 0xAB_u8) }
    #[test] fn i16_trunc_to_u8() { assert_eq!(i16::trunc_to_u8(-0x7755), 0xAB_u8) }
    #[test] fn u32_trunc_to_u8() { assert_eq!(u32::trunc_to_u8(0x888888AB), 0xAB_u8) }
    #[test] fn i32_trunc_to_u8() { assert_eq!(i32::trunc_to_u8(-0x77777755), 0xAB_u8) }
    #[test] fn u64_trunc_to_u8() { assert_eq!(u64::trunc_to_u8(0x88888888888888AB), 0xAB_u8) }
    #[test] fn i64_trunc_to_u8() { assert_eq!(i64::trunc_to_u8(-0x7777777777777755), 0xAB_u8) }
    #[test] fn usize_trunc_to_u8() { assert_eq!(usize::trunc_to_u8(0x888888AB), 0xAB_u8) }
    #[test] fn isize_trunc_to_u8() { assert_eq!(isize::trunc_to_u8(-0x77777755), 0xAB_u8) }
    #[test] fn u8_lsb0_mask() { assert_eq!(u8::lsb_mask(0), 0x00); }
    #[test] fn u8_lsb1_mask() { assert_eq!(u8::lsb_mask(1), 0x01); }
    #[test] fn u8_lsb7_mask() { assert_eq!(u8::lsb_mask(7), 0x7F); }
    #[test] fn u8_lsb8_mask() { assert_eq!(u8::lsb_mask(8), 0xFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u8_lsb9_mask() { u8::lsb_mask(9); }
    #[test] fn u16_lsb15_mask() { assert_eq!(u16::lsb_mask(15), 0x7FFF); }
    #[test] fn u16_lsb16_mask() { assert_eq!(u16::lsb_mask(16), 0xFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u16_lsb17_mask() { u16::lsb_mask(17); }
    #[test] fn u32_lsb31_mask() { assert_eq!(u32::lsb_mask(31), 0x7FFFFFFF); }
    #[test] fn u32_lsb32_mask() { assert_eq!(u32::lsb_mask(32), 0xFFFFFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u32_lsb33_mask() { u32::lsb_mask(33); }
    #[test] fn u64_lsb63_mask() { assert_eq!(u64::lsb_mask(63), 0x7FFFFFFFFFFFFFFF); }
    #[test] fn u64_lsb64_mask() { assert_eq!(u64::lsb_mask(64), 0xFFFFFFFFFFFFFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u64_lsb65_mask() { u64::lsb_mask(65); }
    #[test] fn usize_lsb_near_max_mask() { assert_eq!(usize::lsb_mask(usize::SIZE * BITS_PER_BYTE - 1), (!0_usize) >> 1); }
    #[test] fn usize_lsb_max_mask() { assert_eq!(usize::lsb_mask(usize::SIZE * BITS_PER_BYTE), !0_usize); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn usize_lsb_over_max_mask() { usize::lsb_mask(usize::SIZE * BITS_PER_BYTE + 1); }
    #[test] fn u8_msb0_mask() { assert_eq!(u8::msb_mask(0), !0x00); }
    #[test] fn u8_msb1_mask() { assert_eq!(u8::msb_mask(1), !0x01); }
    #[test] fn u8_msb7_mask() { assert_eq!(u8::msb_mask(7), !0x7F); }
    #[test] fn u8_msb8_mask() { assert_eq!(u8::msb_mask(8), !0xFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u8_msb9_mask() { u8::msb_mask(9); }
    #[test] fn u16_msb15_mask() { assert_eq!(u16::msb_mask(15), !0x7FFF); }
    #[test] fn u16_msb16_mask() { assert_eq!(u16::msb_mask(16), !0xFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u16_msb17_mask() { u16::msb_mask(17); }
    #[test] fn u32_msb31_mask() { assert_eq!(u32::msb_mask(31), !0x7FFFFFFF); }
    #[test] fn u32_msb32_mask() { assert_eq!(u32::msb_mask(32), !0xFFFFFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u32_msb33_mask() { u32::msb_mask(33); }
    #[test] fn u64_msb63_mask() { assert_eq!(u64::msb_mask(63), !0x7FFFFFFFFFFFFFFF); }
    #[test] fn u64_msb64_mask() { assert_eq!(u64::msb_mask(64), !0xFFFFFFFFFFFFFFFF); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn u64_msb65_mask() { u64::msb_mask(65); }
    #[test] fn usize_msb_near_max_mask() { assert_eq!(usize::msb_mask(usize::SIZE * BITS_PER_BYTE - 1), !((!0_usize) >> 1)); }
    #[test] fn usize_msb_max_mask() { assert_eq!(usize::msb_mask(usize::SIZE * BITS_PER_BYTE), 0_usize); }
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    #[test] fn usize_msb_over_max_mask() { usize::msb_mask(usize::SIZE * BITS_PER_BYTE + 1); }

}

