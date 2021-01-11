extern crate num;
use core::mem::size_of;

pub fn uint_le_decode<T>(src: &[u8]) -> Option<T>
where T: num::traits::Unsigned + num::traits::int::PrimInt + core::ops::Shl + core::ops::BitOrAssign {
    if src.len() < size_of::<T>() {
        None
    } else {
        let mut v: T = num::zero();
        let mut sh = 0_usize;
        for b in src[..size_of::<T>()].iter() {
            v |= T::from(*b).unwrap() << sh;
            sh += 8;
        }
        Some(v)
    }
}

pub fn uint_be_decode<T>(src: &[u8]) -> Option<T>
where T: num::traits::Unsigned + num::traits::int::PrimInt + core::ops::Shl + core::ops::BitOr {
    if src.len() < size_of::<T>() {
        None
    } else {
        let mut v: T = num::zero();
        for b in src[..size_of::<T>()].iter() {
            v = (v << 8) | T::from(*b).unwrap();
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16le_on_truncated_buffer() {
        assert_eq!(uint_le_decode::<u16>(b"\x12"), None);
    }

    #[test]
    fn u16le_decode() {
        assert_eq!(uint_le_decode::<u16>(b"\x12\x34").unwrap(), 0x3412);
    }

    #[test]
    fn u16be_on_truncated_buffer() {
        assert_eq!(uint_be_decode::<u16>(b"\x12"), None);
    }


    #[test]
    fn u16be_decode() {
        assert_eq!(uint_be_decode::<u16>(b"\x12\x34").unwrap(), 0x1234);
    }

}
