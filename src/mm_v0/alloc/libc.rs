use crate::mm_v0::layout::NonZeroMemBlockLayout;
use super::AllocError;
use super::RawAllocator;

const USIZE_BYTE_COUNT: usize = core::mem::size_of::<usize>();

pub struct MallocRawAllocator { }

impl MallocRawAllocator {
    pub fn new() -> Self {
        Self { }
    }
}

unsafe impl RawAllocator for MallocRawAllocator {
    fn alloc(
        &mut self,
        layout: NonZeroMemBlockLayout
    ) -> Result<*mut u8, AllocError> {
        if layout.align_as_usize() > 2 * USIZE_BYTE_COUNT {
            return Err(AllocError::UnsupportedAlignment);
        }

        let r = unsafe { libc::malloc(layout.size_as_usize() as libc::size_t) as *mut u8 };
        if r.is_null() {
            Err(AllocError::NotEnoughMemory)
        } else {
            Ok(r)
        }
    }
    unsafe fn free(
        &mut self,
        ptr: *mut u8,
        _layout: NonZeroMemBlockLayout,
    ) {
        libc::free(ptr as *mut libc::c_void);
    }
    fn name(&self) -> &'static str { "Malloc" }
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use crate::mm_v0::layout::MemBlockLayout;

    #[test]
    fn malloc_1_byte() {
        let ra = MallocRawAllocator::new();
        let a = AllocatorRef::new(&ra);
        let p = a.alloc(123u8).unwrap();
        assert_eq!(*p, 123u8);
    }

    #[test]
    fn malloc_fails_for_ridiculously_large_size() {
        let mut ra = MallocRawAllocator::new();
        let l = MemBlockLayout::new(usize::MAX, 1).unwrap();
        let n = l.to_non_zero_layout().unwrap();
        assert_eq!(ra.alloc(n).unwrap_err(), AllocError::NotEnoughMemory);
    }

}
