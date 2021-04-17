use core::cell::UnsafeCell;
use core::marker::PhantomData;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;
use crate::num::usize_align_up;

use super::NonNull;
use super::HbAllocator;
use super::HbAllocError;

struct BumpAllocatorState<'a> {
    begin_addr: usize,
    current_addr: usize,
    end_addr: usize,
    lifeline: PhantomData<&'a u8>,
}

pub struct BumpAllocator<'a> {
    state: UnsafeCell<BumpAllocatorState<'a>>
}

impl<'a> BumpAllocator<'a> {
    pub fn new(buffer: &'a mut [u8]) -> Self {
        let b = buffer.as_ptr() as usize;
        let e = b + buffer.len();
        BumpAllocator {
            state: BumpAllocatorState {
                begin_addr: b,
                current_addr: b,
                end_addr: e,
                lifeline: PhantomData
            }.into()
        }
    }
    fn is_last_allocation(
        &self,
        ptr: NonNull<u8>,
        size: NonZeroUsize
    ) -> bool {
        let state: &'a BumpAllocatorState<'a> = unsafe {
            &*(self.state.get() as *mut BumpAllocatorState<'a>)
        };
        state.current_addr == (ptr.as_ptr() as usize) + size.get()
    }
    pub fn space_left(&self) -> usize {
        let state: &'a BumpAllocatorState<'a> = unsafe {
            &*(self.state.get() as *mut BumpAllocatorState<'a>)
        };
        state.end_addr - state.current_addr
    }
}

unsafe impl<'a> HbAllocator for BumpAllocator<'a> {
    unsafe fn alloc(
        &self,
        size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, HbAllocError> {
        let state: &'a mut BumpAllocatorState<'a> = &mut
            *(self.state.get() as *mut BumpAllocatorState<'a>);
        usize_align_up(state.current_addr, align)
            .map_or(None, |v| v.checked_add(size.get()))
            .map_or(None, |v| if v <= state.end_addr {
                let addr = state.current_addr;
                state.current_addr = v;
                NonNull::new(addr as *mut u8)
            } else { None })
            .ok_or(HbAllocError::NotEnoughMemory)
    }
    unsafe fn free(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        _align: Pow2Usize
    ) {
        if self.is_last_allocation(ptr, current_size) {
            let state: &'a mut BumpAllocatorState<'a> = &mut
                *(self.state.get() as *mut BumpAllocatorState<'a>);
            state.current_addr -= current_size.get();
        }
    }
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_larger_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, HbAllocError> {
        if self.is_last_allocation(ptr, current_size) &&
            align.is_non_null_ptr_aligned(ptr) {
            let state: &'a mut BumpAllocatorState<'a> = &mut 
                *(self.state.get() as *mut BumpAllocatorState<'a>);
            let extra_size = new_larger_size.get() - current_size.get();
            if extra_size <= state.end_addr - state.current_addr {
                state.current_addr += extra_size;
                Ok(ptr)
            } else {
                Err(HbAllocError::NotEnoughMemory)
            }
        } else {
            let new_ptr = self.alloc(new_larger_size, align)?;
            core::ptr::copy(ptr.as_ptr(), new_ptr.as_ptr(), current_size.get());
            Ok(new_ptr)
        }
    }
    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        current_size: NonZeroUsize,
        new_smaller_size: NonZeroUsize,
        align: Pow2Usize
    ) -> Result<NonNull<u8>, HbAllocError> {
        if !align.is_non_null_ptr_aligned(ptr) {
            Err(HbAllocError::UnsupportedAlignment)
        } else {
            if self.is_last_allocation(ptr, current_size) {
                let state: &'a mut BumpAllocatorState<'a> = &mut
                    *(self.state.get() as *mut BumpAllocatorState<'a>);
                state.current_addr -= current_size.get() - new_smaller_size.get();
            }
            Ok(ptr)
        }
    }
    fn supports_contains(&self) -> bool { true }
    fn contains(&self, ptr: NonNull<u8>) -> bool {
        let state: &'a BumpAllocatorState<'a> = unsafe {
            &*(self.state.get() as *mut BumpAllocatorState<'a>)
        };
        let addr = ptr.as_ptr() as usize;
        state.begin_addr <= addr && addr < state.end_addr
    }
    fn name(&self) -> &'static str { "bump-allocator" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocator_name_contains_bump() {
        let mut buffer = [0_u8; 16];
        let a = BumpAllocator::new(&mut buffer);
        assert!(a.name().contains("bump"));
    }


    #[test]
    fn alloc_1_byte_in_a_1_byte_buffer_works() {
        let mut buffer = [0_u8; 1];
        let a = BumpAllocator::new(&mut buffer);
        assert_eq!(
            unsafe {
                a.alloc(NonZeroUsize::new(1).unwrap(), Pow2Usize::one())
            }.unwrap().as_ptr(),
            buffer.as_mut_ptr());
    }

    #[test]
    fn alloc_2_bytes_in_a_1_byte_buffer_fails() {
        let mut buffer = [0_u8; 1];
        let a = BumpAllocator::new(&mut buffer);
        assert_eq!(
            unsafe {
                a.alloc(NonZeroUsize::new(2).unwrap(), Pow2Usize::one())
            }.unwrap_err(),
            HbAllocError::NotEnoughMemory);
    }

    #[test]
    fn dropping_last_allocation_reclaims_memory() {
        let mut buffer = [0_u8; 1];
        let a = BumpAllocator::new(&mut buffer);
        {
            let mut b = a.to_ref().alloc_item(42_u8).unwrap();
            assert_eq!(*b, 42_u8);
            *b = 0xAB_u8;
            assert_eq!(*b, 0xAB_u8);

        }
        {
            let mut b = a.to_ref().alloc_item(0x5A_u8).unwrap();
            assert_eq!(*b, 0x5A_u8);
            *b = 0xA5_u8;
            assert_eq!(*b, 0xA5_u8);
        }
    }

    #[test]
    fn grow_last_allocation_succeeds() {
        let mut buffer = [0xAA_u8; 2];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            )
        }.unwrap();
        unsafe { *p1.as_ptr() = 0x99_u8 };
        let p2 = unsafe {
            a.grow(
                p1,
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::one())
        }.unwrap();
        let s = unsafe { core::slice::from_raw_parts(p2.as_ptr(), 2_usize) };
        assert_eq!(s, [0x99_u8, 0xAA_u8]);
    }

    #[test]
    fn grow_last_allocation_fails() {
        let mut buffer = [0xAA_u8; 2];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            )
        }.unwrap();
        unsafe { *p1.as_ptr() = 0x99_u8 };
        let e2 = unsafe {
            a.grow(p1,
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(3).unwrap(),
                Pow2Usize::one())
        }.unwrap_err();
        assert_eq!(e2, HbAllocError::NotEnoughMemory);
    }

    #[test]
    fn grow_by_doing_a_new_allocation_succeeds() {
        let mut buffer = [0xAA_u8; 4];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            )
        }.unwrap();
        unsafe { *p1.as_ptr() = 0x5A_u8 };
        let p2 = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            )
        }.unwrap();
        unsafe { *p2.as_ptr() = 0xA5_u8 };
        let p3 = unsafe {
            a.grow(
                p1,
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(2).unwrap(),
                Pow2Usize::one())
        }.unwrap();
        let s = unsafe { core::slice::from_raw_parts(p3.as_ptr(), 2_usize) };
        assert_eq!(s, [0x5A_u8, 0xAA_u8]);
        assert_eq!(unsafe { *p2.as_ptr() }, 0xA5_u8);
    }

    #[test]
    fn fail_to_grow_by_reallocation() {
        let mut buffer = [0xAA_u8; 4];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe {
            a.alloc(
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one()
            )
        }.unwrap();
        unsafe { *p1.as_ptr() = 0x5A_u8 };
        let p2 = unsafe { a.alloc(
            NonZeroUsize::new(1).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        unsafe { *p2.as_ptr() = 0xA5_u8 };
        let e3 = unsafe {
            a.grow(
                p1,
                NonZeroUsize::new(1).unwrap(),
                NonZeroUsize::new(3).unwrap(),
                Pow2Usize::one())
        }.unwrap_err();
        assert_eq!(e3, HbAllocError::NotEnoughMemory);
        assert_eq!(unsafe { *p1.as_ptr() }, 0x5A_u8);
        assert_eq!(unsafe { *p2.as_ptr() }, 0xA5_u8);
    }

    #[test]
    fn shrink_last_allocation_reclaims_memory() {
        let mut buffer = [0xAA_u8; 2];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe { a.alloc(
            NonZeroUsize::new(2).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        unsafe { *p1.as_ptr() = 0x12_u8 };
        let p2 = unsafe {
            a.shrink(
                p1,
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one())
        }.unwrap();
        assert_eq!(unsafe { *p2.as_ptr() }, 0x12_u8);
        let p3 = unsafe { a.alloc(
            NonZeroUsize::new(1).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        assert_eq!(p3.as_ptr(), unsafe { p2.as_ptr().offset(1) });
    }

    #[test]
    fn shrink_non_last_allocation() {
        let mut buffer = [0xAA_u8; 4];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe { a.alloc(
            NonZeroUsize::new(2).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        unsafe { *p1.as_ptr() = 0x5A_u8 };
        unsafe { *p1.as_ptr().offset(1) = 0xA5_u8 };
        let p2 = unsafe { a.alloc(
            NonZeroUsize::new(1).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        unsafe { *p2.as_ptr() = 0xBB_u8 };
        let p3 = unsafe {
            a.shrink(
                p1,
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::one())
        }.unwrap();
        assert_eq!(unsafe { *p3.as_ptr() }, 0x5A_u8);
        assert_eq!(unsafe { *p2.as_ptr() }, 0xBB_u8);
    }

    #[test]
    fn shrink_with_higher_alignment_fails() {
        let mut buffer = [0xAA_u8; 4];
        let a = BumpAllocator::new(&mut buffer);
        let p1 = unsafe { a.alloc(
            NonZeroUsize::new(2).unwrap(),
            Pow2Usize::one()
        ) }.unwrap();
        let e2 = unsafe { 
            a.shrink(
                p1,
                NonZeroUsize::new(2).unwrap(),
                NonZeroUsize::new(1).unwrap(),
                Pow2Usize::max()
            )
        }.unwrap_err();
        assert_eq!(e2, HbAllocError::UnsupportedAlignment);
    }

    #[test]
    fn contains_is_supported() {
        let mut buffer = [0xAA_u8; 4];
        let a = BumpAllocator::new(&mut buffer);
        assert!(a.supports_contains());
    }

    #[test]
    fn contains_true_only_for_pointers_inside_buffer() {
        let mut buffer = [0xAA_u8; 47];
        let b = buffer.as_mut_ptr();
        let n = buffer.len();
        let a = BumpAllocator::new(&mut buffer);
        assert!(a.contains(NonNull::new(b).unwrap()));
        assert!(a.contains(NonNull::new(unsafe { b.offset(n as isize - 1) }).unwrap()));
        assert!(!a.contains(NonNull::new(unsafe { b.offset(n as isize) }).unwrap()));
        assert!(!a.contains(NonNull::new(unsafe { b.offset(-1) }).unwrap()));
    }

}
