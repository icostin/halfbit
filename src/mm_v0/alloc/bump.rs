use core::marker::PhantomData;
use crate::num::NonZeroUsize;
use crate::num::usize_align_up;
use crate::mm_v0::layout::NonZeroMemBlockLayout;
use super::AllocError;
use super::RawAllocator;

pub struct BumpRawAllocator<'a> {
    begin_addr: usize,
    end_addr: usize,
    lifetime: PhantomData<&'a u8>,
}

impl<'a> BumpRawAllocator<'a> {
    pub fn new(buffer: &'a [u8]) -> BumpRawAllocator<'a> {
        let begin_addr = buffer.as_ptr() as usize;
        let end_addr = begin_addr + buffer.len();
        BumpRawAllocator {
            begin_addr: begin_addr,
            end_addr: end_addr,
            lifetime: PhantomData
        }
    }
    fn is_last_allocation(
        &self,
        ptr: *mut u8,
        layout: &NonZeroMemBlockLayout) -> bool {
        self.begin_addr == (ptr as usize) + layout.size_as_usize()
    }
    fn unallocated_size(&self) -> usize {
        self.end_addr - self.begin_addr
    }

}

unsafe impl<'a> RawAllocator for BumpRawAllocator<'a> {
    fn alloc(
        &mut self,
        layout: NonZeroMemBlockLayout
    ) -> Result<*mut u8, AllocError> {
        let addr = usize_align_up(self.begin_addr, layout.align());
        if addr.is_none() {
            return Err(AllocError::NotEnoughMemory);
        }
        let addr = addr.unwrap();
        self.begin_addr = addr.checked_add(layout.size_as_usize()).map_or(
            Err(AllocError::NotEnoughMemory),
            |x| if x > self.end_addr {
                Err(AllocError::NotEnoughMemory)
            } else {
                Ok(x)
            })?;
        Ok(addr as *mut u8)
    }
    unsafe fn free(
        &mut self,
        ptr: *mut u8,
        layout: NonZeroMemBlockLayout
    ) {
        if self.is_last_allocation(ptr, &layout) {
            self.begin_addr = ptr as usize;
        }
    }
    unsafe fn grow(
        &mut self,
        ptr: *mut u8,
        current_layout: NonZeroMemBlockLayout,
        new_size: NonZeroUsize
    ) -> Result<*mut u8, AllocError> {
        let unallocated_size = self.unallocated_size();
        if self.is_last_allocation(ptr, &current_layout)
            && new_size.get() - current_layout.size_as_usize()
                <= unallocated_size {
            self.begin_addr += new_size.get() - current_layout.size_as_usize();
            Ok(ptr)
        } else {
            let new_layout = NonZeroMemBlockLayout::from_parts(
                new_size, current_layout.align());
            let new_ptr = self.alloc(new_layout)?;
            core::ptr::copy(ptr, new_ptr, current_layout.size_as_usize());
            Ok(new_ptr)
        }
    }
    unsafe fn shrink(
        &mut self,
        ptr: *mut u8,
        current_layout: NonZeroMemBlockLayout,
        new_size: NonZeroUsize) -> Result<*mut u8, AllocError> {
        if self.is_last_allocation(ptr, &current_layout) {
            self.begin_addr -= current_layout.size_as_usize() - new_size.get();
        }

        Ok(ptr)
    }
    fn name(&self) -> &'static str { "BumpAllocator" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn relevant_name() {
        let nra = BumpRawAllocator::new(&mut [0u8; 0]);
        let name = nra.name();
        assert!(name.contains("bump") || name.contains("Bump"));
        assert!(name.contains("alloc") || name.contains("Alloc"));
    }

    #[test]
    fn raw_alloc_1_byte_in_a_1_byte_buffer_works() {
        let mut buffer = [0u8; 1];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let layout = NonZeroMemBlockLayout::from_type::<u8>();
        assert_eq!(ra.alloc(layout).unwrap() as *const u8,
            buffer.as_ptr());
    }

    #[test]
    fn raw_alloc_2_bytes_in_a_1_byte_buffer_fails() {
        let mut buffer = [0u8; 1];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let layout = NonZeroMemBlockLayout::from_type::<u16>();
        assert_eq!(ra.alloc(layout).unwrap_err(),
            AllocError::NotEnoughMemory);
    }

    #[test]
    fn alloc_1_byte_in_a_1_byte_buffer_works() {
        let mut buffer = [0u8; 1];
        let ra = BumpRawAllocator::new(&mut buffer);
        let a = AllocatorRef::new(&ra);
        let r = a.alloc(42u8);
        assert!(r.is_ok());
        let mut o = r.unwrap();
        assert_eq!(*o, 42u8);
        *o = 0xABu8;
        assert_eq!(*o, 0xABu8);
    }

