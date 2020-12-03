use core::cell::UnsafeCell;

use crate::num::{
    NonZeroUsize,
    Pow2Usize,
};

use super::{
    NonNull,
    AllocError,
    Allocator,
};

pub struct SingleAllocState<'a> {
    buffer: &'a mut [u8],
    used: usize,
}
pub struct SingleAlloc<'a> {
    state: UnsafeCell<SingleAllocState<'a>>
}

impl<'a> SingleAlloc<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        SingleAlloc {
            state: SingleAllocState {
                buffer: buffer,
                used: 0usize,
            }.into(),
        }
    }
    fn check_allocation(
        &self,
        ptr: NonNull<u8>,
        size: NonZeroUsize,
        align: Pow2Usize,
    ) {
        let state: &'a SingleAllocState<'a> = unsafe {
            &*(self.state.get() as *mut SingleAllocState<'a>)
        };
        if state.used == 0 {
            panic!("cannot free what hasn't been allocated!");
        } else if state.buffer.as_ptr() != ptr.as_ptr() {
            panic!("bad pointer");
        } else if state.used != size.get() {
            panic!("bad size");
        } else if ((state.buffer.as_ptr() as usize) & (align.get() - 1)) != 0 {
            panic!("bad alignment");
        }

    }
}

unsafe impl<'a> Allocator for SingleAlloc<'a> {
    fn alloc(
        &self,
        size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        let state: &'a mut SingleAllocState<'a> = unsafe {
            &mut *(self.state.get() as *mut SingleAllocState<'a>)
        };
        if state.used != 0 {
            Err(AllocError::OperationFailed)
        } else if ((state.buffer.as_ptr() as usize) & (align.get() - 1)) != 0 {
            Err(AllocError::UnsupportedAlignment)
        } else if size.get() > state.buffer.len() {
            Err(AllocError::NotEnoughMemory)
        } else {
            state.used = size.get();
            Ok(NonNull::new(state.buffer.as_mut_ptr()).unwrap())
        }
    }
    unsafe fn free(
        &self,
        ptr: NonNull<u8>,
        size: NonZeroUsize,
        align: Pow2Usize) {
        self.check_allocation(ptr, size, align);
        let state: &'a mut SingleAllocState<'a> = {
            &mut *(self.state.get() as *mut SingleAllocState<'a>)
        };
        state.used = 0;
    }
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_larger_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        self.check_allocation(ptr, current_size, align);
        let state: &'a mut SingleAllocState<'a> = {
            &mut *(self.state.get() as *mut SingleAllocState<'a>)
        };
        if new_larger_size.get() > state.buffer.len() {
            Err(AllocError::NotEnoughMemory)
        } else {
            state.used = new_larger_size.get();
            Ok(ptr)
        }
    }
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_smaller_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, AllocError> {
        self.check_allocation(ptr, current_size, align);
        let state: &'a mut SingleAllocState<'a> = {
            &mut *(self.state.get() as *mut SingleAllocState<'a>)
        };
        if new_smaller_size.get() > state.buffer.len() {
            Err(AllocError::NotEnoughMemory)
        } else {
            state.used = new_smaller_size.get();
            Ok(ptr)
        }
    }
    fn supports_contains(&self) -> bool {
        true
    }
    fn contains(
        &self,
        ptr: NonNull<u8>
    ) -> bool {
        let state: &'a SingleAllocState<'a> = unsafe {
            &*(self.state.get() as *mut SingleAllocState<'a>)
        };
        let begin = state.buffer.as_ptr() as usize;
        let end = begin + state.buffer.len();
        let ptr = ptr.as_ptr() as usize;
        ptr >= begin && ptr < end
    }
    fn name(&self) -> &'static str {
        "single-alloc"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_responds_appropriately() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        assert!(a.name().contains("single-alloc"));
    }

    #[test]
    fn alloc_smaller_than_buffer_size_works_on_new_instance() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let r = a.alloc(NonZeroUsize::new(6).unwrap(),
                        Pow2Usize::new(1).unwrap());
        assert_eq!(r.unwrap(), NonNull::new(buf.as_mut_ptr()).unwrap());
    }

    #[test]
    fn alloc_buffer_size_works_on_new_instance() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let r = a.alloc(NonZeroUsize::new(7).unwrap(),
                        Pow2Usize::new(1).unwrap());
        assert_eq!(r.unwrap(), NonNull::new(buf.as_mut_ptr()).unwrap());
    }
    #[test]
    fn alloc_with_unsuitable_alignment_fails() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let r = a.alloc(NonZeroUsize::new(8).unwrap(),
                        Pow2Usize::max());
        assert_eq!(r.unwrap_err(), AllocError::UnsupportedAlignment);
    }


