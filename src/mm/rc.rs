use core::cell::UnsafeCell;
use core::ops::Deref;
use core::ptr::NonNull;
use core::borrow::Borrow;
use core::fmt;
use core::ptr;
use core::mem;
use core::cmp::max;
use core::num::NonZeroUsize;

#[cfg(feature = "nightly")]
use core::marker::Unsize;

use crate::num::Pow2Usize;

use super::Allocator;
use super::AllocatorRef;
use super::AllocError;

pub struct RcPayload<T: ?Sized>(UnsafeCell<T>);

struct RcCtlBlock<'a> {
    strong: usize,
    weak: usize,
    allocator: AllocatorRef<'a>,
}

pub struct Rc<'a, T>
where T: ?Sized {
    data: &'a RcPayload<T>,
}

pub struct RcWeak<'a, T>
where T: ?Sized {
    data: &'a RcPayload<T>,
}

fn rc_alignment(payload_align: usize) -> Pow2Usize {
    Pow2Usize::new(max(mem::align_of::<RcCtlBlock<'_>>(), payload_align)).unwrap()
}
fn rc_align_of<T: Sized>() -> Pow2Usize {
    rc_alignment(mem::align_of::<RcPayload<T>>())
}

fn rc_align_of_val<T: ?Sized>(payload: &RcPayload<T>) -> Pow2Usize {
    rc_alignment(mem::align_of_val(payload))
}

fn rc_ctl_alloc_size(align: Pow2Usize) -> usize {
    align.align_up(mem::size_of::<RcCtlBlock<'_>>()).unwrap()
}

unsafe fn rc_ctl_block<'a, T:?Sized>(payload: &'a RcPayload<T>) -> &mut RcCtlBlock<'a> {
    let uptr = payload as *const RcPayload<T> as *const u8 as usize;
    let uptr = uptr - mem::size_of::<RcCtlBlock<'_>>();
    &mut *(uptr as *mut RcCtlBlock<'a>)
}

unsafe fn free_if_unreferenced<T: ?Sized>(payload: &RcPayload<T>) {

    let ctl = rc_ctl_block(payload);
    if ctl.strong == 0 && ctl.weak == 0 {
        let align = rc_align_of_val(payload);
        let payload_ptr = payload.0.get();
        let ctl_alloc_size = rc_ctl_alloc_size(align);
        let uptr = payload_ptr as *const u8 as usize - ctl_alloc_size;
        let size = NonZeroUsize::new(mem::size_of_val(payload) + ctl_alloc_size).unwrap();
        ctl.allocator.free(NonNull::new(uptr as *mut u8).unwrap(), size, align);
    }
}

impl<'a, T> Rc<'a, T>
where T: Sized {

    pub fn new(
        allocator: AllocatorRef<'a>,
        value: T,
    ) -> Result<Self, (AllocError, T)> {

        let align = rc_align_of::<T>();
        let ctl_alloc_size = rc_ctl_alloc_size(align);
        let size = NonZeroUsize::new(ctl_alloc_size + mem::size_of::<RcPayload<T>>()).unwrap();
        match unsafe { allocator.alloc(size, align) } {
            Ok(ptr) => {
                let uptr = (ptr.as_ptr() as usize) + ctl_alloc_size;
                let data_ptr = uptr as *mut RcPayload<T>;
                let uptr = uptr - mem::size_of::<RcCtlBlock<'a>>();
                let ctl_ptr = uptr as *mut RcCtlBlock<'a>;
                unsafe {
                    ptr::write(data_ptr, RcPayload(UnsafeCell::new(value)));
                    ptr::write(ctl_ptr, RcCtlBlock { strong: 1, weak: 0, allocator: allocator });
                    Ok(Rc { data: &*data_ptr })
                }
            },
            Err(e) => Err((e, value))
        }
    }

}

impl<T> Rc<'_, T>
where T: ?Sized {

    pub fn strong_count(rc: &Rc<'_, T>) -> usize {
        let rc_block = unsafe { rc_ctl_block(rc.data) };
        rc_block.strong
    }

    pub fn weak_count(rc: &Rc<'_, T>) -> usize {
        let rc_block = unsafe { rc_ctl_block(rc.data) };
        rc_block.weak
    }

    pub fn get_mut<'a>(rc: &'a mut Rc<'_, T>) -> Option<&'a mut T> {
        let rc_block = unsafe { rc_ctl_block(rc.data) };
        if rc_block.strong == 1 && rc_block.weak == 0 {
            Some(unsafe { &mut *rc.data.0.get() })
        } else {
            None
        }
    }

    pub fn ptr_eq<'a, 'b>(a: &Rc<'a, T>, b: &Rc<'b, T>) -> bool {
        NonNull::new(a.data as *const RcPayload<T> as *mut RcPayload<T>) ==
        NonNull::new(b.data as *const RcPayload<T> as *mut RcPayload<T>)
    }

    #[cfg(feature = "nightly")]
    pub fn to_dyn<'a, U>(rc: Rc<'a, T>) -> Rc<'a, U>
    where
        T: Unsize<U>,
        U: ?Sized
    {
        let data = rc.data;
        mem::forget(rc);
        Rc { data }
    }

    pub unsafe fn to_payload<'a>(rc: Rc<'a, T>) -> &'a RcPayload<T> {
        let data = rc.data;
        mem::forget(rc);
        data
    }
    pub unsafe fn from_payload<'a>(payload: &'a RcPayload<T>) -> Rc<'a, T> {
        Rc { data: payload }
    }

    pub fn downgrade<'a>(rc: &Rc<'a, T>) -> RcWeak<'a, T> {
        let rc_block = unsafe { rc_ctl_block(rc.data) };
        rc_block.weak += 1;
        RcWeak { data: rc.data }
    }

}

