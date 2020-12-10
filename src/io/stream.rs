use super::error::Error;

pub trait Stream {
    fn read(&mut self, _buf: &[u8]) -> Result<usize, Error> {
        panic!("at the disco");
    }
    fn supports_read(&self) -> bool {
        false
    }
    fn supports_write_(&self) -> bool {
        false
    }
    fn supports_seek(&self) -> bool {
        false
    }
}

