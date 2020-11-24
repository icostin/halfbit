use core::ptr;
use core::mem;
use core::ops::{ Drop, Deref, DerefMut };
use super::layout::MemBlockLayoutError;
use super::layout::{ MemBlockLayout, NonZeroMemBlockLayout };
use core::num::NonZeroUsize;

/* AllocError ***************************************************************/
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

impl From<MemBlockLayoutError> for AllocError {
    fn from(e: MemBlockLayoutError) -> Self {
        match e {
            MemBlockLayoutError::InvalidAlignment => AllocError::InvalidAlignment,
            MemBlockLayoutError::AlignedSizeTooBig => AllocError::AlignedSizeTooBig,
        }
    }
}

/* RawAllocator *************************************************************/
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
    unsafe fn grow(
        &mut self,
        _ptr: *mut u8,
        _current_layout: NonZeroMemBlockLayout,
        _new_size: NonZeroUsize) -> Result<*mut u8, AllocError> {
        Err(AllocError::UnsupportedOperation)
    }
    unsafe fn shrink(
        &mut self,
        _ptr: *mut u8,
        _current_layout: NonZeroMemBlockLayout,
        _new_size: NonZeroUsize) -> Result<*mut u8, AllocError> {
        Err(AllocError::UnsupportedOperation)
    }
    fn name(&self) -> &'static str;
    fn to_ref(&self) -> AllocatorRef where Self: Sized { AllocatorRef::new(self) }
}

/* AllocatorRef *************************************************************/
#[derive(Copy, Clone)]
pub struct AllocatorRef<'a> {
    raw_allocator: &'a (dyn RawAllocator + 'a)
}

impl<'a> AllocatorRef<'a> {
    pub fn new(raw_allocator: &'a dyn RawAllocator) -> AllocatorRef<'a> {
        AllocatorRef { raw_allocator }
    }
    fn get_raw_allocator_mut(&self) -> &'a mut dyn RawAllocator {
        let a = self.raw_allocator as *const dyn RawAllocator;
        let b = a as *mut dyn RawAllocator;
        unsafe { &mut *b }
    }
    pub fn alloc<T: Sized>(
        &self,
        value: T,
    ) -> Result<Box<'a, T>, AllocError> {
        let ra: &'a mut dyn RawAllocator = self.get_raw_allocator_mut();
        let layout = NonZeroMemBlockLayout::from_type::<T>();
        let alloc_result = ra.alloc(layout);
        match alloc_result {
            Ok(ptr) => {
                unsafe { ptr::write(ptr as *mut T, value) };
                let ao: Box<'a, T> = Box {
                    ptr: unsafe { &mut *(ptr as *mut T) },
                    allocator: AllocatorRef::new(ra),
                };
                Ok(ao)
            },
            Err(e) => Err(e)
        }
    }
}

impl<'a> core::fmt::Debug for AllocatorRef<'a> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>)
    -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{}@{:X}", self.raw_allocator.name(), ((self.raw_allocator as *const dyn RawAllocator) as *const u8) as usize)
    }
}

#[derive(Debug)]
pub struct Box<'a, T> {
    ptr: *mut T,
    allocator: AllocatorRef<'a>
}

impl<'a, T> Drop for Box<'a, T> {
    fn drop(&mut self) {
        mem::drop(unsafe{&mut *self.ptr });
        let raw_allocator = self.allocator.get_raw_allocator_mut();
        unsafe {
            raw_allocator.free(self.ptr as *mut u8,
                NonZeroMemBlockLayout::from_type::<T>())
        };
    }
}

impl<'a, T> Deref for Box<'a, T> {
    type Target = T;
    fn deref (&self) -> &Self::Target {
        unsafe { & *self.ptr }
    }
}
impl<'a, T> DerefMut for Box<'a, T> {
    fn deref_mut (&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

pub mod vector;
pub use self::vector::Vector;

pub mod null;
pub use self::null::NullRawAllocator;

pub mod bump;
pub use self::bump::BumpRawAllocator;

#[cfg(feature = "use-libc")]
pub mod libc;

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

