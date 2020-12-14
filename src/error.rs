use crate::mm::String;
use core::fmt::Debug;

#[derive(Debug)]
pub struct Error<'a, T>
where T: Sized + Debug {
    data: T,
    msg: String<'a>,
}

impl<'a, T> Error<'a, T> 
where T: Sized + Debug {
    pub fn new(data: T, msg: String<'a>) -> Error<'a, T> {
        Error { data, msg }
    }
    pub fn with_str(data: T, msg: &'a str) -> Error<'a, T> {
        Error::new(data, String::map_str(msg))
    }
    pub fn get_data(&self) -> &T { &self.data }
    pub fn get_msg(&self) -> &str { self.msg.as_str() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_and_get() {
        let e = Error::with_str(0x123_u32, "abc");
        assert_eq!(*e.get_data(), 0x123_u32);
        assert_eq!(e.get_msg(), "abc");
    }

}
