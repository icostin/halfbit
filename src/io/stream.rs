pub trait Stream {
    fn read(&mut self, buf: &[u8]) {
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

