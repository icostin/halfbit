
#[derive(PartialEq, Debug)]
pub enum AllocError {
    InvalidAlignment, // alignment not a power of 2
    AlignedSizeTooBig, // aligned size overflows usize
    UnsupportedAlignment, // allocator cannot guarantee requested alignment
    UnsupportedSize, // allocator does not support requested size
    NotEnoughMemory, // the proverbial hits the fan
    OperationFailed, // failure performing the operation (OS mem mapping error)
    UnsupportedOperation, // alloc, resize, free not supported
}

use super::layout::MemBlockLayoutError;
impl From<MemBlockLayoutError> for AllocError {
    fn from(e: MemBlockLayoutError) -> Self {
        match e {
            MemBlockLayoutError::InvalidAlignment => AllocError::InvalidAlignment,
            MemBlockLayoutError::AlignedSizeTooBig => AllocError::AlignedSizeTooBig,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_error_converts_to_alloc_error() {
        assert_eq!(AllocError::InvalidAlignment,
                   From::from(MemBlockLayoutError::InvalidAlignment));
        assert_eq!(AllocError::AlignedSizeTooBig,
                   From::from(MemBlockLayoutError::AlignedSizeTooBig));
    }
}

