use core::fmt::Write as FmtWrite;
use core::fmt::Result as FmtResult;
use core::cell::UnsafeCell;
use super::ErrorCode;
use super::IOError;
use super::IOPartialError;
use super::IOResult;
use super::IOPartialResult;
use crate::exectx::ExecutionContext;
use crate::xc_err;

pub enum SeekFrom {
    Start(u64),
    Current(i64),
    End(i64),
}

fn relative_position<'a>(
    pos: u64,
    disp: i64
) -> IOResult<'static, u64> {
    if disp < 0 {
        let udisp = -disp as u64;
        if udisp <= pos {
            Ok(pos - udisp)
        } else {
            Err(IOError::with_str(
                ErrorCode::UnsupportedPosition,
                "seek to negative position"))
        }
    } else if let Some(new_pos) = pos.checked_add(disp as u64) {
        Ok(new_pos)
    } else {
        Err(IOError::with_str(
            ErrorCode::UnsupportedPosition,
            "seek to position too large for u64"))
    }
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
    ) -> IOPartialResult<'a, usize> {
        let mut size_read = 0_usize;
        let mut buf = &mut buf[..];

        while buf.len() != 0 {
            match self.read(buf, exe_ctx) {
                Ok(n) => {
                    if n == 0 { break; }
                    size_read += n;
                    buf = &mut buf[n..];
                },
                Err(e) => match e.get_data() {
                    ErrorCode::Interrupted => {},
                    _ => { return Err(IOPartialError::from_error_and_size(e, size_read)); }
                }
            }
        }
        Ok(size_read)
    }

    fn read_exact<'a>(
        &mut self,
        buf: &mut [u8],
        exe_ctx: &mut ExecutionContext<'a>,
    ) -> IOPartialResult<'a, ()> {
        let size_read = self.read_uninterrupted(buf, exe_ctx)?;
        if size_read == buf.len() {
            Ok(())
        } else {
            Err(xc_err!(exe_ctx, (ErrorCode::UnexpectedEnd, size_read), "read_exact encountered EOF", "read_exact got {}/{} bytes", size_read, buf.len()))
        }
    }

    fn read_u8<'a>(
        &mut self,
        exe_ctx: &mut ExecutionContext<'a>,
    ) -> IOPartialResult<'a, u8> {
        let mut buf = [0_u8; 1];
        self.read_exact(&mut buf, exe_ctx)
        .map(|_| buf[0])
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
    fn write_all<'a>(
        &mut self,
        buf: &[u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOPartialResult<'a, ()> {
        let mut size_written = 0_usize;
        let mut buf = &buf[..];
        while buf.len() > 0 {
            match self.write(buf, exe_ctx) {
                Ok(n) => {
                    size_written += n;
                    buf = &buf[n..];
                },
                Err(e) => match e.get_data() {
                    ErrorCode::Interrupted => {},
                    _ => { return Err(IOPartialError::from_error_and_size(e, size_written)); }
                }
            }
        }
        Ok(())
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

pub trait RandomAccessRead: Read + Seek {
    fn seek_read<'a>(
        &mut self,
        pos: u64,
        buf: &mut [u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOPartialResult<'a, usize> {
        self.seek(SeekFrom::Start(pos), exe_ctx)?;
        self.read_uninterrupted(buf, exe_ctx)
    }
}
impl<T: Read + Seek> RandomAccessRead for T {}

pub trait Stream: RandomAccessRead + Write + Truncate {}
impl<T: RandomAccessRead + Write + Truncate> Stream for T {}

impl<'a> FmtWrite for dyn Write + 'a {
    fn write_str(&mut self, s: &str) -> FmtResult {
        let mut xc = ExecutionContext::nop();
        self.write_all(s.as_bytes(), &mut xc)?;
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
    use crate::io::ErrorCode;

    struct DefaultStream {}
    impl Read for DefaultStream {}
    impl Write for DefaultStream {}
    impl Seek for DefaultStream {}
    impl Truncate for DefaultStream {}

    #[test]
    fn default_read_returns_unsupported() {
        let mut xc = ExecutionContext::nop();
        let mut ds = DefaultStream { };
        let mut buf = [0_u8; 4];
        let e = ds.read(&mut buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("read not supported"));
    }

    #[test]
    fn default_write_returns_unsupported() {
        let mut xc = ExecutionContext::nop();
        let mut ds = DefaultStream { };
        let buf = [0_u8; 4];
        let e = ds.write(&buf, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("write not supported"));
    }

    #[test]
    fn default_seek_returns_unsupported() {
        let mut xc = ExecutionContext::nop();
        let mut ds = DefaultStream { };
        let e = ds.seek(SeekFrom::Start(123), &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("seek not supported"));
    }

    #[test]
    fn default_truncate_returns_unsupported() {
        let mut xc = ExecutionContext::nop();
        let mut ds = DefaultStream { };
        let e = ds.truncate(123, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
        assert!(e.get_msg().contains("truncate not supported"));
    }

    #[test]
    fn null_read_outputs_0_bytes() {
        let mut xc = ExecutionContext::nop();
        let mut n = Null::new();
        let mut buf = [0_u8; 4];
        assert_eq!(n.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn null_write_consumes_all_buffer() {
        let mut xc = ExecutionContext::nop();
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
        assert_eq!(stream.read_u8(&mut xc).unwrap(), 0x21);
    }

    #[test]
    fn read_byte_when_no_data_is_left() {
        let mut stream = BufferAsOnePassROStream::new(b"");
        let mut xc = ExecutionContext::nop();
        assert_eq!(*stream.read_u8(&mut xc).unwrap_err().get_data(),
            (ErrorCode::UnexpectedEnd, 0));

    }

    #[test]
    fn read_byte_when_read_returns_error() {
        let mut stream = DefaultStream { };
        let mut xc = ExecutionContext::nop();
        assert_eq!(*stream.read_u8(&mut xc).unwrap_err().get_data(),
            (ErrorCode::UnsupportedOperation, 0));
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
                _ => { let b = self.1;
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
        let mut xc = ExecutionContext::nop();
        let mut r = IntermittentReader(0x2030220, 0x10);
        let mut buf1 = [0_u8; 6];
        assert_eq!(r.read_uninterrupted(&mut buf1, &mut xc).unwrap(), 6);
        assert_eq!(r.0, 0x201);
        assert_eq!(r.1, 0x12);
        assert_eq!(buf1, *b"\x10\x10\x11\x11\x12\x12");

        let mut buf2 = [0_u8; 16];
        assert_eq!(r.read_uninterrupted(&mut buf2, &mut xc).unwrap(), 3);
        assert_eq!(buf2[0..3], *b"\x12\x13\x13");
    }

    #[test]
    fn read_uninterrupted_with_error() {
        let mut xc = ExecutionContext::nop();
        let mut r = IntermittentReader(0x2F3040, 0x10);

        let mut buf1 = [0_u8; 6];
        assert_eq!(r.read_uninterrupted(&mut buf1, &mut xc).unwrap(), 6);
        assert_eq!(r.0, 0x2F1);
        assert_eq!(r.1, 0x11);
        assert_eq!(buf1, *b"\x10\x10\x10\x10\x11\x11");

        let mut buf2 = [0_u8; 16];
        let e2 = r.read_uninterrupted(&mut buf2, &mut xc).unwrap_err();
        assert_eq!(e2.get_processed_size(), 1);
        assert_eq!(e2.get_error_code(), ErrorCode::Unsuccessful);
    }

    struct SeekReadTester {
        pos: u64,
        interrupt_next_read: bool,
        fail_start_pos: u64,
        end_pos: u64,
    }
    impl Read for SeekReadTester {
        fn read<'a>(
            &mut self,
            buf: &mut [u8],
            _exe_ctx: &mut ExecutionContext<'a>
        ) -> IOResult<'a, usize> {
            if buf.len() == 0 || self.pos >= self.end_pos {
                Ok(0)
            } else if self.pos >= self.fail_start_pos {
                Err(IOError::with_str(ErrorCode::Unsuccessful, "meh"))
            } else if self.interrupt_next_read {
                self.interrupt_next_read = false;
                Err(IOError::with_str(ErrorCode::Interrupted, "induced interruption"))
            } else {
                self.interrupt_next_read = true;
                buf[0] = ((self.pos & 15) + 0x41) as u8;
                self.pos += 1;
                Ok(1)
            }
        }
    }
    impl Seek for SeekReadTester {
        fn seek<'a>(
            &mut self,
            target: SeekFrom,
            _exe_ctx: &mut ExecutionContext<'a>
        ) -> IOResult<'a, u64> {
            self.pos = match target {
                SeekFrom::Start(pos) => pos,
                _ => { panic!("seek_read should only use Start"); }
            };
            Ok(self.pos)
        }
    }

    #[test]
    fn seek_read_ok() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: false,
            fail_start_pos: 16,
            end_pos: 32,
        };
        let mut buf = [0_u8; 5];
        let mut xc = ExecutionContext::nop();
        assert_eq!(f.seek_read(1, &mut buf, &mut xc).unwrap(), 5);
        assert_eq!(buf, *b"BCDEF");
    }

    #[test]
    fn seek_read_to_end_ok() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: false,
            fail_start_pos: 32,
            end_pos: 32,
        };
        let mut buf = [0_u8; 5];
        let mut xc = ExecutionContext::nop();
        assert_eq!(f.seek_read(28, &mut buf, &mut xc).unwrap(), 4);
        assert_eq!(buf, *b"MNOP\x00");
    }

    #[test]
    fn seek_read_partial() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: true,
            fail_start_pos: 16,
            end_pos: 32,
        };
        let mut buf = [0_u8; 5];
        let mut xc = ExecutionContext::nop();
        let e = f.seek_read(12, &mut buf, &mut xc).unwrap_err();
        assert_eq!(e.get_error_code(), ErrorCode::Unsuccessful);
        assert_eq!(e.get_processed_size(), 4);
        assert_eq!(buf, *b"MNOP\x00");
    }

    #[test]
    fn seek_read_fail() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: true,
            fail_start_pos: 16,
            end_pos: 32,
        };
        let mut buf = [0_u8; 5];
        let mut xc = ExecutionContext::nop();
        let e = f.seek_read(20, &mut buf, &mut xc).unwrap_err();
        assert_eq!(e.get_error_code(), ErrorCode::Unsuccessful);
        assert_eq!(e.get_processed_size(), 0);
        assert_eq!(buf, *b"\x00\x00\x00\x00\x00");
    }

    #[test]
    #[should_panic(expected = "should only use Start")]
    fn seek_read_tester_panics_on_seek_current() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: true,
            fail_start_pos: 16,
            end_pos: 32,
        };
        let mut xc = ExecutionContext::nop();
        match f.seek(SeekFrom::Current(0), &mut xc) { _ => () };
    }

    #[test]
    #[should_panic(expected = "should only use Start")]
    fn seek_read_tester_panics_on_seek_end() {
        let mut f = SeekReadTester {
            pos: 0,
            interrupt_next_read: true,
            fail_start_pos: 16,
            end_pos: 32,
        };
        let mut xc = ExecutionContext::nop();
        match f.seek(SeekFrom::End(0), &mut xc) { _ => () };
    }

    struct WriteAllTester {
        buffer: [u8; 10],
        size: usize,
        fail_offset: usize,
        interrupt_next_write: bool,
    }
    impl Write for WriteAllTester {
        fn write<'a>(
            &mut self,
            buf: &[u8],
            _exe_ctx: &mut ExecutionContext<'a>
        ) -> IOResult<'a, usize> {
            if buf.len() == 0 {
                Ok(0)
            } else if self.size >= self.buffer.len() {
                Err(IOError::with_str(ErrorCode::NoSpace, "no space"))
            } else if self.size == self.fail_offset {
                self.fail_offset = usize::MAX;
                Err(IOError::with_str(ErrorCode::Unsuccessful, "induced fail"))
            } else if self.interrupt_next_write {
                self.interrupt_next_write = false;
                Err(IOError::with_str(ErrorCode::Interrupted, "interrupted"))
            } else {
                self.interrupt_next_write = true;
                self.buffer[self.size] = buf[0];
                self.size += 1;
                Ok(1)
            }
        }
    }

    #[test]
    fn write_all_ok() {
        let mut f = WriteAllTester {
            buffer: [0_u8; 10],
            size: 0,
            fail_offset: 11,
            interrupt_next_write: true,
        };
        let mut xc = ExecutionContext::nop();
        assert_eq!(f.write(b"", &mut xc).unwrap(), 0);
        f.write_all(b"ABCDEF", &mut xc).unwrap();
        let e = f.write_all(b"abcde", &mut xc).unwrap_err();
        assert_eq!(e.get_processed_size(), 4);
        assert_eq!(e.get_error_code(), ErrorCode::NoSpace);
        assert_eq!(f.size, 10);
        assert_eq!(f.buffer, *b"ABCDEFabcd");
    }

    #[test]
    fn write_all_partial() {
        let mut f = WriteAllTester {
            buffer: [0_u8; 10],
            size: 0,
            fail_offset: 3,
            interrupt_next_write: true,
        };
        let mut xc = ExecutionContext::nop();
        let e = f.write_all(b"ABCDEF", &mut xc).unwrap_err();
        assert_eq!(e.get_processed_size(), 3);
        assert_eq!(e.get_error_code(), ErrorCode::Unsuccessful);

        f.write_all(b"abcde", &mut xc).unwrap();
        assert_eq!(f.size, 8);
        assert_eq!(f.buffer[0..8], *b"ABCabcde");

    }

}

