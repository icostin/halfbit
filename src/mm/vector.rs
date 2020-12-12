use core::ptr::NonNull;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;

use super::Allocator;
use super::AllocatorRef;
use super::AllocError;

#[derive(Debug)]
pub struct Vector<'a, T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    allocator: AllocatorRef<'a>,
}

use super::nop_alloc::NOP_ALLOCATOR;

impl<'a, T> Vector<'a, T> {

    pub fn new(allocator: AllocatorRef<'a>) -> Vector<'a, T> {
        let item_size = core::mem::size_of::<T>();
        if item_size == 0 {
            panic!("zero sized types!");
        }
        Vector {
            allocator: allocator,
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    pub fn map_slice(slice: &'a [T]) -> Vector<'a, T> {
        Vector {
            allocator: NOP_ALLOCATOR.to_ref(),
            ptr: NonNull::new(slice.as_ptr() as *mut T).unwrap(),
            len: slice.len(),
            cap: 0
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn reserve(&mut self, count: usize) -> Result<(), AllocError> {
        let item_size = core::mem::size_of::<T>();
        debug_assert!(item_size != 0);
        let max_cap = usize::MAX / item_size;
        if count > max_cap - self.len {
            return Err(AllocError::UnsupportedSize);
        }
        let len_needed = self.len + count;
        if len_needed <= self.cap {
            return Ok(());
        }
        let mut cap_to_try = Pow2Usize::from_smaller_or_equal_usize(len_needed)
            .map(|x| core::cmp::min(x.get(), max_cap)).unwrap_or(len_needed);
        let item_align = core::mem::align_of::<T>();
        loop {
            match unsafe { self.allocator.alloc_or_grow(
                    self.ptr.cast::<u8>(),
                    self.cap * item_size,
                    NonZeroUsize::new(cap_to_try * item_size).unwrap(),
                    Pow2Usize::new(item_align).unwrap())
            } {
                Ok(new_ptr) => {
                    self.ptr = new_ptr.cast::<T>();
                    self.cap = cap_to_try;
                    return Ok(());
                },
                Err(e) => {
                    if cap_to_try == len_needed {
                        return Err(e);
                    }
                    cap_to_try = len_needed;
                }
            }
        }
    }

    pub fn push(&mut self, v: T) -> Result<(), (AllocError, T)> {
        if let Err(e) = self.reserve(1) {
            return Err((e, v));
        }
        debug_assert!(self.len < self.cap);
        unsafe {
            core::ptr::write(self.ptr.as_ptr().offset(self.len as isize), v);
        }
        self.len += 1;

        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;
            Some(unsafe {
                core::ptr::read(self.ptr.as_ptr().offset(self.len as isize))
            })
        }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), core::cmp::min(self.len, self.cap)) }
    }

    pub fn append_from_slice(&mut self, src: &[T]) -> Result<(), AllocError>
    where T: Copy {
        self.reserve(src.len())?;
        unsafe {
            let mut p = self.ptr.as_ptr().offset(self.len as isize);
            for v in src {
                core::ptr::write(p, *v);
                p = p.offset(1);
            }
        }
        self.len += src.len();
        Ok(())
    }
}

impl<'a, T> Drop for Vector<'a, T> {
    fn drop(&mut self) {
        for i in 0..self.len {
            unsafe {
                core::ptr::drop_in_place(self.ptr.as_ptr().offset(i as isize));
            }
        }
        if self.cap != 0 {
            unsafe {
                self.allocator.free(
                    self.ptr.cast::<u8>(),
                    NonZeroUsize::new(core::mem::size_of::<T>() * self.cap).unwrap(),
                    Pow2Usize::new(core::mem::align_of::<T>()).unwrap()
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::no_sup_allocator;
    use super::super::SingleAlloc;

    #[test]
    fn new_vector_is_empty() {
        let a = no_sup_allocator();
        let v: Vector<'_, u16> = Vector::new(a.to_ref());
        assert!(v.is_empty());
    }

    #[test]
    fn failed_push_returns_original_value() {
        let a = no_sup_allocator();
        let mut v: Vector<'_, u16> = Vector::new(a.to_ref());
        let (e, x) = v.push(0xAA55u16).unwrap_err();
        assert_eq!(e, AllocError::UnsupportedOperation);
        assert_eq!(x, 0xAA55u16);
    }

    #[test]
    fn pop_on_empty_vector_returns_none() {
        let a = no_sup_allocator();
        let ar = a.to_ref();
        let mut v = ar.vector::<u16>();
        assert!(v.pop().is_none());
        assert!(v.pop().is_none());
    }


    #[test]
    fn simple_push_pop_works() {
        let mut buffer = [0u8; 2];
        let a = SingleAlloc::new(&mut buffer);
        let ar = a.to_ref();
        let mut v = ar.vector::<u16>();
        v.push(0x1234_u16).unwrap();
        assert_eq!(v.len(), 1_usize);
        assert_eq!(v.pop().unwrap(), 0x1234_u16);
    }

    #[test]
    fn vector_is_usable_after_push_failure() {
        let mut buffer = [0u8; 4];
        let a = SingleAlloc::new(&mut buffer);
        let ar = a.to_ref();
        let mut v = ar.vector::<u16>();

        v.push(0x1234_u16).unwrap();
        v.push(0x5678_u16).unwrap();
        assert_eq!(v.len(), 2_usize);

        let (e, x) = v.push(0x9ABC_u16).unwrap_err();
        assert_eq!(e, AllocError::NotEnoughMemory);
        assert_eq!(x, 0x9ABC_u16);
        assert_eq!(v.len(), 2_usize);

        assert_eq!(v.pop().unwrap(), 0x5678_u16);

        v.push(0xDEF0_u16).unwrap();

        assert_eq!(v.pop().unwrap(), 0xDEF0_u16);
        assert_eq!(v.pop().unwrap(), 0x1234_u16);
    }

    #[test]
    fn reserve_with_count_that_overflows_usize() {
        let mut buffer = [0u8; 4];
        let a = SingleAlloc::new(&mut buffer);
        let ar = a.to_ref();
        let mut v = ar.vector::<u16>();
        v.push(0x1234_u16).unwrap();
        assert_eq!(v.reserve(usize::MAX).unwrap_err(), AllocError::UnsupportedSize);
    }

    struct PretendAlloc<'a>(&'a mut [u8]);
    unsafe impl Allocator for PretendAlloc<'_> {
        unsafe fn alloc(
            &self,
            _size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, AllocError> {
            Ok(NonNull::new(self.0.as_ptr() as *mut u8).unwrap())
        }
        unsafe fn free(
            &self,
            _ptr: NonNull<u8>,
            _current_size: NonZeroUsize,
            _align: Pow2Usize) {
        }
        unsafe fn grow(
            &self,
            ptr: NonNull<u8>,
            _current_size: NonZeroUsize,
            _new_larger_size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, AllocError> {
            Ok(ptr)
        }
    }
    #[test]
    fn reserve_with_large_count_that_prevents_power_of_2_rounding_of_cap() {
        let mut buffer = [0u8; 4];
        let a = PretendAlloc(&mut buffer);
        let ar = a.to_ref();
        let mut v = ar.vector::<u8>();
        v.push(0xA1_u8).unwrap();
        v.reserve(usize::MAX / 2 + 1).unwrap();
        assert_eq!(v.cap(), usize::MAX / 2 + 2);
    }

    #[test]
    fn get_slice_from_vector() {
        let mut buffer = [0u8; 4];
        let a = SingleAlloc::new(&mut buffer);
        let ar = a.to_ref();
        let mut v = ar.vector::<u16>();

        v.push(0x1234_u16).unwrap();
        v.push(0x5678_u16).unwrap();

        let s = v.as_slice();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0], 0x1234);
        assert_eq!(s[1], 0x5678);
    }

    #[test]
    fn slice_as_vector_works() {
        let mut x: [u16; 4] = [ 2, 4, 6, 8 ];
        {
            let v = Vector::map_slice(&x);
            //assert_eq!(v.push(10).unwrap_err(), (AllocError::UnsupportedOperation, 10));
            assert_eq!(v.as_slice(), [ 2_u16, 4_u16, 6_u16, 8_u16 ]);
            assert_eq!(v.cap(), 0);
        }
        x[2] = 66;
        {
            let v = Vector::map_slice(&x);
            //assert_eq!(v.push(10).unwrap_err(), (AllocError::UnsupportedOperation, 10));
            assert_eq!(v.as_slice(), [ 2_u16, 4_u16, 66_u16, 8_u16 ]);
            assert_eq!(v.cap(), 0);
        }
    }

    #[test]
    fn mut_slice_from_vector_created_from_slice_must_be_empty() {
        let x: [u16; 4] = [ 2, 4, 6, 8 ];
        let mut v = Vector::map_slice(&x);
        assert_eq!(v.as_mut_slice().len(), 0);
    }

    #[test]
    #[should_panic(expected = "zero sized")]
    fn panic_creating_vector_with_zero_sized_items() {
        let mut buffer = [0u8; 4];
        let a = SingleAlloc::new(&mut buffer);
        let ar = a.to_ref();
        let _v = ar.vector::<()>();
    }
}

