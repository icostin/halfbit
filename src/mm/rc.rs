use super::Allocator;
use super::AllocatorRef;
use super::AllocError;

pub struct RcBlock<'a> {
    strong: usize,
    weak: usize,
    allocator: AllocatorRef<'a>,
}

pub struct Rc<'a, T> {
    rc_ref: &'a mut (RcBlock<'a>, T)
}

impl<'a, T> Rc<'a, T> {
    pub fn new(
        allocator: AllocatorRef<'a>,
        value: T,
    ) -> Result<Self, (AllocError, T)> {
        panic!();
    }
}
