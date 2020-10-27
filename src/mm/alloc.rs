
#[derive(PartialEq, Debug)]
pub enum AllocError {
    InvalidAlignment, // alignment not a power of 2
    SizeTooBig, // aligned size overflows usize
    UnsupportedAlignment, // allocator cannot guarantee requested alignment
    UnsupportedSize, // allocator does not support requested size
    NotEnoughMemory, // the proverbial hits the fan
    OperationFailed, // failure performing the operation (OS mem mapping error)
    UnsupportedOperation, // alloc, resize, free not supported
}

use super::layout::MemBlockLayout;

pub unsafe trait RawAllocator {
    fn alloc(
        &mut self,
        layout: MemBlockLayout
    ) -> Result<*mut u8, AllocError>;
    unsafe fn free(
        &mut self,
        ptr: *mut u8,
        layout: MemBlockLayout
    );
    fn name() -> &'static str;
}

