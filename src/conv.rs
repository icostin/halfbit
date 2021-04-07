use crate::num::PrimitiveInt;
use crate::num::BITS_PER_BYTE;

pub fn int_le_decode<T: PrimitiveInt>(src: &[u8]) -> Option<T> {
    if src.len() < T::SIZE {
        None
    } else {
        let mut v = T::ZERO;
        let mut sh = 0_u8;
        for b in src[..T::SIZE].iter() {
            v = v | (T::reinterpret_u8(*b) << sh);
            sh += 8;
        }
        Some(v)
    }
}

pub fn int_be_decode<T: PrimitiveInt>(src: &[u8]) -> Option<T> {
    if src.len() < T::SIZE {
        None
    } else {
        let mut v = T::ZERO;
        for b in src[..T::SIZE].iter() {
            v = (v << BITS_PER_BYTE) | T::reinterpret_u8(*b);
        }
        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16le_on_truncated_buffer() {
        assert_eq!(int_le_decode::<u16>(b"\x12"), None);
    }

    #[test]
    fn u16le_decode() {
        assert_eq!(int_le_decode::<u16>(b"\x12\x34").unwrap(), 0x3412);
    }

    #[test]
    fn u16be_on_truncated_buffer() {
        assert_eq!(int_be_decode::<u16>(b"\x12"), None);
    }

    #[test]
    fn u16be_decode() {
        assert_eq!(int_be_decode::<u16>(b"\x12\x34").unwrap(), 0x1234);
    }

}
