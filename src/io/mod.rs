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
            ErrorCode::Unsuccessful => "unsuccesful",
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
}
