use core::ptr::NonNull;
use core::fmt::Display;
use core::fmt::Formatter;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;

use super::HbAllocator;
use super::HbAllocatorRef;
use super::HbAllocError;

#[derive(Debug)]
pub struct Vector<'a, T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
    allocator: HbAllocatorRef<'a>,
}

use super::nop_alloc::NOP_ALLOCATOR;

impl<'a, T> Vector<'a, T> {

    pub fn new(allocator: HbAllocatorRef<'a>) -> Vector<'a, T> {
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

    pub fn reserve(&mut self, count: usize) -> Result<(), HbAllocError> {
        let item_size = core::mem::size_of::<T>();
        debug_assert!(item_size != 0);
        let max_cap = usize::MAX / item_size;
        if count > max_cap - self.len {
            return Err(HbAllocError::UnsupportedSize);
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

    pub fn push(&mut self, v: T) -> Result<(), (HbAllocError, T)> {
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

    pub fn append_from_slice(&mut self, src: &[T]) -> Result<(), HbAllocError>
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

    pub fn append_vector(
        &mut self,
        tail: Vector<'a, T>
    ) -> Result<(), HbAllocError> {
        self.reserve(tail.len())?;
        unsafe {
            core::ptr::copy_nonoverlapping(
                tail.as_slice().as_ptr(),
                self.ptr.as_ptr().offset(self.len as isize),
                tail.len());
            tail.allocator.free(
                tail.ptr.cast::<u8>(),
                NonZeroUsize::new(core::mem::size_of::<T>() * tail.cap).unwrap(),
                Pow2Usize::new(core::mem::align_of::<T>()).unwrap()
            );
            self.len += tail.len;
            core::mem::forget(tail)
        }
        Ok(())
    }

    pub fn from_slice(
        allocator: HbAllocatorRef<'a>,
        src: &[T]
    ) -> Result<Self, HbAllocError>
    where T: Copy {
        let mut v: Self = Vector::new(allocator);
        v.append_from_slice(src)?;
        Ok(v)
    }

    pub fn dup<'b>(
        &self,
        allocator: HbAllocatorRef<'b>,
    ) -> Result<Vector<'b, T>, HbAllocError>
    where T: Copy {
        Vector::from_slice(allocator, self.as_slice())
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

impl<'a, T: PartialEq> PartialEq for Vector<'a, T> {
    fn eq<'b>(&self, other: &Vector<'b, T>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl<'a, T: Display> Display for Vector<'a, T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut first = true;
        for v in self.as_slice() {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }
            Display::fmt(v, f)?;
        }
        Ok(())
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
        assert_eq!(e, HbAllocError::UnsupportedOperation);
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
        assert_eq!(e, HbAllocError::NotEnoughMemory);
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
        assert_eq!(v.reserve(usize::MAX).unwrap_err(), HbAllocError::UnsupportedSize);
    }

    struct PretendAlloc<'a>(&'a mut [u8]);
    unsafe impl HbAllocator for PretendAlloc<'_> {
        unsafe fn alloc(
            &self,
            _size: NonZeroUsize,
            _align: Pow2Usize
        ) -> Result<NonNull<u8>, HbAllocError> {
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
        ) -> Result<NonNull<u8>, HbAllocError> {
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
            //assert_eq!(v.push(10).unwrap_err(), (HbAllocError::UnsupportedOperation, 10));
            assert_eq!(v.as_slice(), [ 2_u16, 4_u16, 6_u16, 8_u16 ]);
            assert_eq!(v.cap(), 0);
        }
        x[2] = 66;
        {
            let v = Vector::map_slice(&x);
            //assert_eq!(v.push(10).unwrap_err(), (HbAllocError::UnsupportedOperation, 10));
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

    #[test]
    fn from_slice() {
        let mut buffer = [0u8; 100];
        let a = SingleAlloc::new(&mut buffer);
        let x: [u16; 4] = [ 2, 4, 6, 8 ];
        let mut v = Vector::from_slice(a.to_ref(), &x).unwrap();
        v.as_mut_slice()[2] = 7;
        assert_eq!(v.as_slice(), [2_u16, 4_u16, 7_u16, 8_u16 ]);
    }

    #[test]
    fn partial_eq() {
        let mut buffer = [0u8; 100];
        let a = SingleAlloc::new(&mut buffer);
        let x: [u16; 4] = [ 2, 4, 6, 8 ];
        let mut v = Vector::from_slice(a.to_ref(), &x).unwrap();
        v.as_mut_slice()[2] = 7;
        let v2 = Vector::map_slice(&[2_u16, 4, 7, 8]);
        assert_eq!(v, v2);
    }

    #[test]
    fn append_vector() {
        let mut buf1 = [0_u8; 100];
        let mut buf2 = [0_u8; 100];
        let a1 = SingleAlloc::new(&mut buf1);
        let a2 = SingleAlloc::new(&mut buf2);
        let x1: [u16; 4] = [ 2, 4, 6, 8 ];
        let x2: [u16; 3] = [ 1, 3, 5 ];
        let v1 = Vector::from_slice(a1.to_ref(), &x1).unwrap();
        let mut v2 = Vector::from_slice(a2.to_ref(), &x2).unwrap();
        v2.append_vector(v1).unwrap();
        assert_eq!(v2.as_slice(), [ 1_u16, 3_u16, 5_u16, 2_u16, 4_u16, 6_u16, 8_u16 ]);
        assert!(!a1.is_in_use());
    }

    #[test]
    fn dup() {
        let mut buf1 = [0_u8; 100];
        let mut buf2 = [0_u8; 100];
        let a1 = SingleAlloc::new(&mut buf1);
        let a2 = SingleAlloc::new(&mut buf2);
        let x1: [u16; 4] = [ 2, 4, 6, 8 ];
        let v1 = Vector::from_slice(a1.to_ref(), &x1).unwrap();
        let v2 = v1.dup(a2.to_ref()).unwrap();
        core::mem::drop(v1);
        assert_eq!(v2.as_slice(), [ 2_u16, 4, 6, 8 ]);
        assert!(a2.is_in_use());
    }
}

