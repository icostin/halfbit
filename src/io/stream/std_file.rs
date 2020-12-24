extern crate std;
use core::fmt::Write as FmtWrite;
use std::io::Read;
use std::io::Write;
use std::io::Seek;
use std::io::ErrorKind as StdIOErrorKind;
use std::io::SeekFrom as StdIOSeekFrom;
use std::fs::File;

use super::Stream;
use super::SeekFrom;

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

impl Stream for File {
    fn read<'a>(
        &mut self,
        buf: &mut [u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Read::read(self, buf)
            .map_err(|e| convert_error(e, "read failed", exe_ctx))
    }
    fn write<'a>(
        &mut self,
        buf: &[u8],
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, usize> {
        Write::write(self, buf)
            .map_err(|e| convert_error(e, "write failed", exe_ctx))
    }
    fn seek<'a>(
        &mut self,
        target: SeekFrom,
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, u64> {
        Seek::seek(self, target.into())
            .map_err(|e| convert_error(e, "seek failed", exe_ctx))
    }
    fn truncate<'a>(
        &mut self,
        size: u64,
        exe_ctx: &mut ExecutionContext<'a>
    ) -> IOResult<'a, ()> {
        File::set_len(self, size)
            .map_err(|e| convert_error(e, "truncate failed", exe_ctx))
    }
    fn supports_read(&self) -> bool { true }
    fn supports_write(&self) -> bool { true }
    fn supports_seek(&self) -> bool { true }
    fn provider_name(&self) -> &'static str { "std-file" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::OpenOptions;
    use crate::io::stream::NULL_STREAM;
    use crate::mm::Allocator;
    use crate::mm::BumpAllocator;

    #[test]
    fn write_seek_read_on_temp_file() {
        let mut alloc_buffer = [0_u8; 0x400];
        let a = BumpAllocator::new(&mut alloc_buffer);
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get());

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
    }

}
