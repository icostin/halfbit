#[derive(PartialEq, Debug)]
#[non_exhaustive]
pub enum ErrorCode {
    Unsuccessful, // some error that we cannot map to any of the below
    UnsupportedOperation,
    Interrupted,
    WouldBlock,
    BadOsHandle,
    UnexpectedEnd,
}

pub type IOError<'a> = crate::error::Error<'a, ErrorCode>;
pub type IOResult<'a, T> = Result<T, IOError<'a>>;

pub mod stream;
pub use stream::Null as NullStream;

#[cfg(test)]
mod tests {
}
