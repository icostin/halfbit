use super::{
    NonNull,
    Allocator,
    AllocatorRef,
    AllocError,
};

pub struct Box<'a, T> {
    allocator: AllocatorRef<'a>,
    payload: NonNull<T>,
}

impl<'a, T> Box<'a, T> {
    pub fn new(
        allocator: AllocatorRef<'a>,
        value: T,
    ) -> Result<Self, (AllocError, T)> {
        let size = core::mem::size_of::<T>();
        if size == 0 {
            return Ok(Box{ allocator: allocator, payload: NonNull::dangling() });
        }

        let align = core::mem::align_of::<T>();
        panic!("to do");
    }
}

impl<'a, T> Drop for Box<'a, T> {
    fn drop(&mut self) {
        let size = core::mem::size_of::<T>();
        if size == 0 { return; }
        panic!("to do");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm_v1::no_sup_allocator;

    #[test]
    fn zero_sized_boxed_payload_works_without_allocating() {
        let a = no_sup_allocator();
        let b = Box::new(a.to_ref(), ());
    }


}
