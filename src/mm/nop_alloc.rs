use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;
use super::NonNull;
use super::AllocError;
use super::Allocator;

pub struct NopAllocator { }

pub const NOP_ALLOCATOR: NopAllocator = NopAllocator { };

unsafe impl Allocator for NopAllocator {
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
    }
    unsafe fn grow(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_larger_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        Err(AllocError::UnsupportedOperation)
    }
    unsafe fn shrink(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        Err(AllocError::UnsupportedOperation)
    }
    fn supports_contains(&self) -> bool {
        false
    }
    fn contains(
        &self,
        _ptr: NonNull<u8>
    ) -> bool {
        panic!("contains not supported!");
    }
    fn name(&self) -> &'static str {
        "nop-allocator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_name() {
        assert!(NOP_ALLOCATOR.name().contains("nop"));
    }
}
