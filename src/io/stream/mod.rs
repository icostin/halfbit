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

pub trait Read {
    fn read<'a>(
        &mut self,
        _buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Err(IOError::with_str(
                ErrorCode::UnsupportedOperation, "read not supported"))
    }
    fn read_uninterrupted<'a>(
        &mut self,
        buf: &mut [u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> (usize, IOResult<'a, ()>) {
        let mut size_read = 0_usize;
        let mut buf = &mut buf[..];

        while buf.len() != 0 {
            match self.read(buf, exe_ctx) {
                Ok(n) => {
                    if n == 0 {
                        break;
                    }
                    size_read += n;
                    buf = &mut buf[n..];
                },
                Err(e) => {
                    match e.get_data() {
                        ErrorCode::Interrupted => {
                            continue;
                        },
                        _ => {
                            return (size_read, Err(e));
                        }
                    }
                }
            }
        }
        (size_read, Ok(()))
    }
    fn read_byte<'a>(
        &mut self,
        exe_ctx: &mut ExecutionContext<'a>,
    ) -> IOResult<'a, u8> {
        let mut buf = [0_u8; 1];
        self.read(&mut buf, exe_ctx)
        .and_then(|size_read|
            if size_read != 0 {
                Ok(buf[0])
            } else {
                Err(IOError::with_str(
                    ErrorCode::UnexpectedEnd, "read byte after EOF"))
            })
     }
}

pub trait Write {
    fn write<'a>(
        &mut self,
        _buf: &[u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Err(IOError::with_str(
                ErrorCode::UnsupportedOperation, "write not supported"))
    }
}

pub trait Seek {
    fn seek<'a>(
        &mut self,
        _target: SeekFrom,
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, u64> {
        Err(IOError::with_str(
                ErrorCode::UnsupportedOperation, "seek not supported"))
    }
}

pub trait Truncate {
    fn truncate<'a>(
        &mut self,
        _size: u64,
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, ()> {
        Err(IOError::with_str(
                ErrorCode::UnsupportedOperation, "truncate not supported"))
    }
}

pub trait RandomAccessRead: Read + Seek {}
impl<T: Read + Seek> RandomAccessRead for T {}

pub trait Stream: RandomAccessRead + Write + Truncate {}
impl<T: RandomAccessRead + Write + Truncate> Stream for T {}

impl<'a> FmtWrite for dyn Write + 'a {
    fn write_str(&mut self, s: &str) -> FmtResult {
        let mut xc = ExecutionContext::nop();
        self.write(s.as_bytes(), &mut xc)?;
        Ok(())
    }
}

pub struct Null { }

impl Null {
    pub fn new() -> Null {
        Null {}
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

impl Read for Null {
    fn read<'a>(
        &mut self,
        _buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Ok(0)
    }
}
impl Write for Null {
    fn write<'a>(
        &mut self,
        buf: &[u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Ok(buf.len())
    }
}

impl Seek for Null {}
impl Truncate for Null {}

pub struct Zero {}
impl Zero {
    pub fn new() -> Zero { Zero {} }
}
impl Read for Zero {
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        for v in buf.iter_mut() {
            *v = 0;
        }
        Ok(buf.len())
    }
}
impl Write for Zero {}
impl Seek for Zero {}
impl Truncate for Zero {}

pub mod buffer;
pub use buffer::BufferAsRWStream;
pub use buffer::BufferAsROStream;
pub use buffer::BufferAsOnePassROStream;

