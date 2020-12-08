pub use core::ptr::NonNull;

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
}

pub unsafe trait Allocator {
    fn alloc(
        &self,
        _size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        panic!("alloc not implemented");
    }
    unsafe fn free(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _align: Pow2Usize
    ) {
        panic!("free not implemented!");
    }
    unsafe fn grow(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_larger_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        panic!("grow not implemented");
    }
    unsafe fn shrink(
        &self,
        _ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        panic!("shrink not implemented");
    }
    fn supports_contains(&self) -> bool {
        false
    }
    fn contains(
        &self,
        _ptr: NonNull<u8>
    ) -> bool {
        panic!("contains not implemented!");
    }
    fn name(&self) -> &'static str {
        "some-allocator"
    }
    fn to_ref(&self) -> AllocatorRef
    where Self: Sized {
        AllocatorRef { allocator: self as &dyn Allocator }
    }
}

#[derive(Copy, Clone)]
pub struct AllocatorRef<'a> {
    allocator: &'a (dyn Allocator + 'a)
}

impl<'a> core::fmt::Debug for AllocatorRef<'a> {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>)
    -> core::result::Result<(), core::fmt::Error> {
        write!(fmt, "{}@{:X}", self.name(), ((self.allocator as *const dyn Allocator) as *const u8) as usize)
    }
}

unsafe impl<'a> Allocator for AllocatorRef<'a> {
    fn alloc(
        &self,
        size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        self.allocator.alloc(size, align)
    }
    unsafe fn free(
        &self,
        ptr: NonNull<u8>,
        size: NonZeroUsize,
        align: Pow2Usize) {
        self.allocator.free(ptr, size, align);
    }
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_larger_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        self.allocator.grow(ptr, current_size, new_larger_size, align)
    }
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_smaller_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        self.allocator.shrink(ptr, current_size, new_smaller_size, align)
    }
    fn supports_contains(&self) -> bool {
        self.allocator.supports_contains()
    }
    fn contains(
        &self,
        ptr: NonNull<u8>
    ) -> bool {
        self.allocator.contains(ptr)
    }
    fn name(&self) -> &'static str {
        self.allocator.name()
    }
    fn to_ref(&self) -> AllocatorRef
    where Self: Sized {
        *self
    }
}

pub mod no_sup_alloc;
pub use no_sup_alloc::no_sup_allocator as no_sup_allocator;

pub mod single_alloc;
pub use single_alloc::SingleAlloc as SingleAlloc;

pub mod bump_alloc;
pub use bump_alloc::BumpAllocator as BumpAllocator;

#[cfg(feature = "use-libc")]
pub mod libc_malloc;
#[cfg(feature = "use-libc")]
pub use libc_malloc::Malloc as Malloc;

pub mod r#box;
pub use r#box::Box as Box;

pub mod vector;
pub use vector::Vector as Vector;

impl<'a> AllocatorRef<'a> {
    pub fn alloc_item<T: Sized>(self, v: T) -> Result<Box<'a, T>, (AllocError, T)> {
        Box::new(self, v)
    }

    pub fn vector<T: Sized>(&'a self) -> Vector<'a, T> {
        Vector::new(*self)
    }

    pub unsafe fn alloc_or_grow(
        &'a self,
        ptr: NonNull<u8>,
        current_size: usize,
        new_larger_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        if current_size == 0 {
            self.alloc(new_larger_size, align)
        } else {
            self.grow(
                ptr,
                NonZeroUsize::new(current_size).unwrap(),
                new_larger_size,
                align)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DefaultAllocator { }
    unsafe impl Allocator for DefaultAllocator { }

    #[test]
    #[should_panic(expected = "alloc not implemented")]
    fn default_alloc_panics() {
        let a = DefaultAllocator { };
        let _r = a.alloc(NonZeroUsize::new(1).unwrap(),
            Pow2Usize::new(1).unwrap());
    }

    #[test]
    #[should_panic(expected = "free not implemented")]
    fn default_free_panics() {
        let a = DefaultAllocator { };
        unsafe {
            a.free(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::new(1).unwrap()
            );
        }
    }

    #[test]
    #[should_panic(expected = "grow not implemented")]
    fn default_grow_panics() {
        let a = DefaultAllocator { };
        unsafe {
            a.grow(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        }.unwrap_or(NonNull::dangling());
    }

    #[test]
    #[should_panic(expected = "shrink not implemented")]
    fn default_shrink_panics() {
        let a = DefaultAllocator { };
        unsafe {
            a.shrink(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        }.unwrap_or(NonNull::dangling());
    }

    #[test]
    fn default_supports_contains_returns_false() {
        let a = DefaultAllocator { };
        assert!(!a.supports_contains());
    }

    #[test]
    #[should_panic(expected = "contains not implemented")]
    fn default_contains_panics() {
        let a = DefaultAllocator { };
        a.contains(NonNull::dangling());
    }

    #[test]
    fn default_name_responds() {
        let a = DefaultAllocator { };
        assert!(a.name().contains("allocator"));
    }

    #[test]
    fn default_to_ref_works() {
        let a = DefaultAllocator { };
        let ar = a.to_ref();
        assert!(ar.name().contains("allocator"));
    }

}

