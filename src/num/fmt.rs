use core::num::NonZeroU8;
use core::num::NonZeroU32;
use core::str;
use core::convert::{ TryFrom, TryInto };
use crate::num::PrimitiveInt;
use crate::num::PrimitiveUInt;

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

    pub fn zero_radix_prefix(self) -> &'static str {
        match self.unwrap() {
            2 => "0r2_",
            3 => "0r3_",
            4 => "0r4_",
            5 => "0r5_",
            6 => "0r6_",
            7 => "0r7_",
            8 => "0r8_",
            9 => "0r9_",
            10 => "0r10_",
            11 => "0r11_",
            12 => "0r12_",
            13 => "0r13_",
            14 => "0r14_",
            15 => "0r15_",
            16 => "0r16_",
            17 => "0r17_",
            18 => "0r18_",
            19 => "0r19_",
            20 => "0r20_",
            21 => "0r21_",
            22 => "0r22_",
            23 => "0r23_",
            24 => "0r24_",
            25 => "0r25_",
            26 => "0r26_",
            27 => "0r27_",
            28 => "0r28_",
            29 => "0r29_",
            30 => "0r30_",
            31 => "0r31_",
            32 => "0r32_",
            33 => "0r33_",
            34 => "0r34_",
            35 => "0r35_",
            _ => panic!("bad radix"),
        }
    }

    pub fn default_explicit_prefix(self) -> &'static str {
        match self.unwrap() {
            2 => "0b",
            8 => "0o",
            10 => "0d",
            16 => "0x",
            _ => self.zero_radix_prefix()
        }
    }
    pub fn default_prefix(self) -> &'static str {
        match self.unwrap() {
            10 => "",
            _ => self.default_explicit_prefix()
        }
    }
}
impl From<Radix> for u8 {
    fn from(v: Radix) -> u8 { v.0.get() }
}
impl TryFrom<u8> for Radix {
    type Error = ();
    fn try_from (v: u8) -> Result<Self, Self::Error> {
        Radix::new(v).ok_or(())
    }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub enum RadixNotation {
    None,
    PrefixZeroRadix,
    DefaultExplicitPrefix,
    DefaultPrefix,
}
impl RadixNotation {
    pub fn prefix(self, radix: Radix) -> &'static str {
        match self {
            RadixNotation::None => "",
            RadixNotation::PrefixZeroRadix => radix.zero_radix_prefix(),
            RadixNotation::DefaultExplicitPrefix => radix.default_explicit_prefix(),
            RadixNotation::DefaultPrefix => radix.default_prefix(),
        }
    }
}
impl From<RadixNotation> for u8 {
    fn from(v: RadixNotation) -> u8 { v as u8 }
}
impl TryFrom<u8> for RadixNotation {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(RadixNotation::None),
            1 => Ok(RadixNotation::PrefixZeroRadix),
            2 => Ok(RadixNotation::DefaultExplicitPrefix),
            3 => Ok(RadixNotation::DefaultPrefix),
            _ => Err(())
        }
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
impl From<MinDigitCount> for u8 {
    fn from(v: MinDigitCount) -> u8 { v.0.get() }
}
impl TryFrom<u8> for MinDigitCount {
    type Error = ();
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        MinDigitCount::new(v).ok_or(())
    }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub enum PositiveSign {
    Hidden,
    Space,
    Plus,
}
impl PositiveSign {
    fn push_sign(self, buf: &mut ReverseFillBuffer<'_>) -> Result<(), ()> {
        match self {
            PositiveSign::Hidden => Ok(()),
            PositiveSign::Space => buf.push(b' '),
            PositiveSign::Plus => buf.push(b'+'),
        }
    }
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
impl ZeroSign {
    fn push_sign(self, buf: &mut ReverseFillBuffer<'_>) -> Result<(), ()> {
        match self {
            ZeroSign::Hidden => Ok(()),
            ZeroSign::Space => buf.push(b' '),
            ZeroSign::Plus => buf.push(b'+'),
            ZeroSign::Minus => buf.push(b'-'),
        }
    }
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

struct ReverseFillBuffer<'a> {
    buf: &'a mut [u8],
    pos: usize,
}
impl<'a> ReverseFillBuffer<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        let pos = buf.len();
        ReverseFillBuffer { buf, pos }
    }
    fn push(&mut self, v: u8) -> Result<(), ()> {
        if self.pos > 0 {
            self.pos -= 1;
            self.buf[self.pos] = v;
            Ok(())
        } else {
            Err(())
        }
    }
    fn push_str(&mut self, v: &str) -> Result<(), ()> {
        if self.pos >= v.len() {
            self.pos -= v.len();
            self.buf[self.pos..self.pos+v.len()].copy_from_slice(v.as_bytes());
            Ok(())
        } else {
            Err(())
        }
    }
    fn to_used_slice(self) -> &'a [u8] {
        &self.buf[self.pos..]
    }
}

