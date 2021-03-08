use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ptr::NonNull;
use core::borrow::Borrow;
use core::marker::Unsize;
use core::fmt;

use crate::num::NonZeroUsize;
use crate::num::Pow2Usize;

use super::Allocator;
use super::AllocatorRef;
use super::AllocError;

struct RcCtlBlock<'a> {
    strong: usize,
    weak: usize,
    allocator: AllocatorRef<'a>,
}

type RcData<'a, T> = UnsafeCell<(RcCtlBlock<'a>, T)>;

pub struct Rc<'a, T>
where T: ?Sized {
    data: &'a RcData<'a, T>,
}

pub struct RcWeak<'a, T>
where T: ?Sized {
    data: &'a RcData<'a, T>,
}

unsafe fn free_if_unreferenced<'a, T: ?Sized>(rc_block: &mut (RcCtlBlock<'a>, T)) {

    if rc_block.0.strong != 0 || rc_block.0.weak != 0 { return; }

    let size = core::mem::size_of_val(rc_block);
    let size = NonZeroUsize::new(size).unwrap();

    let align = core::mem::align_of_val(rc_block);
    let align = Pow2Usize::new(align).unwrap();

    let allocator = rc_block.0.allocator;
    allocator.free(
        NonNull::new(rc_block as *mut (RcCtlBlock<'a>, T) as *mut u8).unwrap(),
        size,
        align);
}

impl<'a, T> Rc<'a, T>
where T: Sized {

    pub fn new(
        allocator: AllocatorRef<'a>,
        value: T,
    ) -> Result<Self, (AllocError, T)> {
        let size = core::mem::size_of::<RcData<'a, T>>();
        let size = NonZeroUsize::new(size).unwrap();

        let align = core::mem::align_of::<RcData<'a, T>>();
        let align = Pow2Usize::new(align).unwrap();

        match unsafe { allocator.alloc(size, align) } {
            Ok(ptr) => {
                let ptr = ptr.cast::<RcData<'a, T>>().as_ptr();
                unsafe {
                    core::ptr::write(ptr,
                        UnsafeCell::new(
                            (RcCtlBlock { strong: 1, weak: 0, allocator: allocator },
                             value)));

                    Ok(Rc { data: &mut *ptr })
                }
            },
            Err(e) => Err((e, value))
        }
    }

}

impl<T> Rc<'_, T>
where T: ?Sized {

    pub fn strong_count(rc: &Rc<'_, T>) -> usize {
        let rc_data = unsafe { &*rc.data.get() };
        let rc_block = &rc_data.0;
        rc_block.strong
    }

    pub fn weak_count(rc: &Rc<'_, T>) -> usize {
        let rc_data = unsafe { &*rc.data.get() };
        let rc_block = &rc_data.0;
        rc_block.weak
    }

    pub fn get_mut<'a>(rc: &'a mut Rc<'_, T>) -> Option<&'a mut T> {
        let rc_data = unsafe { &mut *rc.data.get() };
        let rc_block = &rc_data.0;
        if rc_block.strong == 1 && rc_block.weak == 0 {
            Some(&mut rc_data.1)
        } else {
            None
        }
    }

    pub fn ptr_eq<'a, 'b>(a: &Rc<'a, T>, b: &Rc<'b, T>) -> bool {
        a.data as *const RcData<'a, T> as *const u8
            == b.data as *const RcData<'b, T> as *const u8
    }

    pub fn to_dyn<'a, U>(rc: Rc<'a, T>) -> Rc<'a, U>
    where
        T: Unsize<U>,
        U: ?Sized
    {
        let data: &RcData<'a, U> = rc.data;
        core::mem::forget(rc);
        Rc { data }
    }

    pub fn downgrade<'a>(rc: &Rc<'a, T>) -> RcWeak<'a, T> {
        let rc_data = unsafe { &mut *rc.data.get() };
        let mut rc_block = &mut rc_data.0;
        rc_block.weak += 1;
        RcWeak { data: rc.data }
    }

}

impl<'a, T> AsRef<T> for Rc<'a, T> where T: ?Sized {

    fn as_ref(&self) -> &T {
        let rc_data = unsafe { &mut *self.data.get() };
        &rc_data.1
    }

}

impl<'a, T> Borrow<T> for Rc<'a, T> where T: ?Sized {

    fn borrow(&self) -> &T {
        panic!();
    }

}

impl<'a, T> Clone for Rc<'a, T> where T: ?Sized {

    fn clone(&self) -> Rc<'a, T> {
        let rc_data = unsafe { &mut *self.data.get() };
        let mut rc_block = &mut rc_data.0;
        rc_block.strong += 1;
        Rc { data: self.data }
    }

}

impl<'a, T> fmt::Debug for Rc<'a, T> where T: ?Sized + fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rc[{}+{}]{{{:?}}}", Rc::strong_count(self), Rc::weak_count(self), self.as_ref())
    }
}

impl<'a, T> Deref for Rc<'a, T> where T: ?Sized {

    type Target = T;

    fn deref(&self) -> &T {
        panic!();
    }

}

impl<'a, T> Drop for Rc<'a, T> where T: ?Sized {

    fn drop(&mut self) {
        let rc_data = unsafe { &mut *self.data.get() };
        let mut rc_block = &mut rc_data.0;
        assert!(rc_block.strong > 0);
        rc_block.strong -= 1;
        if rc_block.strong == 0 {
            unsafe {
                core::ptr::drop_in_place(&mut rc_data.1 as *mut T);
                free_if_unreferenced(rc_data);
            }
        }
    }

}

impl<'a, T> RcWeak<'a, T> where T: ?Sized {

