use super::Vector;
use super::Allocator;
use super::AllocatorRef;
use super::AllocError;
use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;

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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn simple_fmt_test() {
        let mut buffer = [0; 256];
        let mut a = BumpAllocator::new(&mut buffer);
        let mut s = String::new(a.to_ref());
        write!(s, "This is {:?}: {} = 0x{:04X}!", "so easy", 1234, 1234);
        assert_eq!(s.as_str(), "This is \"so easy\": 1234 = 0x04D2!");
    }
}

