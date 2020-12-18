use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;
use core::cell::UnsafeCell;
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
    fn write_str<'a>(
        &mut self,
        data: &str,
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        self.write(data.as_bytes(), exe_ctx)
    }
    fn supports_read(&self) -> bool { false }
    fn supports_write(&self) -> bool { false }
    fn supports_seek(&self) -> bool { false }
    fn provider_name(&self) -> &'static str { "stream" }
}

impl FmtWrite for dyn Stream {
    fn write_str(&mut self, s: &str) -> FmtResult {
        let mut xc = ExecutionContext::nop();
        self.write(s.as_bytes(), &mut xc)?;
        Ok(())
    }
}

pub struct Null { }

impl Null {
    pub fn new() -> Null {
        Null { }
    }
}

pub struct NullWrapper {
    n: UnsafeCell<Null>
}

impl NullWrapper {
    pub fn get(&self) -> &mut Null {
        unsafe { &mut *(self.n.get() as *mut Null) }
    }
}
unsafe impl Sync for NullWrapper { }


pub static NULL_STREAM: NullWrapper = NullWrapper {
    n: UnsafeCell::new(Null{})
};

impl Stream for Null {
    fn provider_name(&self) -> &'static str { "null-stream" }
    fn supports_read(&self) -> bool { true }
    fn read<'a>(
        &mut self,
        _buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Ok(0)
    }
    fn supports_write(&self) -> bool { true }
    fn write<'a>(
        &mut self,
        buf: &[u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        Ok(buf.len())
    }
}

pub struct Zero { }
impl Zero {
    pub fn new() -> Zero { Zero { } }
}
impl Stream for Zero {
    fn supports_read(&self) -> bool { true }
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<usize> {
        for v in buf.iter_mut() {
            *v = 0;
        }
        Ok(buf.len())
    }
}

pub mod buffer;
//pub use buffer::BufferAsRWStream;
pub use buffer::BufferAsROStream;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exectx::ExecutionContext;
    use crate::mm::Allocator;
    use crate::mm::NOP_ALLOCATOR;
    use crate::io::ErrorCode;

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
        let buf = [0_u8; 7];
        assert!(n.supports_write());
        assert_eq!(n.write(&buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn null_provider_name() {
        let n = Null::new();
        assert!(n.provider_name().contains("null"));
    }

    #[test]
    fn null_wrapper_works() {
        let mut xc = ExecutionContext::nop();
        let buf = [0_u8; 5];
        let n = NULL_STREAM.get();
        {
            let nn = NULL_STREAM.get();
            assert_eq!(nn.write(&buf, &mut xc).unwrap(), buf.len());
        }
        assert_eq!(n.write(&buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn fmt_into_null_stream() {
        let mut n = Null::new();
        let nn: &mut dyn Stream = &mut n;
        write!(nn, "This is {:?}: {} = 0x{:04X}!", "so easy", 1234, 1234).unwrap();
    }

    #[test]
    fn zero_read_returns_zeroes() {
        let mut f = Zero::new();
        let mut buf = [1_u8; 5];
        let mut xc = ExecutionContext::nop();
        assert!(f.supports_read());
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn zero_write_not_supported() {
        let mut f = Zero::new();
        let buf = [1_u8; 5];
        let mut xc = ExecutionContext::nop();
        assert!(!f.supports_write());
        let e = f.write(&buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
    }
}
