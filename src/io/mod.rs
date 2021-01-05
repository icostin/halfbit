use crate::mm::String;
use crate::error::Error;

#[derive(Copy, Clone, PartialEq, Debug)]
#[non_exhaustive]
pub enum ErrorCode {
    Unsuccessful, // some error that we cannot map to any of the below
    UnsupportedOperation,
    Interrupted,
    WouldBlock,
    BadOsHandle,
    UnexpectedEnd,
    UnsupportedPosition, // seek to a negative offset or to some large position past end that is not supported by the stream handler
    NoSpace,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::Unsuccessful => "unsuccessful",
            ErrorCode::UnsupportedOperation => "unsupported operation",
            ErrorCode::Interrupted => "interrupted",
            ErrorCode::WouldBlock => "would block",
            ErrorCode::BadOsHandle => "bad OS handle",
            ErrorCode::UnexpectedEnd => "unexpected end",
            ErrorCode::UnsupportedPosition => "unsupported position",
            ErrorCode::NoSpace => "no space",
        }
    }
}

impl core::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Display::fmt(self.as_str(), f)
    }
}


pub type IOError<'a> = Error<'a, ErrorCode>;
pub type IOResult<'a, T> = Result<T, IOError<'a>>;

impl<'a> IOError<'a> {
    pub fn get_error_code(&self) -> ErrorCode {
        *self.get_data()
    }
}

pub type IOPartialError<'a> = Error<'a, (ErrorCode, usize)>;
pub type IOPartialResult<'a, T> = Result<T, IOPartialError<'a>>;

impl<'a> IOPartialError<'a> {
    pub fn from_parts(
        code: ErrorCode,
        processed_size: usize,
        msg: String<'a>,
    ) -> Self {
        Error::new((code, processed_size), msg)
    }
    pub fn from_error_and_size(
        e: IOError<'a>,
        processed_size: usize,
    ) -> Self {
        let (code, msg) = e.to_parts();
        Error::new((code, processed_size), msg)
    }
    pub fn get_error_code(&self) -> ErrorCode {
        self.get_data().0
    }
    pub fn get_processed_size(&self) -> usize {
        self.get_data().1
    }
    pub fn to_error(self) -> IOError<'a> {
        let (data, msg) = self.to_parts();
        Error::new(data.0, msg)
    }
}

pub mod stream;
pub use stream::Null as NullStream;

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use std::string::String as StdString;
    use core::fmt::Write;

    fn error_code_fmt(e: ErrorCode, text_contained: &str) {
        let mut s = StdString::new();
        write!(s, "{}", e).unwrap();
        s.make_ascii_lowercase();
        assert!(s.contains(text_contained));
    }

    #[test]
    fn error_code_fmt_unsuccessful() {
        error_code_fmt(ErrorCode::Unsuccessful, "unsuccessful");
    }
    #[test]
    fn error_code_fmt_unsupported_operation() {
        error_code_fmt(ErrorCode::UnsupportedOperation, "unsupported operation");
    }
    #[test]
    fn error_code_fmt_interrupted() {
        error_code_fmt(ErrorCode::Interrupted, "interrupted");
    }
    #[test]
    fn error_code_fmt_would_block() {
        error_code_fmt(ErrorCode::WouldBlock, "would block");
    }
    #[test]
    fn error_code_fmt_bad_os_handle() {
        error_code_fmt(ErrorCode::BadOsHandle, "bad os handle");
    }
    #[test]
    fn error_code_fmt_unexpected_end() {
        error_code_fmt(ErrorCode::UnexpectedEnd, "unexpected end");
    }
    #[test]
    fn error_code_fmt_unsupported_position() {
        error_code_fmt(ErrorCode::UnsupportedPosition, "unsupported position");
    }
    #[test]
    fn error_code_fmt_no_space() {
        error_code_fmt(ErrorCode::NoSpace, "no space");
    }

    #[test]
    fn partial_error_from_parts() {
        let s = String::map_str("big boo-boo");
        let pe = IOPartialError::from_parts(ErrorCode::UnsupportedPosition, 123, s);
        assert_eq!(pe.get_error_code(), ErrorCode::UnsupportedPosition);
        assert_eq!(pe.get_processed_size(), 123);
        assert_eq!(pe.get_msg(), "big boo-boo");
    }
    #[test]
    fn partial_error_from_error() {
        let e = IOError::with_str(ErrorCode::NoSpace, "zilch");
        let pe = IOPartialError::from_error_and_size(e, 7);
        assert_eq!(pe.get_error_code(), ErrorCode::NoSpace);
        assert_eq!(pe.get_processed_size(), 7);
        assert_eq!(pe.get_msg(), "zilch");
    }

    #[test]
    fn partial_error_to_error() {
        let s = String::map_str("big boo-boo");
        let pe = IOPartialError::from_parts(ErrorCode::UnsupportedPosition, 123, s);
        let e = pe.to_error();
        assert_eq!(e.get_error_code(), ErrorCode::UnsupportedPosition);
        assert_eq!(e.get_msg(), "big boo-boo");
    }
}
