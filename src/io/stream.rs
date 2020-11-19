pub trait Stream {
    fn supports_seek(&self) -> bool {
        false
    }
}

