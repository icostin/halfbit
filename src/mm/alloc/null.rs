use crate::mm::layout::NonZeroMemBlockLayout;
use super::AllocError;
use super::RawAllocator;

pub struct NullRawAllocator { }

unsafe impl RawAllocator for NullRawAllocator {
    fn alloc(
        &mut self,
        _layout: NonZeroMemBlockLayout
    ) -> Result<*mut u8, AllocError> {
        Err(AllocError::NotEnoughMemory)
    }
    unsafe fn free(
        &mut self,
        _ptr: *mut u8,
        _layout: NonZeroMemBlockLayout
    ) {
        panic!("null allocator cannot free memory because it never allocates");
    }
    fn name(&self) -> &'static str {
        "NullAllocator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;

    #[test]
    fn relevant_name() {
        let mut nra = NullRawAllocator{};
        let name = nra.name();
        assert!(name.contains("null") || name.contains("Null"));
        assert!(name.contains("alloc") || name.contains("Alloc"));
    }

    #[test]
    fn raw_alloc_1_byte_fails() {
        let mut nra = NullRawAllocator{};
        assert_eq!(
            nra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap_err(),
            AllocError::NotEnoughMemory);
    }

    #[test]
    #[should_panic(expected = "null allocator cannot free")]
    fn raw_free_panics() {
        let mut nra = NullRawAllocator{};
        unsafe {
            nra.free(ptr::null_mut::<u8>(),
                     NonZeroMemBlockLayout::from_type::<u8>())
        };
    }


}
