use core::ptr::NonNull;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;

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

impl From<AllocError> for core::fmt::Error {
    fn from(_e: AllocError) -> Self {
        Self { }
    }
}

impl<T> From<(AllocError, T)> for AllocError {
    fn from(src: (AllocError, T)) -> Self {
        src.0
    }
}

pub unsafe trait Allocator {
    unsafe fn alloc(
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
    fn supports_contains(&self) -> bool { false }
    fn contains(
        &self,
        _ptr: NonNull<u8>
    ) -> bool {
        panic!("contains not implemented!");
    }
    fn name(&self) -> &'static str { "some-allocator" }
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
    unsafe fn alloc(
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

pub mod nop_alloc;
pub use nop_alloc::NOP_ALLOCATOR as NOP_ALLOCATOR;

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

pub mod string;
pub use string::String as String;

pub mod rc;
pub use rc::Rc as Rc;
pub use rc::RcWeak as RcWeak;

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
            self.grow(ptr, NonZeroUsize::new(current_size).unwrap(), new_larger_size, align)
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
        let _r = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        };
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
        match unsafe {
            a.grow(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        } { _ => {} };
    }

    #[test]
    #[should_panic(expected = "shrink not implemented")]
    fn default_shrink_panics() {
        let a = DefaultAllocator { };
        match unsafe {
            a.shrink(
                NonNull::dangling(),
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        } { _ => {} };
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

    #[test]
    fn fmt_error_from_alloc_error() {
        let _fe: core::fmt::Error = AllocError::OperationFailed.into();
    }

    extern crate std;
    use std::string::String as StdString;
    use core::fmt::Write;

    #[test]
    fn fmt_on_default_allocator() {
        let a = DefaultAllocator { };
        let mut s = StdString::new();
        write!(s, "{:?}", a.to_ref()).unwrap();
        assert!(s.as_str().contains("allocator@"));
    }

    struct ShrinkTestAllocator { }
    unsafe impl Allocator for ShrinkTestAllocator {
        unsafe fn shrink(
            &self,
            _ptr: NonNull<u8>,
            _current_size: NonZeroUsize,
            _new_smaller_size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, AllocError> {
            Ok(NonNull::new(0xA1B2C3D4_usize as *mut u8).unwrap())
        }
    }
    #[test]
    fn allocator_ref_shrink_calls_allocator_shrink() {
        let a = ShrinkTestAllocator { };
        let ar = a.to_ref();
        let p = unsafe {
            ar.shrink(
                NonNull::dangling(),
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::new(1).unwrap()
            )
        }.unwrap();
        assert_eq!(p.as_ptr(), 0xA1B2C3D4_usize as *mut u8);
    }

    struct ContainsSupTestAllocator { }
    unsafe impl Allocator for ContainsSupTestAllocator {
        fn supports_contains(&self) -> bool { true }
        fn contains(&self, ptr: NonNull<u8>) -> bool {
            (ptr.as_ptr() as usize) & 1 == 1
        }
    }
    #[test]
    fn contains_on_allocator_ref_forwards_to_allocator() {
        let a = ContainsSupTestAllocator { };
        let ar = a.to_ref();
        assert!(ar.supports_contains());
        assert!(ar.contains(NonNull::new(1 as *mut u8).unwrap()));
        assert!(!ar.contains(NonNull::new(2 as *mut u8).unwrap()));
    }

    #[test]
    fn allocator_ref_to_ref_copies_internal_ref() {
        let a = DefaultAllocator { };
        let ar = a.to_ref();
        let arr = ar.to_ref();
        let size = core::mem::size_of::<AllocatorRef<'_>>();
        assert_eq!(
            unsafe { core::slice::from_raw_parts(&ar as *const AllocatorRef as *const u8, size) },
            unsafe { core::slice::from_raw_parts(&arr as *const AllocatorRef as *const u8, size) });
    }

    struct AllocOrGrowTestAllocator();
    unsafe impl Allocator for AllocOrGrowTestAllocator {
        unsafe fn alloc(
            &self,
            size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, AllocError> {
            Ok(NonNull::new((size.get() * 1000 + size.get()) as *mut u8).unwrap())
        }
        unsafe fn grow(
            &self,
            ptr: NonNull<u8>,
            current_size: NonZeroUsize,
            new_larger_size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, AllocError> {
            Ok(NonNull::new(((ptr.as_ptr() as usize) - current_size.get() + new_larger_size.get()) as *mut u8).unwrap())
        }
    }
    #[test]
    fn alloc_or_grow_first_allocates_then_grows() {
        let a = AllocOrGrowTestAllocator();
        let ar = a.to_ref();
        let mut p = NonNull::<u8>::dangling();
        p = unsafe { ar.alloc_or_grow(p, 0, NonZeroUsize::new(123).unwrap(), Pow2Usize::one()).unwrap() };
        assert_eq!(p.as_ptr() as usize, 123123);
        p = unsafe { ar.alloc_or_grow(p, 123, NonZeroUsize::new(456).unwrap(), Pow2Usize::one()).unwrap() };
        assert_eq!(p.as_ptr() as usize, 123456);
        p = unsafe { ar.alloc_or_grow(p, 456, NonZeroUsize::new(789).unwrap(), Pow2Usize::one()).unwrap() };
        assert_eq!(p.as_ptr() as usize, 123789);
    }
}