#[derive(Clone, Copy,  Debug, PartialEq)]
pub struct MiniNumFmtPack {
    pack: NonZeroU32,
}
impl MiniNumFmtPack {

    const MIN_DIGIT_COUNT_BIT_POS: u8 = 0;
    const MIN_DIGIT_COUNT_BIT_COUNT: u8 = 8;

    const RADIX_BIT_POS: u8 = Self::MIN_DIGIT_COUNT_BIT_POS + Self::MIN_DIGIT_COUNT_BIT_COUNT;
    const RADIX_BIT_COUNT: u8 = 6;

    const RADIX_NOTATION_BIT_POS: u8 = Self::RADIX_BIT_POS + Self::RADIX_BIT_COUNT;
    const RADIX_NOTATION_BIT_COUNT: u8 = 2;

    const POSITIVE_SIGN_BIT_POS: u8 = Self::RADIX_NOTATION_BIT_POS + Self::RADIX_NOTATION_BIT_COUNT;
    const POSITIVE_SIGN_BIT_COUNT: u8 = 2;

    const ZERO_SIGN_BIT_POS: u8 = Self::POSITIVE_SIGN_BIT_POS + Self::POSITIVE_SIGN_BIT_COUNT;
    const ZERO_SIGN_BIT_COUNT: u8 = 2;

    pub fn new(
        radix: Radix,
        radix_notation: RadixNotation,
        min_digit_count: MinDigitCount,
        positive_sign: PositiveSign,
        zero_sign: ZeroSign,
    ) -> MiniNumFmtPack {
        MiniNumFmtPack {
            pack: NonZeroU32::new(
                ((u8::from(min_digit_count) as u32) << Self::MIN_DIGIT_COUNT_BIT_POS) |
                ((u8::from(radix) as u32) << Self::RADIX_BIT_POS) |
                ((u8::from(radix_notation) as u32) << Self::RADIX_NOTATION_BIT_POS) |
                ((u8::from(positive_sign) as u32) << Self::POSITIVE_SIGN_BIT_POS) |
                ((u8::from(zero_sign) as u32) << Self::ZERO_SIGN_BIT_POS)).unwrap()
        }
    }
    fn get_bits(self, pos: u8, count: u8) -> u32 {
        (self.pack.get() >> pos) & u32::lsb_mask(count.into())
    }
    fn get_bits_u8(self, pos: u8, count: u8) -> u8 {
        self.get_bits(pos, count).try_into().unwrap()
    }
    // fn set_bits(&mut self, pos: u8, count: u8, v: u32) {
    //     assert!((v & u32::lsb_mask(count.into())) == v);
    //     self.pack = NonZeroU32::new((self.pack.get() & u32::excl_bit_range_mask(pos.into(), count.into())) | (v << pos)).unwrap();
    // }
    pub fn default() -> MiniNumFmtPack {
        MiniNumFmtPack::new(
            Radix::new(10).unwrap(),
            RadixNotation::DefaultExplicitPrefix,
            MinDigitCount::new(1).unwrap(),
            PositiveSign::Hidden,
            ZeroSign::Hidden)
    }
    pub fn get_radix(self) -> Radix {
        Radix::new(self.get_bits_u8(Self::RADIX_BIT_POS, Self::RADIX_BIT_COUNT)).unwrap()
    }
    pub fn get_radix_notation(self) -> RadixNotation {
        self.get_bits_u8(Self::RADIX_NOTATION_BIT_POS, Self::RADIX_NOTATION_BIT_COUNT).try_into().unwrap()
    }
    pub fn get_min_digit_count(self) -> MinDigitCount {
        MinDigitCount::new(self.get_bits_u8(Self::MIN_DIGIT_COUNT_BIT_POS, Self::MIN_DIGIT_COUNT_BIT_COUNT)).unwrap()
    }
    pub fn get_positive_sign(self) -> PositiveSign {
        self.get_bits_u8(Self::POSITIVE_SIGN_BIT_POS, Self::POSITIVE_SIGN_BIT_COUNT).try_into().unwrap()
    }
    pub fn get_zero_sign(self) -> ZeroSign {
        self.get_bits_u8(Self::ZERO_SIGN_BIT_POS, Self::ZERO_SIGN_BIT_COUNT).try_into().unwrap()
    }

