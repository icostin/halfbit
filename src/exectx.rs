use crate::mm::AllocatorRef;
use crate::mm::Box;
use crate::mm::AllocError;
use crate::mm::Allocator;
use crate::mm::NOP_ALLOCATOR;
use crate::io::stream::Write;
use crate::io::stream::NULL_STREAM;

/* ExecutionContext *********************************************************/
pub struct ExecutionContext<'a> {
    main_allocator: AllocatorRef<'a>,
    error_allocator: AllocatorRef<'a>,
    log_stream: &'a mut (dyn Write + 'a),
    // TODO: some TLS-style storage
}

impl<'a> ExecutionContext<'a> {

    pub fn new(
        main_allocator: AllocatorRef<'a>,
        error_allocator: AllocatorRef<'a>,
        log_stream: &'a mut (dyn Write + 'a),
    ) -> ExecutionContext<'a> {
        ExecutionContext { main_allocator, error_allocator, log_stream }
    }

    pub fn nop() -> ExecutionContext<'a> {
        ExecutionContext {
            main_allocator: NOP_ALLOCATOR.to_ref(),
            error_allocator: NOP_ALLOCATOR.to_ref(),
            log_stream: NULL_STREAM.get()
        }
    }

    pub fn get_main_allocator(&self) -> AllocatorRef<'a> {
        self.main_allocator
    }

    pub fn get_error_allocator(&self) -> AllocatorRef<'a> {
        self.error_allocator
    }

    pub fn get_log_stream(&mut self) -> &mut (dyn Write + '_) {
        self.log_stream
    }

    pub fn to_box<T: Sized>(
        &self,
        v: T
    ) -> Result<Box<'_, T>, (AllocError, T)> {
        self.get_main_allocator().alloc_item(v)
    }
}

#[macro_export]
macro_rules! make_err {
    ( $xc:expr, $err_data:expr, $oom_msg:expr, $( $x:tt )+ ) => {
        {
            use core::fmt::Write;
            use crate::mm::String;
            use crate::error::Error;
            let mut msg = String::new($xc.get_error_allocator());
            if let Err(_) = write!(msg, $( $x )*) {
                msg = String::map_str($oom_msg);
            }
            Error::new($err_data, msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mm::BumpAllocator;
    use crate::mm::Allocator;
    use crate::io::NullStream;

    #[test]
    fn create_simple_exe_ctx() {
        let mut buf = [0_u8; 0x100];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log);
        assert!(xc.get_main_allocator().name().contains("bump"));
        assert!(xc.get_error_allocator().name().contains("bump"));
    }

    #[test]
    fn to_box_happy_case() {
        let mut buf = [0_u8; 0x100];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log);
        let b = xc.to_box(0x12345_u32).unwrap();
        assert_eq!(*b, 0x12345_u32);
    }

    #[test]
    fn to_box_fails() {
        let mut buf = [0_u8; 3];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log);
        let (e, v) = xc.to_box(0x12345_u32).unwrap_err();
        assert_eq!(e, AllocError::NotEnoughMemory);
        assert_eq!(v, 0x12345_u32);
    }

    #[test]
    fn make_err_on_nop_exectx() {
        let xc = ExecutionContext::nop();
        let e = make_err!(&xc, 123, "oom-error-text", "look:{}", 123);
        assert_eq!(*e.get_msg(), *"oom-error-text");
    }

}
