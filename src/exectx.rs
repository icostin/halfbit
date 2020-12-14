//use crate::mm::Allocator;
use crate::mm::AllocatorRef;
use crate::mm::Box;
use crate::mm::AllocError;
use crate::io::stream::Stream;

/* ExecutionContext *********************************************************/
pub struct ExecutionContext<'a> {
    main_allocator: AllocatorRef<'a>,
    error_allocator: AllocatorRef<'a>,
    log_stream: &'a mut (dyn Stream + 'a),
    // TODO: some TLS-style storage
}

impl<'a> ExecutionContext<'a> {

    pub fn new(
        main_allocator: AllocatorRef<'a>,
        error_allocator: AllocatorRef<'a>,
        log_stream: &'a mut (dyn Stream + 'a),
    ) -> ExecutionContext<'a> {
        ExecutionContext { main_allocator, error_allocator, log_stream }
    }

    pub fn get_main_allocator(&self) -> AllocatorRef<'_> {
        self.main_allocator
    }

    pub fn get_error_allocator(&self) -> AllocatorRef<'_> {
        self.error_allocator
    }

    pub fn get_log_stream(&mut self) -> &mut (dyn Stream + '_) {
        self.log_stream
    }

    pub fn to_box<T: Sized>(
        &self,
        v: T
    ) -> Result<Box<'_, T>, (AllocError, T)> {
        self.get_main_allocator().alloc_item(v)
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
        assert!(xc.get_log_stream().provider_name().contains("null-stream"));
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


}