    #[test]
    fn alloc_larger_than_buffer_size_fails() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let r = a.alloc(NonZeroUsize::new(8).unwrap(),
                        Pow2Usize::new(1).unwrap());
        assert_eq!(r.unwrap_err(), AllocError::NotEnoughMemory);
    }

    #[test]
    fn freeing_previous_allocation_works() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let size = NonZeroUsize::new(6).unwrap();
        let align = Pow2Usize::new(1).unwrap();
        let ptr = a.alloc(size, align).unwrap();
        unsafe { a.free(ptr, size, align) };
    }

    #[test]
    #[should_panic(expected = "hasn't been allocated")]
    fn freeing_unallocated_buffer_panics() {
        let mut buf = [0u8; 7];
        let ptr = NonNull::new(buf.as_mut_ptr()).unwrap();
        let a = SingleAlloc::new(&mut buf);
        let size = NonZeroUsize::new(1).unwrap();
        let align = Pow2Usize::new(1).unwrap();
        unsafe { a.free(ptr, size, align) };
    }

    #[test]
    #[should_panic(expected = "bad pointer")]
    fn freeing_mismatched_pointer_panics() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let size = NonZeroUsize::new(6).unwrap();
        let align = Pow2Usize::new(1).unwrap();
        let _ptr = a.alloc(size, align).unwrap();
        unsafe { a.free(NonNull::dangling(), size, align) };
    }

    #[test]
    #[should_panic(expected = "bad size")]
    fn freeing_mismatched_size_panics() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let size = NonZeroUsize::new(6).unwrap();
        let mismatched_size = NonZeroUsize::new(5).unwrap();
        let align = Pow2Usize::new(1).unwrap();
        let ptr = a.alloc(size, align).unwrap();
        unsafe { a.free(ptr, mismatched_size, align) };
    }

    #[test]
    fn grow_smaller_than_buffer_size_works() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let p = a.alloc(NonZeroUsize::new(3).unwrap(),
                        Pow2Usize::new(1).unwrap()).unwrap();
        let r = unsafe {
            a.grow(
                p,
                NonZeroUsize::new(3).unwrap(),
                NonZeroUsize::new(5).unwrap(),
                Pow2Usize::new(1).unwrap(),
            )
        };
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), NonNull::new(buf.as_mut_ptr()).unwrap());
    }

    #[test]
    fn grow_to_buffer_size_works() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let p = a.alloc(NonZeroUsize::new(3).unwrap(),
                        Pow2Usize::new(1).unwrap()).unwrap();
        let r = unsafe {
            a.grow(
                p,
                NonZeroUsize::new(3).unwrap(),
                NonZeroUsize::new(7).unwrap(),
                Pow2Usize::new(1).unwrap(),
            )
        };
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), NonNull::new(buf.as_mut_ptr()).unwrap());
    }

    #[test]
    fn grow_to_larger_than_buffer_size_fails() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let p = a.alloc(NonZeroUsize::new(3).unwrap(),
                        Pow2Usize::new(1).unwrap()).unwrap();
        let r = unsafe {
            a.grow(
                p,
                NonZeroUsize::new(3).unwrap(),
                NonZeroUsize::new(8).unwrap(),
                Pow2Usize::new(1).unwrap(),
            )
        };
        assert!(r.is_err());
        assert_eq!(r.unwrap_err(), AllocError::NotEnoughMemory);
    }

    #[test]
    fn shrink_from_buffer_size_works() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let p = a.alloc(NonZeroUsize::new(7).unwrap(),
                        Pow2Usize::new(1).unwrap()).unwrap();
        let r = unsafe {
            a.shrink(
                p,
                NonZeroUsize::new(7).unwrap(),
                NonZeroUsize::new(3).unwrap(),
                Pow2Usize::new(1).unwrap(),
            )
        };
        assert!(r.is_ok());
        assert_eq!(r.unwrap(), NonNull::new(buf.as_mut_ptr()).unwrap());
    }

    #[test]
    fn contains_is_supported() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        assert!(a.supports_contains());
    }

    #[test]
    fn contains_on_allocated_pointer_returns_true() {
        let mut buf = [0u8; 7];
        let a = SingleAlloc::new(&mut buf);
        let p = a.alloc(NonZeroUsize::new(3).unwrap(),
                        Pow2Usize::new(1).unwrap()).unwrap();
        assert!(a.contains(p));
    }

    #[test]
    fn contains_on_unallocated_buffer_pointer_still_returns_true() {
        let mut buf = [0u8; 7];
        let p = NonNull::new(buf.as_mut_ptr()).unwrap();
        let a = SingleAlloc::new(&mut buf);
        assert!(a.contains(p));
    }

}

