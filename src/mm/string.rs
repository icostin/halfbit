use super::Vector;
use super::AllocatorRef;
use core::fmt::Debug;
use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;
use core::fmt::Display as FmtDisplay;
use core::fmt::Formatter as FmtFormatter;

// UTF-8 string
pub struct String<'a> {
    data: Vector<'a, u8>,
}


impl<'a> String<'a> {
    pub fn new(allocator: AllocatorRef<'a>) -> String<'a> {
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
}

impl FmtWrite for String<'_> {
    fn write_str(&mut self, s: &str) -> FmtResult {
        self.data.append_from_slice(s.as_bytes())?;
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
}