    #[test]
    fn dropping_last_allocation_reclaims_memory() {
        let mut buffer = [0u8; 1];
        let ra = BumpRawAllocator::new(&mut buffer);
        let a = AllocatorRef::new(&ra);
        {
            let r = a.alloc(42u8);
            assert!(r.is_ok());
            let mut o = r.unwrap();
            assert_eq!(*o, 42u8);
            *o = 0xABu8;
            assert_eq!(*o, 0xABu8);
        }

        {
            let r = a.alloc(0x5Au8);
            assert!(r.is_ok());
            let mut o = r.unwrap();
            assert_eq!(*o, 0x5Au8);
            *o = 0xA5u8;
            assert_eq!(*o, 0xA5u8);
        }
    }

    #[test]
    fn grow_last_allocation_succeeds() {
        let mut buffer = [0xAAu8; 2];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p1 = 0x99u8 };
        let p2 = unsafe { ra.grow(p1, NonZeroMemBlockLayout::from_type::<u8>(),
            NonZeroUsize::new(2usize).unwrap()) }.unwrap();
        let s = unsafe { core::slice::from_raw_parts(p2, 2usize) };
        assert_eq!(s, [0x99u8, 0xAAu8]);
    }

    #[test]
    fn grow_last_allocation_fails() {
        let mut buffer = [0xAAu8; 2];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p1 = 0x99u8 };
        let e2 = unsafe { ra.grow(p1, NonZeroMemBlockLayout::from_type::<u8>(),
            NonZeroUsize::new(3usize).unwrap()) }.unwrap_err();
        assert_eq!(e2, AllocError::NotEnoughMemory);
    }

    #[test]
    fn grow_by_doing_a_new_allocation_succeeds() {
        let mut buffer = [0xAAu8; 4];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p1 = 0x5Au8 };
        let p2 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p2 = 0xA5u8 };
        let p3 = unsafe { ra.grow(p1, NonZeroMemBlockLayout::from_type::<u8>(),
            NonZeroUsize::new(2usize).unwrap()) }.unwrap();
        let s = unsafe { core::slice::from_raw_parts(p3, 2usize) };
        assert_eq!(s, [0x5Au8, 0xAAu8]);
        assert_eq!(unsafe { *p2 }, 0xA5u8);
    }

    #[test]
    fn fail_to_grow_by_reallocation() {
        let mut buffer = [0xAAu8; 4];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p1 = 0x5Au8 };
        let p2 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p2 = 0xA5u8 };
        let e3 = unsafe { ra.grow(p1, NonZeroMemBlockLayout::from_type::<u8>(),
            NonZeroUsize::new(3usize).unwrap()) }.unwrap_err();
        assert_eq!(e3, AllocError::NotEnoughMemory);
        assert_eq!(unsafe { *p1 }, 0x5Au8);
        assert_eq!(unsafe { *p2 }, 0xA5u8);
    }

    #[test]
    fn shrink_last_allocation_reclaims_memory() {
        let mut buffer = [0xAAu8; 2];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<[u8; 2usize]>()).unwrap();
        unsafe { *p1 = 0x12u8 };
        let p2 = unsafe { ra.shrink(p1, NonZeroMemBlockLayout::from_type::<[u8; 2usize]>(), NonZeroUsize::new(1usize).unwrap()) }.unwrap();
        assert_eq!(unsafe { *p2 }, 0x12u8);
        let p3 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        assert_eq!(p3 as usize, (p2 as usize) + 1usize);
    }

    #[test]
    fn shrink_non_last_allocation() {
        let mut buffer = [0xAAu8; 4];
        let mut ra = BumpRawAllocator::new(&mut buffer);
        let p1 = ra.alloc(NonZeroMemBlockLayout::from_type::<[u8; 2usize]>()).unwrap();
        let s = unsafe { core::slice::from_raw_parts_mut(p1, 2usize) };
        s[0] = 0x5Au8;
        s[1] = 0xA5u8;
        let p2 = ra.alloc(NonZeroMemBlockLayout::from_type::<u8>()).unwrap();
        unsafe { *p2 = 0xBBu8 };
        let p3 = unsafe {
            ra.shrink(p1, NonZeroMemBlockLayout::from_type::<[u8; 2usize]>(),
            NonZeroUsize::new(1usize).unwrap())
        }.unwrap();
        assert_eq!(unsafe { *p3 }, 0x5Au8);
    }

}

