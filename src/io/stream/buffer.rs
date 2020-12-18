use super::Stream;
//use crate::exectx::ExecutionContext;

pub struct BufferAsROStream<'a> {
    buffer: &'a[u8],
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
impl Stream for BufferAsROStream<'_> {
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
    fn buf_ro_multiple_reads() {
        let mut f = BufferAsROStream::new(b"Hello world!");
        let mut buf = [0_u8; 7];
        let mut xc = ExecutionContext::nop();
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 7);
        assert_eq!(buf, *b"Hello w");
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 5);
        assert_eq!(buf[0..5], *b"orld!");
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

        assert_eq!(f.seek(SeekFrom::Start(11), &mut xc).unwrap(), 0);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);
    }

    #[test]
    fn buf_ro_seek_current() {
        let mut f = BufferAsROStream::new(b"0123456789");
        let mut buf = [0_u8; 1];
        let mut xc = ExecutionContext::nop();

        assert_eq!(f.seek(SeekFrom::Current(10), &mut xc).unwrap(), 10);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);

        assert_eq!(
            *f.seek(SeekFrom::Current(-11), &mut xc).unwrap_err().get_data(),
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

        assert_eq!(f.seek(SeekFrom::End(-17), &mut xc).unwrap(), 3);
        assert_eq!(f.read(&mut buf, &mut xc).unwrap(), 0);
        assert_eq!(buf, *b"3");
        assert_eq!(f.seek(SeekFrom::End(0), &mut xc).unwrap(), 10);
    }
}

