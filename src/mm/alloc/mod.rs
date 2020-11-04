
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

use super::layout::NonZeroMemBlockLayout;

pub unsafe trait RawAllocator {
    fn alloc(
        &mut self,
        layout: NonZeroMemBlockLayout
    ) -> Result<*mut u8, AllocError>;
    unsafe fn free(
        &mut self,
        ptr: *mut u8,
        layout: NonZeroMemBlockLayout
    );
    fn name(&self) -> &'static str;
}

pub struct AllocatorRef<'a> {
    raw_allocator: &'a dyn RawAllocator
}

impl<'a> AllocatorRef<'a> {
    pub fn new(raw_allocator: &'a dyn RawAllocator) -> AllocatorRef<'a> {
        AllocatorRef { raw_allocator }
    }
}

impl<'a> core::fmt::Debug for AllocatorRef<'a> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>)
    -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{}@{:X}", self.raw_allocator.name(), ((self.raw_allocator as *const dyn RawAllocator) as *const u8) as usize)
    }
}

impl dyn RawAllocator {
    fn get_ref(&self) -> AllocatorRef {
        AllocatorRef::new(self)
    }
}

#[derive(Debug)]
pub struct AllocObject<'a, T> {
    ptr: *mut T,
    allocator: AllocatorRef<'a>
}

use core::ops::{ Deref, DerefMut };
impl<'a, T> Deref for AllocObject<'a, T> {
    type Target = T;
    fn deref (&self) -> &Self::Target {
        unsafe { & *self.ptr }
    }
}
impl<'a, T> DerefMut for AllocObject<'a, T> {
    fn deref_mut (&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}


pub mod null;

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

