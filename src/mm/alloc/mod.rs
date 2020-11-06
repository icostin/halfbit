use core::ptr;
use core::mem;
use core::ops::{ Drop, Deref, DerefMut };
use super::layout::MemBlockLayoutError;
use super::layout::NonZeroMemBlockLayout;

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
    fn name(&self) -> &'static str;
}

/* AllocatorRef *************************************************************/
pub struct AllocatorRef<'a> {
    raw_allocator: &'a (dyn RawAllocator + 'a)
}

impl<'a> AllocatorRef<'a> {
    pub fn new(raw_allocator: &'a dyn RawAllocator) -> AllocatorRef<'a> {
        AllocatorRef { raw_allocator }
    }
    fn get_raw_allocator_mut(&self) -> &'a mut dyn RawAllocator {
        unsafe {
            let a = self.raw_allocator as *const dyn RawAllocator;
            let b = a as *mut dyn RawAllocator;
            let c = &mut *b;
            c
            //&mut *((self.raw_allocator as *const dyn RawAllocator) as *mut dyn RawAllocator)
        }
    }
    pub fn alloc<T: Sized>(
        &self,
        value: T,
    ) -> Result<AllocObject<'a, T>, AllocError> {
        let ra: &'a mut dyn RawAllocator = self.get_raw_allocator_mut();
        let layout = NonZeroMemBlockLayout::from_type::<T>();
        let alloc_result = ra.alloc(layout);
        match alloc_result {
            Ok(ptr) => {
                unsafe { ptr::write(ptr as *mut T, value) };
                let ao: AllocObject<'a, T> = AllocObject {
                    ptr: unsafe { &mut *(ptr as *mut T) },
                    //allocator: AllocatorRef::new(ra) //.get_ref()
                    allocator: ra.get_ref()
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

impl<'a> (dyn RawAllocator + 'a) {
    fn get_ref(&'a self) -> AllocatorRef<'a> {
        AllocatorRef::new(self)
    }
}

/* AllocObject **************************************************************/
#[derive(Debug)]
pub struct AllocObject<'a, T> {
    ptr: *mut T,
    allocator: AllocatorRef<'a>
}

impl<'a, T> Drop for AllocObject<'a, T> {
    fn drop(&mut self) {
        mem::drop(unsafe{&mut *self.ptr });
        let raw_allocator = self.allocator.get_raw_allocator_mut();
        unsafe {
            raw_allocator.free(self.ptr as *mut u8,
                NonZeroMemBlockLayout::from_type::<T>())
        };
    }
}


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

