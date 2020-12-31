#[derive(PartialEq, Debug)]
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

pub type IOError<'a> = crate::error::Error<'a, ErrorCode>;
pub type IOResult<'a, T> = Result<T, IOError<'a>>;

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
}