impl<'a, T> AsRef<T> for Rc<'a, T> where T: ?Sized {

    fn as_ref(&self) -> &T {
        unsafe { &*self.data.0.get() }
    }

}

impl<'a, T> Borrow<T> for Rc<'a, T> where T: ?Sized {

    fn borrow(&self) -> &T {
        self.as_ref()
    }

}

impl<'a, T> Clone for Rc<'a, T> where T: ?Sized {

    fn clone(&self) -> Rc<'a, T> {
        let rc_block = unsafe { rc_ctl_block(self.data) };
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
        self.as_ref()
    }

}

impl<'a, T> Drop for Rc<'a, T> where T: ?Sized {

    fn drop(&mut self) {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        assert!(rc_block.strong > 0);
        rc_block.strong -= 1;
        if rc_block.strong == 0 {
            unsafe {
                ptr::drop_in_place(self.data.0.get());
                free_if_unreferenced(self.data);
            }
        }
    }

}

impl<'a, T> RcWeak<'a, T> where T: ?Sized {

    pub fn upgrade(&self) -> Option<Rc<'a, T>> {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        if rc_block.strong != 0 {
            rc_block.strong += 1;
            Some(Rc { data: self.data })
        } else {
            None
        }
    }

    pub fn strong_count(&self) -> usize {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        rc_block.strong
    }

    pub fn weak_count(&self) -> usize {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        rc_block.weak
    }

}

impl<'a, T> Clone for RcWeak<'a, T> where T: ?Sized {

    fn clone(&self) -> RcWeak<'a, T> {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        rc_block.weak += 1;
        RcWeak { data: self.data }
    }

}

impl<'a, T> Drop for RcWeak<'a, T> where T: ?Sized {

    fn drop(&mut self) {
        let rc_block = unsafe { rc_ctl_block(self.data) };
        assert!(rc_block.weak > 0);
        rc_block.weak -= 1;
        unsafe { free_if_unreferenced(self.data); }
    }

}

#[cfg(not(feature = "nightly"))]
#[macro_export]
macro_rules! dyn_rc {
    ( $func_name:ident, $trait:path ) => {
        fn $func_name<'a, T: $trait>(rc: $crate::mm::Rc<'a, T>) -> $crate::mm::Rc<'a, dyn $trait + 'a> {
            unsafe { 
                let data = $crate::mm::Rc::to_payload(rc);
                $crate::mm::Rc::from_payload(data)
            }
        }
    }
}

#[cfg(feature = "nightly")]
#[macro_export]
macro_rules! dyn_rc {
    ( $func_name:ident, $trait:path ) => {
        fn $func_name<'a, T: $trait>(rc: $crate::mm::Rc<'a, T>) -> $crate::mm::Rc<'a, dyn $trait + 'a> {
            $crate::mm::Rc::to_dyn(rc)
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::SingleAlloc;
    use crate::mm::BumpAllocator;
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

    dyn_rc!(make_fmt_debug_rc, fmt::Debug);

    #[test]
    fn dyn_drop() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let dropometer = AtomicUsize::new(0);

        let rc1 = Rc::new(a.to_ref(), IncOnDrop { drop_counter: &dropometer }).unwrap();
        {
            let mut rc2: Rc<dyn fmt::Debug> = make_fmt_debug_rc(rc1);
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

    #[test]
    fn borrow() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let rc = Rc::new(a.to_ref(), 12345_u32).unwrap();
        let b: &u32 = rc.borrow();
        assert_eq!(b, &12345_u32);
    }

    #[test]
    fn debug_fmt() {
        let mut buffer = [0u8; 128];
        let a = BumpAllocator::new(&mut buffer);

        let mut rc1 = Rc::new(a.to_ref(), 123_u32).unwrap();
        assert_eq!(Rc::strong_count(&rc1), 1);
        assert_eq!(Rc::weak_count(&rc1), 0);
        assert!(Rc::get_mut(&mut rc1).is_some());

        let w1 = Rc::downgrade(&rc1);
        assert_eq!(Rc::strong_count(&rc1), 1);
        assert_eq!(Rc::weak_count(&rc1), 1);
        assert!(Rc::get_mut(&mut rc1).is_none());

        let _w2 = w1.clone();

        extern crate std;
        use fmt::Write;

        let mut s = std::string::String::new();
        write!(s, "{:?}", rc1).unwrap();
        assert_eq!(s, "Rc[1+2]{123}");
    }

    #[test]
    fn deref() {
        let mut buffer = [0u8; 64];
        let a = SingleAlloc::new(&mut buffer);
        let rc = Rc::new(a.to_ref(), 12345_u32).unwrap();
        let b: &u32 = rc.deref();
        assert_eq!(b, &12345_u32);
    }

    #[test]
    fn unsafe_cell_of_t_to_unsafe_cell_of_trait() {
        let _x: &UnsafeCell<dyn fmt::Debug> = &UnsafeCell::new(0_u32);
    }

    #[test]
    fn rc_payload_of_t_to_rc_payload_of_trait() {
        let _x: &RcPayload<dyn fmt::Debug> = &RcPayload(UnsafeCell::new(0_u32));
    }

    #[repr(align(64))]
    struct Align64(u32);

    #[test]
    fn align_of_align64() {
        assert_eq!(mem::align_of::<Align64>(), 64);
    }

}

