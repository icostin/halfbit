use super::IOResult;
use super::IOError;
use super::ErrorCode;
use crate::exectx::ExecutionContext;

pub enum SeekFrom {
    Start(u64),
    Current(i64),
    End(i64),
}

pub trait Stream {
    fn read<'a>(
        &mut self,
        _buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Err(IOError::with_str(ErrorCode::UnsupportedOperation, "read not supported"))
    }
    fn write<'a>(
        &mut self,
        _buf: &[u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Err(IOError::with_str(ErrorCode::UnsupportedOperation, "write not supported"))
    }
    fn seek<'a>(
        &mut self,
        _target: SeekFrom,
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<u64> {
        Err(IOError::with_str(ErrorCode::UnsupportedOperation, "seek not supported"))
    }
    fn truncate<'a>(
        &mut self,
        _size: u64,
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<()> {
        Err(IOError::with_str(ErrorCode::UnsupportedOperation, "truncate not supported"))
    }
    fn supports_read(&self) -> bool { false }
    fn supports_write(&self) -> bool { false }
    fn supports_seek(&self) -> bool { false }
    fn provider_name(&self) -> &'static str { "stream" }
}

pub struct Null { }

impl Null {
    pub fn new() -> Null {
        Null { }
    }
}

impl Stream for Null {
    fn read<'a>(
        &mut self,
        _buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Ok(0)
    }
    fn write<'a>(
        &mut self,
        buf: &[u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Ok(buf.len())
    }
    fn supports_read(&self) -> bool { true }
    fn supports_write(&self) -> bool { true }
    fn provider_name(&self) -> &'static str { "null-stream" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exectx::ExecutionContext;
    use crate::mm::Allocator;
    use crate::mm::NOP_ALLOCATOR;
    use crate::io::ErrorCode;
    use crate::io::IOError;

    struct DefaultStream { }
    impl Stream for DefaultStream { }

    #[test]
    fn default_read_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
        let mut buf = [0_u8; 4];
        assert!(!ds.supports_read());
        let e = ds.read(&mut buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("read not supported"));
    }

    #[test]
    fn default_write_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
        let buf = [0_u8; 4];
        assert!(!ds.supports_write());
        let e = ds.write(&buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("write not supported"));
    }

    #[test]
    fn default_seek_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
        assert!(!ds.supports_seek());
        let e = ds.seek(SeekFrom::Start(123), &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("seek not supported"));
    }

    #[test]
    fn default_truncate_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
        let e = ds.truncate(123, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("truncate not supported"));
    }

    #[test]
    fn default_stream_provider_name() {
        let ds = DefaultStream { };
        assert!(ds.provider_name().contains("stream"));
    }

    #[test]
    fn null_read_outputs_0_bytes() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);

        let mut n = Null::new();
        let mut buf = [0_u8; 4];
        assert!(n.supports_read());
        assert_eq!(n.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn null_write_consumes_all_buffer() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);

        let mut n = Null::new();
        let mut buf = [0_u8; 7];
        assert!(n.supports_write());
        assert_eq!(n.write(&buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn null_provider_name() {
        let n = Null::new();
        assert!(n.provider_name().contains("null"));
    }
}
