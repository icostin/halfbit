use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;
use super::NonNull;
use super::HbAllocError;
use super::HbAllocator;

pub struct NopAllocator { }

pub const NOP_ALLOCATOR: NopAllocator = NopAllocator { };

unsafe impl HbAllocator for NopAllocator {
    unsafe fn alloc(
        &self,
        _size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, HbAllocError> {
        Err(HbAllocError::UnsupportedOperation)
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
    ) -> Result<NonNull<u8>, HbAllocError> {
        Err(HbAllocError::UnsupportedOperation)
    }
    unsafe fn shrink(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, HbAllocError> {
        Err(HbAllocError::UnsupportedOperation)
    }
    fn supports_contains(&self) -> bool { false }
    fn contains(&self, _ptr: NonNull<u8>) -> bool {
        panic!("contains not supported!");
    }
    fn name(&self) -> &'static str { "nop-allocator" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_name() {
        assert!(NOP_ALLOCATOR.name().contains("nop"));
    }

    #[test]
    fn alloc_not_supported() {
        assert_eq!(unsafe { NOP_ALLOCATOR.alloc(NonZeroUsize::new(1).unwrap(), Pow2Usize::one()) }.unwrap_err(), HbAllocError::UnsupportedOperation);
    }

    #[test]
    fn free_silently_completes() {
        unsafe { NOP_ALLOCATOR.free(NonNull::dangling(), NonZeroUsize::new(1).unwrap(), Pow2Usize::one()) };
    }

    #[test]
    fn grow_not_supported() {
        assert_eq!(unsafe { NOP_ALLOCATOR.grow(NonNull::dangling(), NonZeroUsize::new(1).unwrap(), NonZeroUsize::new(2).unwrap(), Pow2Usize::one()) }.unwrap_err(), HbAllocError::UnsupportedOperation);
    }

    #[test]
    fn shrink_not_supported() {
        assert_eq!(unsafe { NOP_ALLOCATOR.shrink(NonNull::dangling(), NonZeroUsize::new(2).unwrap(), NonZeroUsize::new(1).unwrap(), Pow2Usize::one()) }.unwrap_err(), HbAllocError::UnsupportedOperation);
    }

    #[test]
    fn contains_not_supported() {
        assert!(!NOP_ALLOCATOR.supports_contains());
    }

    #[test]
    #[should_panic]
    fn calling_contains_panics() {
        NOP_ALLOCATOR.contains(NonNull::dangling());
    }
}