#[cfg(feature = "use-std")]
pub mod std_file;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exectx::ExecutionContext;
    use crate::mm::Allocator;
    use crate::mm::NOP_ALLOCATOR;
    use crate::io::ErrorCode;

    struct DefaultStream {}
    impl Read for DefaultStream {}
    impl Write for DefaultStream {}
    impl Seek for DefaultStream {}
    impl Truncate for DefaultStream {}

    #[test]
    fn default_read_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
        let mut buf = [0_u8; 4];
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
        let e = ds.write(&buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("write not supported"));
    }

    #[test]
    fn default_seek_returns_unsupported() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);
        let mut ds = DefaultStream { };
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
    fn null_read_outputs_0_bytes() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);

        let mut n = Null::new();
        let mut buf = [0_u8; 4];
        assert_eq!(n.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn null_write_consumes_all_buffer() {
        let mut log = Null::new();
        let mut xc = ExecutionContext::new(NOP_ALLOCATOR.to_ref(), NOP_ALLOCATOR.to_ref(), &mut log);

        let mut n = Null::new();
        let buf = [0_u8; 7];
        assert_eq!(n.write(&buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn null_write_str_consumes_all_buffer() {
        let mut n = Null::new();
        let nw: &mut dyn Write = &mut n;
        nw.write_str("abc").unwrap();
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
        let nn: &mut dyn Write = &mut n;
        write!(nn, "This is {:?}: {} = 0x{:04X}!", "so easy", 1234, 1234).unwrap();
    }

    #[test]
    fn zero_read_returns_zeroes() {
        let mut f = Zero::new();
        let mut buf = [1_u8; 5];
        let mut xc = ExecutionContext::nop();
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), buf.len());
    }

    #[test]
    fn zero_write_not_supported() {
        let mut f = Zero::new();
        let buf = [1_u8; 5];
        let mut xc = ExecutionContext::nop();
        let e = f.write(&buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
    }

    #[test]
    fn read_byte_when_read_has_1_byte() {
        let mut stream = BufferAsOnePassROStream::new(b"!");
        let mut xc = ExecutionContext::nop();
        assert_eq!(stream.read_byte(&mut xc).unwrap(), 0x21);
    }

    #[test]
    fn read_byte_when_no_data_is_left() {
        let mut stream = BufferAsOnePassROStream::new(b"");
        let mut xc = ExecutionContext::nop();
        assert_eq!(*stream.read_byte(&mut xc).unwrap_err().get_data(),
            ErrorCode::UnexpectedEnd);

    }

    #[test]
    fn read_byte_when_read_returns_error() {
        let mut stream = DefaultStream { };
        let mut xc = ExecutionContext::nop();
        assert_eq!(*stream.read_byte(&mut xc).unwrap_err().get_data(),
            ErrorCode::UnsupportedOperation);
    }

    struct IntermittentReader(u64, u8);
    impl Read for IntermittentReader {
        fn read<'a>(
            &mut self,
            buf: &mut [u8],
            _exe_ctx: &mut ExecutionContext<'a>
        ) -> IOResult<'a, usize> {
            let mut cmd = (self.0 & 15) as usize;
            self.0 >>= 4;
            match cmd {
                0 => if self.0 != 0 {
                    Err(IOError::with_str(ErrorCode::Interrupted, "interrupted"))
                } else {
                    Ok(0)
                }
                15 => Err(IOError::with_str(ErrorCode::Unsuccessful, "meh")),
                _ => {
                    let b = self.1;
                    if cmd > buf.len() {
                        self.0 = (self.0 << 4) | (cmd - buf.len()) as u64;
                        cmd = buf.len();
                    } else {
                        self.1 += 1;
                    }
                    for v in buf[0..cmd].iter_mut() {
                        *v = b;
                    }
                    Ok(cmd)
                }
            }
        }
    }
    #[test]
    fn read_uninterrupted_ok() {
        let mut r = IntermittentReader(0x203040, 0x10);
        let mut buf1 = [0_u8; 6];
        let mut xc = ExecutionContext::nop();
        let (n1, r1) = r.read_uninterrupted(&mut buf1, &mut xc);
        assert_eq!(n1, 6);
        assert_eq!(r.0, 0x201);
        assert_eq!(r.1, 0x11);
        r1.unwrap();
        assert_eq!(buf1, *b"\x10\x10\x10\x10\x11\x11");
        let mut buf2 = [0_u8; 16];
        let (n2, r2) = r.read_uninterrupted(&mut buf2, &mut xc);
        assert_eq!(n2, 3);
        assert_eq!(buf2[0..3], *b"\x11\x12\x12");
        r2.unwrap();
    }
}
