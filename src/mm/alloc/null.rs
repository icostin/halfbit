use crate::mm::layout::MemBlockLayout;
use super::AllocError;
use super::RawAllocator;

pub struct NullRawAllocator { }

unsafe impl RawAllocator for NullRawAllocator {
    fn alloc(
        &mut self,
        _layout: MemBlockLayout
    ) -> Result<*mut u8, AllocError> {
        Err(AllocError::NotEnoughMemory)
    }
    unsafe fn free(
        &mut self,
        ptr: *mut u8,
        layout: MemBlockLayout
    ) {
        panic!("aaaaaaaaaa");
    }
    fn name(&self) -> &'static str {
        "NullAllocator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_alloc_1_byte_fails() {
        let mut nra = NullRawAllocator{};
        assert_eq!(nra.alloc(MemBlockLayout::from_type::<u8>()).unwrap_err(),
            AllocError::NotEnoughMemory);
    }

}
