use super::Stream;
use super::SeekFrom;
use crate::io::IOResult;
use crate::io::IOError;
use crate::io::ErrorCode;
use crate::ExecutionContext;

pub struct BufferAsOnePassROStream<'b> {
    buffer: &'b [u8],
}

impl<'b> BufferAsOnePassROStream<'b> {
    pub fn new(buffer: &'b [u8]) -> BufferAsOnePassROStream<'b> {
        BufferAsOnePassROStream { buffer }
    }
}
impl<'b> Stream for BufferAsOnePassROStream<'b> {
    fn supports_read(&self) -> bool { true }
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        let n = core::cmp::min(buf.len(), self.buffer.len());
        let (a, b) = self.buffer.split_at(n);
        buf[0..n].copy_from_slice(a);
        self.buffer = b;

        Ok(n)
    }
}

pub struct BufferAsROStream<'a> {
    buffer: &'a [u8],
    position: u64,
}

impl<'a> BufferAsROStream<'a> {
    pub fn new(buffer: &'a [u8]) -> BufferAsROStream<'a> {
        BufferAsROStream {
            buffer: buffer,
            position: 0
        }
    }
}

fn relative_position<'a>(
    pos: u64,
    disp: i64,
    _xc: &mut ExecutionContext<'a>
) -> IOResult<'a, u64> {
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

impl Stream for BufferAsROStream<'_> {
    fn supports_read(&self) -> bool { true }
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        _exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        if self.position >= self.buffer.len() as u64 {
            return Ok(0);
        }
        let pos = self.position as usize;
        let n = core::cmp::min(buf.len(), self.buffer.len() - pos);
        buf[0..n].copy_from_slice(&self.buffer[pos..pos + n]);
        self.position += n as u64;
        Ok(n)
    }

    fn supports_seek(&self) -> bool { true }
    fn seek<'a>(
        &mut self,
        target: SeekFrom,
        xc: &mut ExecutionContext<'a>
    ) -> IOResult<'a, u64> {
        match target {
            SeekFrom::Start(disp) => {
                self.position = disp;
            },
            SeekFrom::Current(disp) => {
                self.position = relative_position(self.position, disp, xc)?;
            },
            SeekFrom::End(disp) => {
                self.position = relative_position(
                    self.buffer.len() as u64, disp, xc)?;
            }
        }
        Ok(self.position)
    }

}

// pub struct BufferAsRWStream<'a> {
//     buffer: &'a mut [u8],
//     position: u64,
//     size: usize
// }
//
// impl Stream for BufferAsRWStream<'_> {
// }


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::SeekFrom;
    use crate::io::ErrorCode;
    use crate::ExecutionContext;

    #[test]
    fn rel_pos_larger_than_u64() {
        let mut xc = ExecutionContext::nop();
        let e = relative_position(u64::MAX, 1, &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedPosition);
    }

    #[test]
    fn buf_one_pass_ro_multiple_reads() {
        let mut f = BufferAsOnePassROStream::new(b"Hello world!");
        let mut buf = [0_u8; 7];
        let mut xc = ExecutionContext::nop();
        assert!(f.supports_read());
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 7);
        assert_eq!(buf, *b"Hello w");
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 5);
        assert_eq!(buf[0..5], *b"orld!");
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn buf_one_pass_ro_no_seek() {
        let mut f = BufferAsOnePassROStream::new(b"Hello world!");
        let mut xc = ExecutionContext::nop();
        assert!(!f.supports_seek());
        assert!(f.seek(SeekFrom::Start(0), &mut xc).is_err());
        assert!(f.seek(SeekFrom::Current(0), &mut xc).is_err());
        assert!(f.seek(SeekFrom::End(0), &mut xc).is_err());
    }

    #[test]
    fn buf_one_pass_ro_write_not_supported() {
        let mut f = BufferAsOnePassROStream::new(b"0123456789");
        let buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert!(!f.supports_write());
        let e = f.write(&buf, &mut xc).unwrap_err();

        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
    }

    #[test]
    fn buf_ro_multiple_reads() {
        let mut f = BufferAsROStream::new(b"Hello world!");
        let mut buf = [0_u8; 7];
        let mut xc = ExecutionContext::nop();
        assert!(f.supports_read());
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 7);
        assert_eq!(buf, *b"Hello w");
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 5);
        assert_eq!(buf[0..5], *b"orld!");
    }

    #[test]
    fn buf_ro_supports_seek() {
        let f = BufferAsROStream::new(b"Hello world!");
        assert!(f.supports_seek());
    }

    #[test]
    fn buf_ro_seek_start_inside() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let mut buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert_eq!(f.seek(SeekFrom::Start(8), &mut xc).unwrap(), 8);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 1);
        assert_eq!(buf, *b"8");

        assert_eq!(f.seek(SeekFrom::Start(0), &mut xc).unwrap(), 0);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 1);
        assert_eq!(buf, *b"0");
    }

    #[test]
    fn buf_ro_seek_start_outside() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let mut buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert_eq!(f.seek(SeekFrom::Start(10), &mut xc).unwrap(), 10);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);

        assert_eq!(f.seek(SeekFrom::Start(11), &mut xc).unwrap(), 11);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn buf_ro_seek_current() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let mut buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert_eq!(f.seek(SeekFrom::Current(10), &mut xc).unwrap(), 10);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);

        assert_eq!(*f.seek(SeekFrom::Current(-11), &mut xc)
                        .unwrap_err().get_data(),
                   ErrorCode::UnsupportedPosition);

        assert_eq!(f.seek(SeekFrom::Current(-5), &mut xc).unwrap(), 5);
        assert_eq!(f.seek(SeekFrom::Current(0), &mut xc).unwrap(), 5);
    }

    #[test]
    fn buf_ro_seek_end() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let mut buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert_eq!(f.seek(SeekFrom::End(10), &mut xc).unwrap(), 20);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);

        assert_eq!(f.seek(SeekFrom::End(-7), &mut xc).unwrap(), 3);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 1);
        assert_eq!(buf, *b"3");

        assert_eq!(f.seek(SeekFrom::End(0), &mut xc).unwrap(), 10);

        let e = f.seek(SeekFrom::End(-17), &mut xc).unwrap_err();
        assert_eq!(*e.get_data(), ErrorCode::UnsupportedPosition);
    }

    #[test]
    fn buf_ro_write_not_supported() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert!(!f.supports_write());
        let e = f.write(&buf, &mut xc).unwrap_err();

        assert_eq!(*e.get_data(), ErrorCode::UnsupportedOperation);
    }
}

