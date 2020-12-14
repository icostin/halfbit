//use crate::mm::Allocator;
use crate::mm::AllocatorRef;
//use crate::mm::AllocError;
use crate::io::stream::Stream;

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
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log);
        assert!(xc.main_allocator.name().contains("bump"));
        assert!(xc.error_allocator.name().contains("bump"));
        assert!(xc.log_stream.provider_name().contains("null-stream"));
    }
}
