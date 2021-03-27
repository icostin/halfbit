use core::num::NonZeroU8;
use core::num::NonZeroU32;

#[derive(Clone, Copy,  Debug, PartialEq)]
pub struct Radix(NonZeroU8);
impl Radix {
    pub fn new(n: u8) -> Option<Self> {
        if n >= 2 && n <= 36 {
            Some(Radix(NonZeroU8::new(n).unwrap()))
        } else {
            None
        }
    }
    pub fn unwrap(self) -> u8 {
        self.0.get()
    }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub struct MinDigitCount(NonZeroU8);
impl MinDigitCount {
    pub fn new(n: u8) -> Option<Self> {
        let n = if n == 0 { 1 } else { n };
        if n <= 128 {
            Some(MinDigitCount(NonZeroU8::new(n).unwrap()))
        } else {
            None
        }
    }
    pub fn unwrap(self) -> usize {
        self.0.get().into()
    }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub struct MiniPack {
    pack: NonZeroU32,
}


#[derive(Clone, Copy,  Debug, PartialEq)]
pub enum PositiveSign {
    Hidden,
    Space,
    Plus,
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub enum ZeroSign {
    Hidden,
    Space,
    Plus,
    Minus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn radix() {
        assert_eq!(Radix::new(0), None);
        assert_eq!(Radix::new(1), None);
        assert_eq!(Radix::new(2), Some(Radix(NonZeroU8::new(2).unwrap())));
        assert_eq!(Radix::new(10).unwrap().unwrap(), 10);
        assert_eq!(Radix::new(36), Some(Radix(NonZeroU8::new(36).unwrap())));
        assert_eq!(Radix::new(37), None);
    }

    #[test]
    fn min_digit_count() {
        assert_eq!(MinDigitCount::new(1), Some(MinDigitCount(NonZeroU8::new(1).unwrap())));
        assert_eq!(MinDigitCount::new(2).unwrap().unwrap(), 2);
        assert_eq!(MinDigitCount::new(0).unwrap().unwrap(), 1);
        assert_eq!(MinDigitCount::new(255), None);
    }
}