    pub fn upgrade(&self) -> Option<Rc<'a, T>> {
        let rc_data = unsafe { &mut *self.data.get() };
        let mut rc_block = &mut rc_data.0;
        if rc_block.strong != 0 {
            rc_block.strong += 1;
            Some(Rc { data: self.data })
        } else {
            None
        }
    }

    pub fn strong_count(&self) -> usize {
        let rc_data = unsafe { &*self.data.get() };
        let rc_block = &rc_data.0;
        rc_block.strong
    }

    pub fn weak_count(&self) -> usize {
        let rc_data = unsafe { &*self.data.get() };
        let rc_block = &rc_data.0;
        rc_block.weak
    }

}

impl<'a, T> Clone for RcWeak<'a, T> where T: ?Sized {

    fn clone(&self) -> RcWeak<'a, T> {
        panic!();
    }

}

impl<'a, T> Drop for RcWeak<'a, T> where T: ?Sized {

    fn drop(&mut self) {
        let rc_data = unsafe { &mut *self.data.get() };
        let mut rc_block = &mut rc_data.0;
        assert!(rc_block.weak > 0);
        rc_block.weak -= 1;
        unsafe {
            free_if_unreferenced(rc_data);
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::SingleAlloc;
    extern crate std;

    #[test]
    fn rc_new() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        Rc::new(a.to_ref(), 0_u32).unwrap();
    }

    use core::sync::atomic::{ AtomicUsize, Ordering };
    #[derive(Debug)]
    struct IncOnDrop<'a> {
        drop_counter: &'a AtomicUsize,
    }

    impl<'a> Drop for IncOnDrop<'a> {
        fn drop(&mut self) {
            self.drop_counter.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn not_enough_mem() {
        let mut buffer = [0u8; 8];
        let a = SingleAlloc::new(&mut buffer);
        let (e, v) = Rc::new(a.to_ref(), 123_u32).unwrap_err();
        assert_eq!(e, AllocError::NotEnoughMemory);
        assert_eq!(v, 123_u32);
    }

    #[test]
    fn inner_drop_at_the_right_time() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let dropometer = AtomicUsize::new(0);

        let mut rc1 = Rc::new(a.to_ref(), IncOnDrop { drop_counter: &dropometer }).unwrap();
        assert_eq!(Rc::strong_count(&rc1), 1);
        assert_eq!(Rc::weak_count(&rc1), 0);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());
        assert!(Rc::get_mut(&mut rc1).is_some());

        let w1 = Rc::downgrade(&rc1);
        assert_eq!(Rc::strong_count(&rc1), 1);
        assert_eq!(Rc::weak_count(&rc1), 1);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());
        assert!(Rc::get_mut(&mut rc1).is_none());


        let rc2 = rc1.clone();
        assert_eq!(Rc::strong_count(&rc1), 2);
        assert_eq!(Rc::weak_count(&rc1), 1);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());
        assert!(Rc::ptr_eq(&rc1, &rc2));

        {
            let rc3 = w1.upgrade().unwrap();
            assert_eq!(Rc::strong_count(&rc1), 3);
            assert_eq!(Rc::weak_count(&rc1), 1);
            assert_eq!(dropometer.load(Ordering::SeqCst), 0);
            assert!(a.is_in_use());
            assert!(Rc::ptr_eq(&rc1, &rc3));
        }
        assert_eq!(Rc::strong_count(&rc1), 2);
        assert_eq!(Rc::weak_count(&rc1), 1);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());

        let w2 = Rc::downgrade(&rc2);
        assert_eq!(Rc::strong_count(&rc1), 2);
        assert_eq!(Rc::weak_count(&rc1), 2);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());

        core::mem::drop(rc1);
        assert_eq!(RcWeak::strong_count(&w2), 1);
        assert_eq!(RcWeak::weak_count(&w2), 2);
        assert_eq!(dropometer.load(Ordering::SeqCst), 0);
        assert!(a.is_in_use());

        core::mem::drop(rc2);
        assert_eq!(w2.strong_count(), 0);
        assert_eq!(w2.weak_count(), 2);
        assert_eq!(dropometer.load(Ordering::SeqCst), 1);
        assert!(a.is_in_use());

        assert!(w1.upgrade().is_none());

        core::mem::drop(w1);
        assert_eq!(w2.strong_count(), 0);
        assert_eq!(w2.weak_count(), 1);
        assert_eq!(dropometer.load(Ordering::SeqCst), 1);
        assert!(a.is_in_use());

        core::mem::drop(w2);
        assert_eq!(dropometer.load(Ordering::SeqCst), 1);
        assert!(!a.is_in_use());
    }

    #[test]
    fn dyn_drop() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let dropometer = AtomicUsize::new(0);

        let rc1 = Rc::new(a.to_ref(), IncOnDrop { drop_counter: &dropometer }).unwrap();
        {
            let mut rc2: Rc<dyn core::fmt::Debug> = Rc::to_dyn(rc1);
            assert_eq!(Rc::strong_count(&rc2), 1);
            assert_eq!(Rc::weak_count(&rc2), 0);
            assert_eq!(dropometer.load(Ordering::SeqCst), 0);
            assert!(a.is_in_use());
            assert!(Rc::get_mut(&mut rc2).is_some());
        }

        assert_eq!(dropometer.load(Ordering::SeqCst), 1);
        assert!(!a.is_in_use());
    }

    #[test]
    fn as_ref() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let rc = Rc::new(a.to_ref(), 12345_u32).unwrap();
        assert_eq!(rc.as_ref(), &12345_u32);
    }
}

