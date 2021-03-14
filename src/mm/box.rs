use core::ptr::NonNull;
use core::ops::Deref;
use core::ops::DerefMut;
use core::marker::Unsize;
use core::fmt;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;

use super::Allocator;
use super::AllocatorRef;
use super::AllocError;

pub struct Box<'a, T: ?Sized> {
    allocator: AllocatorRef<'a>,
    ptr: NonNull<T>,
}

impl<'a, T: Sized> Box<'a, T> {
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
        match unsafe { allocator.alloc(size, align) } {
            Ok(ptr) => {
                let ptr = ptr.cast::<T>();
                unsafe { core::ptr::write(ptr.as_ptr(), value) };
                Ok(Box { allocator: allocator, ptr: ptr })
            },
            Err(e) => Err((e, value))
        }
    }
}

impl<'a, T: ?Sized> Box<'a, T> {
    pub unsafe fn to_parts(self) -> (AllocatorRef<'a>, NonNull<T>) {
        let x = core::mem::ManuallyDrop::new(self);
        (x.allocator, x.ptr)
    }

    pub fn to_dyn<U>(self) -> Box<'a, U>
    where
        T: Unsize<U>,
        U: ?Sized
    {
        let a = self.allocator;
        let p = self.ptr;
        core::mem::forget(self);
        Box {
            allocator: a,
            ptr: p,
        }
    }
}

impl<'a, T: ?Sized> Deref for Box<'a, T> {
    type Target = T;
    fn deref (&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T: ?Sized> DerefMut for Box<'a, T> {
    fn deref_mut (&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<'a, T: ?Sized> Drop for Box<'a, T> {
    fn drop(&mut self) {
        let v: &T = self.deref();
        let size = core::mem::size_of_val(v);
        unsafe{ core::ptr::drop_in_place(self.ptr.as_ptr()); }
        if size != 0 {
            let size = NonZeroUsize::new(size).unwrap();
            let align = Pow2Usize::new(core::mem::align_of_val(&v)).unwrap();
            unsafe { self.allocator.free(self.ptr.cast::<u8>(), size, align) };
        }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for Box<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v: &T = self.deref();
        write!(f, "halfbit::Box(")
            .and_then(|_| v.fmt(f))
            .and_then(|_| write!(f, ")"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::no_sup_allocator;
    use super::super::SingleAlloc;

    #[test]
    fn size_of_val_on_0_sized() {
        assert_eq!(core::mem::size_of_val(&()), 0);
    }
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
            assert_eq!(drop_count.load(Ordering::SeqCst), 0);
        }
        assert_eq!(drop_count.load(Ordering::SeqCst), 1);
    }

    trait TestDynBoxTrait: fmt::Debug {
        fn tada(&self) -> u8;
        fn inc(&mut self);
    }

    #[derive(Debug)]
    struct TestDynBoxA(u8);
    impl TestDynBoxTrait for TestDynBoxA {
        fn tada(&self) -> u8 { self.0 }
        fn inc(&mut self) { self.0 += 1 }
    }
    #[derive(Debug)]
    struct TestDynBoxB<'a>(&'a mut usize);
    impl<'a> TestDynBoxTrait for TestDynBoxB<'a> {
        fn tada(&self) -> u8 { 0xAB }
        fn inc(&mut self) {}
    }
    impl<'a> Drop for TestDynBoxB<'a> {
        fn drop(&mut self) {
            *self.0 += 1;
        }
    }

    #[test]
    fn dyn_box_ab() {
        use crate::mm::bump_alloc::BumpAllocator;
        let mut buf = [0_u8; 256];
        let ba = BumpAllocator::new(&mut buf);
        assert_eq!(ba.space_left(), 256);
        let mut drop_count = 0_usize;
        extern crate std;
        use std::dbg;
        dbg!(ba.space_left());
        let b = Box::new(ba.to_ref(), TestDynBoxB(&mut drop_count)).unwrap();
        let a = Box::new(ba.to_ref(), TestDynBoxA(0x5A)).unwrap();
        assert_eq!(a.tada(), 0x5A);
        assert_eq!(b.tada(), 0xAB);
        {
            let mut tb = b.to_dyn::<dyn TestDynBoxTrait>();
            let mut ta = a.to_dyn::<dyn TestDynBoxTrait>();
            ta.inc();
            tb.inc();
            assert_eq!(tb.tada(), 0xAB);
            assert_eq!(ta.tada(), 0x5B);
            extern crate std;
            use std::string::String as StdString;
            use core::fmt::Write;
            let mut s = StdString::new();
            write!(s, "{:?}", ta).unwrap();
            assert!(s.contains("TestDynBox"));
        }
        assert_eq!(drop_count, 1);
        assert_eq!(ba.space_left(), 256);
    }

    #[test]
    fn to_dyn() {
        let mut buffer = [0u8; 16];
        let a = SingleAlloc::new(&mut buffer);
        {
            let c: Box<'_, dyn fmt::Debug>;
            {
                let b = Box::new(a.to_ref(), 0xAA55u16).unwrap();
                assert_eq!(*b, 0xAA55u16);
                assert!(a.is_in_use());
                c = b.to_dyn();
            }
            assert!(a.is_in_use());
            extern crate std;
            use std::format;
            assert_eq!(format!("{:06?}", c), "halfbit::Box(043605)");
        }
        assert!(!a.is_in_use());
    }

}
