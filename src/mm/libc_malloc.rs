use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;
use super::Allocator;
use super::AllocError;
use super::NonNull;

const USIZE_BYTE_COUNT: usize = core::mem::size_of::<usize>();
const MALLOC_ALIGNMENT: usize = 2 * USIZE_BYTE_COUNT;

pub struct Malloc { }

impl Malloc {
    pub fn new() -> Self {
        Self { }
    }
}

unsafe impl Allocator for Malloc {
    unsafe fn alloc(
        &self,
        size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        if align.get() > MALLOC_ALIGNMENT {
            Err(AllocError::UnsupportedAlignment)
        } else {
            NonNull::new(unsafe {
                libc::malloc(size.get() as libc::size_t) as *mut u8
            }).ok_or(AllocError::NotEnoughMemory)
        }
    }
    unsafe fn free(
        &self,
        ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        _align: Pow2Usize
    ) {
        libc::free(ptr.as_ptr() as *mut libc::c_void);
    }
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        new_larger_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        NonNull::new(
            libc::realloc(
                ptr.as_ptr() as *mut libc::c_void,
                new_larger_size.get() as libc::size_t
            ) as *mut u8
        ).ok_or(AllocError::NotEnoughMemory)
    }
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        _current_size: NonZeroUsize,
        new_smaller_size: NonZeroUsize,
        _align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        NonNull::new(
            libc::realloc(
                ptr.as_ptr() as *mut libc::c_void,
                new_smaller_size.get() as libc::size_t
            ) as *mut u8
        ).ok_or(AllocError::NotEnoughMemory)
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
        "libc-malloc"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malloc_1_byte() {
        let a = Malloc::new();
        assert_eq!(*a.to_ref().alloc_item(123_u8).unwrap(), 123_u8);
    }

    #[test]
    fn malloc_fails_for_ridiculously_large_size() {
        let a = Malloc::new();
        assert_eq!(
            unsafe { a.alloc(
                NonZeroUsize::new(usize::MAX).unwrap(),
                Pow2Usize::one()
            ) }.unwrap_err(),
            AllocError::NotEnoughMemory);
    }

    #[test]
    fn grow_works() {
        let a = Malloc::new();
        let p1 = unsafe { a.alloc(
            NonZeroUsize::new(1).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        unsafe { *p1.as_ptr() = 0xAA_u8 };
        let p2 = unsafe { a.alloc(
            NonZeroUsize::new(1).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        let p3 = unsafe {
            a.grow(
                p1,
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(128).unwrap(),
                Pow2Usize::one())
        }.unwrap();
        assert_eq!(unsafe { *p3.as_ptr() }, 0xAA_u8);
        unsafe {
            a.free(
                p2,
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            );
            a.free(
                p3,
                NonZeroUsize::new(128).unwrap(),
                Pow2Usize::one()
            );
        }

    }
}

