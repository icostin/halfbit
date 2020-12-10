use crate::mm::String;

#[derive(PartialEq, Debug)]
#[non_exhaustive]
pub enum ErrorCode {
    Unsuccessful, // some error that we cannot map to any of the below
    Interrupted,
    WouldBlock,
    BadOsHandle,
    UnexpectedEnd,
}

pub struct Error<'a> {
    code: ErrorCode,
    msg: String<'a>
}

impl<'a> Error<'a> {
    pub fn new(code: ErrorCode, msg: String<'a>) -> Error<'a> {
        Error {
            code,
            msg,
        }
    }
}

#[cfg(test)]
mod tests {

}