    pub fn int_fmt<'a, T: IntFmt>(
        self,
        n: T,
        buf: &'a mut [u8],
    ) -> Result<&'a str, ()> {
        let radix = self.get_radix();
        let radix_prefix = self.get_radix_notation().prefix(radix);
        let min_digit_count = self.get_min_digit_count();
        let positive_sign = self.get_positive_sign();
        let zero_sign = self.get_zero_sign();
        n.int_fmt_buf(
            radix,
            radix_prefix,
            min_digit_count,
            positive_sign,
            zero_sign,
            buf)
    }
}

trait UIntFmt {
    fn uint_fmt_buf<'a>(
        &self,
        radix: Radix,
        radix_prefix: &str,
        min_digit_count: MinDigitCount,
        buf: &'a mut [u8]
    ) -> Result<ReverseFillBuffer<'a>, ()>
    where Self: Sized;

}
impl<T: PrimitiveUInt> UIntFmt for T {
    fn uint_fmt_buf<'a>(
        &self,
        radix: Radix,
        radix_prefix: &str,
        min_digit_count: MinDigitCount,
        buf: &'a mut [u8]
    ) -> Result<ReverseFillBuffer<'a>, ()>
    where Self: Sized {
        let min_digit_count = min_digit_count.unwrap();
        let radix = Self::reinterpret_u8(radix.unwrap());
        let mut buf = ReverseFillBuffer::new(buf);
        let mut n = *self;
        let mut digit_count = 0_usize;
        while n != Self::ZERO || digit_count < min_digit_count {
            digit_count += 1;
            let digit = (n % radix).trunc_to_u8() as usize;
            buf.push(b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"[digit])?;
            n = n / radix;
        }
        buf.push_str(radix_prefix)?;
        Ok(buf)
    }

}

pub trait IntFmt {
    fn int_fmt_buf<'a>(
        &self,
        radix: Radix,
        radix_prefix: &str,
        min_digit_count: MinDigitCount,
        positive_sign: PositiveSign,
        zero_sign: ZeroSign,
        buf: &'a mut [u8],
    ) -> Result<&'a str, ()>;
}

impl<T: PrimitiveInt> IntFmt for T {
    fn int_fmt_buf<'a>(
        &self,
        radix: Radix,
        radix_prefix: &str,
        min_digit_count: MinDigitCount,
        positive_sign: PositiveSign,
        zero_sign: ZeroSign,
        buf: &'a mut [u8],
    ) -> Result<&'a str, ()> {
        let mut buf = self.abs_uint().uint_fmt_buf(radix, radix_prefix, min_digit_count, buf)?;

        if *self < Self::ZERO {
            buf.push(b'-')
        } else if *self == Self::ZERO {
            zero_sign.push_sign(&mut buf)
        } else {
            positive_sign.push_sign(&mut buf)
        }?;
        str::from_utf8(buf.to_used_slice()).map_err(|_| ())
    }
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

    #[test]
    fn radix_notation_conv() {
        use core::convert::TryInto;
        assert_eq!(0_u8, RadixNotation::None.into());
        assert_eq!(1_u8, RadixNotation::PrefixZeroRadix.into());
        assert_eq!(2_u8, RadixNotation::DefaultExplicitPrefix.into());
        assert_eq!(3_u8, RadixNotation::DefaultPrefix.into());
        assert_eq!(RadixNotation::None, 0_u8.try_into().unwrap());
        assert_eq!(RadixNotation::PrefixZeroRadix, 1_u8.try_into().unwrap());
        assert_eq!(RadixNotation::DefaultExplicitPrefix, 2_u8.try_into().unwrap());
        assert_eq!(RadixNotation::DefaultPrefix, 3_u8.try_into().unwrap());
        assert_eq!(RadixNotation::try_from(4_u8), Err(()));
    }

    #[test]
    fn mini_num_fmt_pack() {
        let nf = MiniNumFmtPack::new(
            Radix::new(16).unwrap(),
            RadixNotation::DefaultPrefix,
            MinDigitCount::new(6).unwrap(),
            PositiveSign::Plus,
            ZeroSign::Space);
        {
            let mut buf = [0_u8; 32];
            assert_eq!(nf.int_fmt(0x12345_u32, &mut buf).unwrap(), "+0x012345");
        }
        {
            let mut buf = [0_u8; 32];
            assert_eq!(nf.int_fmt(-0x12345_i32, &mut buf).unwrap(), "-0x012345");
        }
    }
}
