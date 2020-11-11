use core::marker::PhantomData;
use crate::num::usize_align_up;
use crate::mm::layout::NonZeroMemBlockLayout;
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
        if self.begin_addr == (ptr as usize) + layout.size_as_usize() {
            self.begin_addr = ptr as usize;
        }
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

}

