use crate::num::{
    NonZeroUsize,
    Pow2Usize,
};

use super::{
    NonNull,
    AllocError,
    Allocator,
};

pub struct NoSupAllocator { }

unsafe impl Allocator for NoSupAllocator {
    fn alloc(
        &self,
        _size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        Err(AllocError::UnsupportedOperation)
    }
    unsafe fn free(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _align: Pow2Usize) {
        panic!("cannot free what hasn't been allocated!");
    }
    unsafe fn grow(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_larger_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        panic!("cannot grow what hasn't been allocated!");
    }
    unsafe fn shrink(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        panic!("cannot shrink what hasn't been allocated!");
    }
    fn supports_contains(&self) -> bool {
        true
    }
    fn contains(
        &self,
        _ptr: NonNull<u8>
    ) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "no-sup-allocator"
    }
}

pub fn no_sup_allocator() -> NoSupAllocator {
    NoSupAllocator { }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_sup_allocator_has_specific_name() {
        let a = no_sup_allocator();
        assert!(a.name().contains("no-sup"));
    }

    #[test]
    fn no_sup_allocator_fails_to_alloc() {
        let a = no_sup_allocator();
        let r = a.alloc(NonZeroUsize::new(1).unwrap(),
            Pow2Usize::new(1).unwrap());
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), AllocError::UnsupportedOperation);
    }

    #[test]
    #[should_panic(expected = "what hasn't been allocated")]
    fn no_sup_allocator_panics_on_free() {
        let a = no_sup_allocator();
        unsafe {
            a.free(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::new(1).unwrap()
            );
        }
    }

    #[test]
    #[should_panic(expected = "what hasn't been allocated")]
    fn no_sup_allocator_panics_on_grow() {
        let a = no_sup_allocator();
        unsafe {
            a.grow(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        }.unwrap_or(NonNull::dangling());
    }

    #[test]
    #[should_panic(expected = "what hasn't been allocated")]
    fn no_sup_allocator_panics_on_shrink() {
        let a = no_sup_allocator();
        unsafe {
            a.shrink(
                NonNull::dangling(),
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        }.unwrap_or(NonNull::dangling());
    }

    #[test]
    fn no_sup_allocator_supports_contains() {
        let a = no_sup_allocator();
        assert!(a.supports_contains());
    }

    #[test]
    fn no_sup_allocator_returns_false_for_contains() {
        let a = no_sup_allocator();
        assert_eq!(a.contains(NonNull::dangling()), false);
    }
}

