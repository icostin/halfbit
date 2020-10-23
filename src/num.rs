extern crate num;

pub fn is_power_of_2<T> (n: T) -> bool
    where T: num::traits::Unsigned + num::traits::int::PrimInt {
    let zero: T = num::zero();
    let one: T = num::One::one();
    n != zero && (n & (n - one)) == zero
}

use core::num::NonZeroUsize;

#[derive(Debug)]
pub struct Pow2Usize(NonZeroUsize);

impl Pow2Usize {

    pub fn new (n: usize) -> Option<Self> {
        if is_power_of_2(n) {
            Some(Pow2Usize(NonZeroUsize::new(n).unwrap()))
        } else {
            None
        }
    }

    pub fn get (self) -> usize {
        self.0.get()
    }

    pub fn one () -> Self {
        Pow2Usize::new(1).unwrap()
    }

    pub fn max () -> Self {
        Pow2Usize::new(!((!0) >> 1)).unwrap()
    }

    pub fn next (self) -> Option<Self> {
        Pow2Usize::new(self.get().wrapping_shl(1))
    }

    pub fn prev (self) -> Option<Self> {
        Pow2Usize::new(self.get().wrapping_shr(1))
    }

    pub fn shl (self, count: u32) -> Option<Self> {
        if count >= (core::mem::size_of::<usize>() as u32) * 8 {
            None
        } else {
            Pow2Usize::new(self.get().wrapping_shl(count))
        }
    }

    pub fn shr (self, count: u32) -> Option<Self> {
        if count >= (core::mem::size_of::<usize>() as u32) * 8 {
            None
        } else {
            Pow2Usize::new(self.get().wrapping_shr(count))
        }
    }

}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_power_of_2_checks () {
        assert!(!is_power_of_2(0u32));
        assert!(is_power_of_2(1u8));
        assert!(is_power_of_2(2u16));
        assert!(!is_power_of_2(3u64));
        assert!(is_power_of_2(4usize));
    }

    #[test]
    fn pow2usize_max () {
        assert_eq!(Pow2Usize::max().get().swap_bytes(), 0x80usize);
    }

    #[test]
    fn pow2usize_max_next () {
        assert!(Pow2Usize::max().next().is_none());
    }

    #[test]
    fn pow2usize_max_prev () {
        assert_eq!(Pow2Usize::max().prev().unwrap().get().swap_bytes(),
            0x40usize);
    }

    #[test]
    fn pow2usize_1_next () {
        assert_eq!(Pow2Usize::one().next().unwrap().get(), 2usize);
    }

    #[test]
    fn pow2usize_1_prev () {
        assert!(Pow2Usize::one().prev().is_none());
    }

    #[test]
    fn pow2usize_1_shl_non_overflowing () {
        assert_eq!(Pow2Usize::one().shl(2).unwrap().get(), 4usize);
    }

    #[test]
    fn pow2usize_1_shr_underflow_value () {
        assert!(Pow2Usize::one().shr(2).is_none());
    }

    #[test]
    fn pow2usize_1_shl_overflow_counter () {
        assert!(Pow2Usize::one().shl(0x81).is_none());
    }

    #[test]
    fn pow2usize_1_shr_overflow_counter () {
        assert!(Pow2Usize::new(2).unwrap().shr(0x80).is_none());
    }


}
