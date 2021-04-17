extern crate std;
use core::fmt::Write as FmtWrite;
use std::io::Read as StdRead;
use std::io::Write as StdWrite;
use std::io::Seek as StdSeek;
use std::io::ErrorKind as StdIOErrorKind;
use std::io::SeekFrom as StdIOSeekFrom;
use std::fs::File;

use super::Read;
use super::Write;
use super::Seek;
use super::SeekFrom;
use super::Truncate;

use crate::mm::AllocatorRef;
use crate::mm::String;
use crate::io::IOResult;
use crate::io::IOError;
use crate::io::ErrorCode;
use crate::ExecutionContext;

fn convert_error_with_allocator<'a>(
    e: std::io::Error,
    msg_pfx: &'static str,
    a: AllocatorRef<'a>,
) -> IOError<'a> {
    let ec: ErrorCode = match e.kind() {
        StdIOErrorKind::Interrupted => ErrorCode::Interrupted,
        _ => ErrorCode::Unsuccessful
    };
    let mut msg = String::new(a);
    write!(msg, "{}: {}", msg_pfx, e)
        .unwrap_or_else(|_| msg = String::map_str(msg_pfx));
    IOError::new(ec, msg)
}

fn convert_error<'a>(
    e: std::io::Error,
    msg_pfx: &'static str,
    exe_ctx: &mut ExecutionContext<'a>,
) -> IOError<'a> {
    convert_error_with_allocator(e, msg_pfx, exe_ctx.get_error_allocator())
}

impl From<SeekFrom> for std::io::SeekFrom {
    fn from(sf: SeekFrom) -> Self {
        match sf {
            SeekFrom::Start(x) => StdIOSeekFrom::Start(x),
            SeekFrom::Current(x) => StdIOSeekFrom::Current(x),
            SeekFrom::End(x) => StdIOSeekFrom::End(x),
        }
    }
}

impl<T: StdRead> Read for T {
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        StdRead::read(self, buf)
            .map_err(|e| convert_error(e, "read failed", exe_ctx))
    }
}

impl<T: StdWrite> Write for T {
    fn write<'a>(
        &mut self,
        buf: &[u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        StdWrite::write(self, buf)
            .map_err(|e| convert_error(e, "write failed", exe_ctx))
    }
}

impl<T: StdSeek> Seek for T {
    fn seek<'a>(
        &mut self,
        target: SeekFrom,
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, u64> {
        StdSeek::seek(self, target.into())
            .map_err(|e| convert_error(e, "seek failed", exe_ctx))
    }
}

impl Truncate for File {
    fn truncate<'a>(
        &mut self,
        size: u64,
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, ()> {
        File::set_len(self, size)
            .map_err(|e| convert_error(e, "truncate failed", exe_ctx))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::OpenOptions;
    use crate::io::stream::NULL_STREAM;
    use crate::io::stream::Stream;
    use crate::mm::HbAllocator;
    use crate::mm::BumpAllocator;
    use crate::LogLevel;

    #[test]
    fn write_seek_read_on_temp_file() {
        let mut alloc_buffer = [0_u8; 0x400];
        let a = BumpAllocator::new(&mut alloc_buffer);
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);

        let mut path = env::temp_dir();
        path.push("halfbit-std-test-file.dat");
        let mut f = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path).unwrap();
        let stream: &mut dyn Stream = &mut f;
        assert_eq!(stream.write(b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ", &mut xc).unwrap(), 36);
        assert_eq!(stream.seek(SeekFrom::Current(-26), &mut xc).unwrap(), 10);
        let mut data = [0_u8; 0x100];
        assert_eq!(stream.read(&mut data[0..15], &mut xc).unwrap(), 15);
        assert_eq!(data[0..15], *b"ABCDEFGHIJKLMNO");
        assert_eq!(stream.read(&mut data, &mut xc).unwrap(), 11);
        assert_eq!(data[0..12], *b"PQRSTUVWXYZL");
        assert!(stream.truncate(22, &mut xc).is_ok());
        assert_eq!(stream.seek(SeekFrom::Start(20), &mut xc).unwrap(), 20);
        assert_eq!(stream.read(&mut data, &mut xc).unwrap(), 2);
        assert_eq!(data[0..4], *b"KLRS");
        assert_eq!(stream.seek(SeekFrom::End(0), &mut xc).unwrap(), 22);
        let e = stream.seek(SeekFrom::Current(-30), &mut xc).unwrap_err();
        assert!(e.get_msg().contains("seek failed"));
        assert!(e.get_msg().contains("os error"));

        let mut xc = ExecutionContext::nop();
        let e = stream.seek(SeekFrom::Current(-30), &mut xc).unwrap_err();
        assert!(e.get_msg().contains("seek failed"));
    }

}
