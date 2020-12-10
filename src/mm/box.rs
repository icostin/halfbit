use crate::num::{
    NonZeroUsize,
    Pow2Usize,
};

use super::{
    NonNull,
    Allocator,
    AllocatorRef,
    AllocError,
};

#[derive(Debug)]
pub struct Box<'a, T> {
    allocator: AllocatorRef<'a>,
    ptr: NonNull<T>,
}

impl<'a, T> Box<'a, T> {
    pub fn new(
        allocator: AllocatorRef<'a>,
        value: T,
    ) -> Result<Self, (AllocError, T)> {
        let size = core::mem::size_of::<T>();
        if size == 0 {
            return Ok(Box{ allocator: allocator, ptr: NonNull::dangling() });
        }

        let size = NonZeroUsize::new(size).unwrap();
        let align = Pow2Usize::new(core::mem::align_of::<T>()).unwrap();
        match allocator.alloc(size, align) {
            Ok(ptr) => {
                let ptr = ptr.cast::<T>();
                unsafe { core::ptr::write(ptr.as_ptr(), value) };
                Ok(Box { allocator: allocator, ptr: ptr })
            },
            Err(e) => Err((e, value))
        }
    }
}

impl<'a, T> Drop for Box<'a, T> {
    fn drop(&mut self) {
        unsafe{ core::ptr::drop_in_place(self.ptr.as_ptr()); }
        let size = core::mem::size_of::<T>();
        if size == 0 { return; }
        let size = NonZeroUsize::new(size).unwrap();
        let align = Pow2Usize::new(core::mem::align_of::<T>()).unwrap();
        unsafe { self.allocator.free(self.ptr.cast::<u8>(), size, align) };
    }
}

impl<'a, T> core::ops::Deref for Box<'a, T> {
    type Target = T;
    fn deref (&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T> core::ops::DerefMut for Box<'a, T> {
    fn deref_mut (&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::no_sup_allocator;
    use super::super::SingleAlloc;

    #[test]
    fn zero_sized_boxed_payload_works_without_allocating() {
        let a = no_sup_allocator();
        let _b = Box::new(a.to_ref(), ()).unwrap();
    }

    #[test]
    fn alloc_failure_errors_out_with_original_value() {
        let a = no_sup_allocator();
        let b = Box::new(a.to_ref(), 0x12345_u32);
        let (e, v) = b.unwrap_err();
        assert_eq!(e, AllocError::UnsupportedOperation);
        assert_eq!(v, 0x12345_u32);
    }

    #[test]
    fn create_and_drop_box() {
        let mut buffer = [0u8; 16];
        let a = SingleAlloc::new(&mut buffer);
        {
            let b = Box::new(a.to_ref(), 0xAA55u16).unwrap();
            assert_eq!(*b, 0xAA55u16);
            assert!(a.is_in_use());
        }
        assert!(!a.is_in_use());
    }

    use core::sync::atomic::{ AtomicUsize, Ordering };
    struct IncOnDrop<'a> {
        drop_counter: &'a AtomicUsize,
    }

    impl<'a> Drop for IncOnDrop<'a> {
        fn drop(&mut self) {
            self.drop_counter.fetch_add(1, Ordering::SeqCst);
        }
    }


    #[test]
    fn drop_gets_called_on_boxed_item() {
        let drop_count = AtomicUsize::new(0);
        let mut buffer = [0u8; 16];
        let a = SingleAlloc::new(&mut buffer);
        {
            let _b = Box::new(a.to_ref(), IncOnDrop {
                drop_counter: &drop_count
            });
            assert!(a.is_in_use());
        }
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

}
