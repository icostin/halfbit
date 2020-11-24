use core::ops::Index;
use super::{ AllocError, AllocatorRef };
use crate::mm::layout::{ MemBlockLayout };
use crate::num::Pow2Usize;

#[derive(Debug)]
pub struct Vector<'a, T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    allocator: AllocatorRef<'a>
}

impl<'a, T> Vector<'a, T> {

    pub fn new(allocator: AllocatorRef<'a>) -> Vector<'a, T> {
        Vector {
            ptr: core::ptr::null_mut::<T>(),
            len: 0,
            cap: 0,
            allocator: allocator
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn available_slot_count(&self) -> usize {
        self.cap - self.len
    }

    fn make_slots_available(&mut self, count: usize) -> Result<(), AllocError> {
        if count <= self.available_slot_count() {
            return Ok(());
        }
        let count_needed = self.len.checked_add(count);
        if count_needed.is_none() {
            return Err(AllocError::UnsupportedSize);
        }
        let count_needed = count_needed.unwrap();
        let count_to_request =
            Pow2Usize::from_lesser_or_equal_usize(count_needed)
            .and_then(|x| Some(x.get()))
            .or_else(|| Some(count_needed))
            .unwrap();

        let item_layout = MemBlockLayout::from_type::<T>();
        let l = item_layout.to_layout_for_array(count_to_request)
            .or_else(|| item_layout.to_layout_for_array(count_needed));

        if l.is_none() {
            return Err(AllocError::UnsupportedSize);
        }

        if self.cap == 0 {

        }

        Err(AllocError::NotImplemented)
    }

    pub fn push(&mut self, _item: T) -> Result<(), AllocError> {
        Err(AllocError::NotImplemented)
    }

}

impl<T> Index<usize> for Vector<'_, T> {
    type Output = T;
    fn index(&self, pos: usize) -> &Self::Output {
        let s = unsafe { core::slice::from_raw_parts(self.ptr, self.len) };
        &s[pos]
    }

}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;

    #[test]
    fn new_produces_an_empty_vector() {
        let n = NullRawAllocator::new();
        let v: Vector<'_, u8> = Vector::new(n.to_ref());
        assert!(v.is_empty());
        assert_eq!(v.len(), 0usize);
    }

    #[test]
    fn push_when_using_null_alloc_fails() {
        let a = NullRawAllocator::new();
        let mut v: Vector<'_, u64> = Vector::new(a.to_ref());
        assert_eq!(v.push(1234u64).unwrap_err(), AllocError::NotEnoughMemory);
    }

    #[test]
    fn push_one_item_successfully() {
        let mut buffer = [0u8, 0x20];
        let ba = BumpRawAllocator::new(&mut buffer);
        let mut v: Vector<'_, u64> = Vector::new(ba.to_ref());
        assert!(v.push(1324u64).is_ok());
    }

}

