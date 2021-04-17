use super::Vector;
use super::HbAllocatorRef;
use super::HbAllocError;
use core::fmt::Debug;
use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;
use core::fmt::Display as FmtDisplay;
use core::fmt::Formatter as FmtFormatter;

// UTF-8 string
#[derive(PartialEq)]
pub struct String<'a> {
    data: Vector<'a, u8>,
}


impl<'a> String<'a> {
    pub fn new(allocator: HbAllocatorRef<'a>) -> String<'a> {
        String {
            data: Vector::new(allocator)
        }
    }
    pub fn map_str(s: &'a str) -> String<'a> {
        String {
            data: Vector::map_slice(s.as_bytes())
        }
    }
    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(self.data.as_slice()) }
    }
    pub fn push(&mut self, c: char) -> Result<(), HbAllocError> {
        let mut buf = [0_u8; 4];
        self.data.append_from_slice(c.encode_utf8(&mut buf).as_bytes())
    }
    pub fn append_str(
        &mut self,
        s: &str,
    ) -> Result<(), HbAllocError> {
        self.data.append_from_slice(s.as_bytes())?;
        Ok(())
    }
    pub fn dup<'b>(
        &self,
        allocator: HbAllocatorRef<'b>,
    ) -> Result<String<'b>, HbAllocError> {
        let mut o = String::new(allocator);
        o.append_str(self.as_str())?;
        Ok(o)
    }
}

impl FmtWrite for String<'_> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        self.append_str(s)?;
        Ok(())
    }
}

impl<'a> Debug for String<'a> {
    fn fmt(&self, fmt: &mut FmtFormatter<'_>) -> FmtResult {
        core::fmt::Debug::fmt(self.as_str(), fmt)
    }
}

impl<'a> FmtDisplay for String<'a> {
    fn fmt(&self, fmt: &mut FmtFormatter<'_>) -> FmtResult {
        FmtDisplay::fmt(self.as_str(), fmt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn simple_fmt_test() {
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        write!(s, "This is {:?}: {} = 0x{:04X}!", "so easy", 1234, 1234).unwrap();
        assert_eq!(s.as_str(), "This is \"so easy\": 1234 = 0x04D2!");
    }

    #[test]
    fn map_str() {
        let b = String::map_str("abc");
        assert_eq!(b.as_str(), "abc");
    }

    #[test]
    fn debug_fmt_uses_str() {
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let mut s = String::new(a.to_ref());

        let b = String::map_str("abc /\\ \"def\"");
        write!(s, "-{:?}-", b).unwrap();
        assert_eq!(s.as_str(), "-\"abc /\\\\ \\\"def\\\"\"-");
    }

    #[test]
    fn push_char() {
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        s.push('\u{101234}').unwrap();
        assert_eq!(s.as_str(), "\u{101234}");
    }


    #[test]
    fn dup() {
        let mut buffer = [0; 256];
        let a = BumpAllocator::new(&mut buffer);
        let b = String::map_str("abc /\\ \"def\"");

        use super::super::NOP_ALLOCATOR;
        assert_eq!(b.dup(NOP_ALLOCATOR.to_ref()).unwrap_err(), HbAllocError::UnsupportedOperation);

        let c = b.dup(a.to_ref()).unwrap();
        assert_eq!(c.as_str(), "abc /\\ \"def\"");
    }
}

