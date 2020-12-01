use crate::num::{
    NonZeroUsize,
    Pow2Usize,
};

#[derive(PartialEq, Debug)]
pub enum AllocError {
    InvalidAlignment, // alignment not a power of 2
    AlignedSizeTooBig, // aligned size overflows usize
    UnsupportedAlignment, // allocator cannot guarantee requested alignment
    UnsupportedSize, // allocator does not support requested size
    NotEnoughMemory, // the proverbial hits the fan
    OperationFailed, // failure performing the operation (OS mem mapping error)
    UnsupportedOperation, // alloc, resize, free not supported
    NotImplemented,
}

pub unsafe trait Allocator {
    fn alloc(
        &self,
        _size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<*mut u8, AllocError> {
        Err(AllocError::NotImplemented)
    }
    unsafe fn grow(
        &self,
        _ptr: *mut u8,
        _current_size: NonZeroUsize,
        _new_larger_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<*mut u8, AllocError> {
        Err(AllocError::NotImplemented)
    }
    unsafe fn shrink(
        &self,
        _ptr: *mut u8,
        _current_size: NonZeroUsize,
        _new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<*mut u8, AllocError> {
        Err(AllocError::NotImplemented)
    }
    unsafe fn free(
        &self,
        _ptr: *mut u8,
        _current_size: NonZeroUsize,
        _align: Pow2Usize) {
    }
    fn supports_contains(&self) -> bool {
        false
    }
    fn contains(
        &self,
        _ptr: *mut u8
    ) -> bool {
        false
    }
    fn name(&self) -> &'static str {
        "some-allocator"
    }
    fn to_ref(&self) -> AllocatorRef
    where Self: Sized {
        AllocatorRef { allocator: self as &dyn Allocator }
    }
}

pub struct AllocatorRef<'a> {
    allocator: &'a (dyn Allocator + 'a)
}

pub mod no_sup_alloc;
use no_sup_alloc::no_sup_allocator as no_sup_allocator;

