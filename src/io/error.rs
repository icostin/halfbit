#[non_exhaustive]
pub enum ErrorCode {
    Unspecified, // some error that we cannot map to any of the below
    Interrupted,
    WouldBlock,
    BadOsHandle,
    UnexpectedEnd,
}

