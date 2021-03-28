use core::num::NonZeroU8;
use core::num::NonZeroU32;
use core::convert::TryFrom;

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
pub enum PositiveSign {
    Hidden,
    Space,
    Plus,
}
impl TryFrom<u8> for PositiveSign {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(PositiveSign::Hidden),
            1 => Ok(PositiveSign::Space),
            2 => Ok(PositiveSign::Plus),
            _ => Err(())
        }
    }
}
impl From<PositiveSign> for u8 {
    fn from(v: PositiveSign) -> u8 { v as u8 }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub enum ZeroSign {
    Hidden,
    Space,
    Plus,
    Minus,
}
impl TryFrom<u8> for ZeroSign {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(ZeroSign::Hidden),
            1 => Ok(ZeroSign::Space),
            2 => Ok(ZeroSign::Plus),
            3 => Ok(ZeroSign::Minus),
            _ => Err(())
        }
    }
}
impl From<ZeroSign> for u8 {
    fn from(v: ZeroSign) -> u8 { v as u8 }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub struct MiniPack {
    pack: NonZeroU32,
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

    #[test]
    fn positive_sign_conv() {
        use core::convert::TryInto;
        assert_eq!(0_u8, PositiveSign::Hidden.into());
        assert_eq!(1_u8, PositiveSign::Space.into());
        assert_eq!(2_u8, PositiveSign::Plus.into());
        assert_eq!(PositiveSign::Hidden, 0_u8.try_into().unwrap());
        assert_eq!(PositiveSign::Space, 1_u8.try_into().unwrap());
        assert_eq!(PositiveSign::Plus, 2_u8.try_into().unwrap());
        assert_eq!(PositiveSign::try_from(3_u8), Err(()));
    }

    #[test]
    fn zero_sign_conv() {
        use core::convert::TryInto;
        assert_eq!(0_u8, ZeroSign::Hidden.into());
        assert_eq!(1_u8, ZeroSign::Space.into());
        assert_eq!(2_u8, ZeroSign::Plus.into());
        assert_eq!(3_u8, ZeroSign::Minus.into());
        assert_eq!(ZeroSign::Hidden, 0_u8.try_into().unwrap());
        assert_eq!(ZeroSign::Space, 1_u8.try_into().unwrap());
        assert_eq!(ZeroSign::Plus, 2_u8.try_into().unwrap());
        assert_eq!(ZeroSign::Minus, 3_u8.try_into().unwrap());
        assert_eq!(ZeroSign::try_from(4_u8), Err(()));
    }

}
