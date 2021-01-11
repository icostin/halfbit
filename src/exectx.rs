use crate::mm::AllocatorRef;
use crate::mm::Box;
use crate::mm::AllocError;
use crate::mm::Allocator;
use crate::mm::NOP_ALLOCATOR;
use crate::mm::String;
use crate::io::stream::Write;
use crate::io::stream::NULL_STREAM;

#[derive(Copy, Clone, PartialEq, PartialOrd, Debug)]
pub enum LogLevel {
    Critical,
    Error,
    Warning,
    Info,
    Debug,
}

/* ExecutionContext *********************************************************/
pub struct ExecutionContext<'a> {
    main_allocator: AllocatorRef<'a>,
    error_allocator: AllocatorRef<'a>,
    log_stream: &'a mut (dyn Write + 'a),
    log_level: LogLevel,
    logging_error_mask: u8,
    // TODO: some TLS-style storage
}

impl<'a> ExecutionContext<'a> {

    pub fn new(
        main_allocator: AllocatorRef<'a>,
        error_allocator: AllocatorRef<'a>,
        log_stream: &'a mut (dyn Write + 'a),
        log_level: LogLevel,
    ) -> ExecutionContext<'a> {
        ExecutionContext {
            main_allocator, error_allocator, log_stream, log_level,
            logging_error_mask: 0,
        }
    }

    pub fn nop() -> ExecutionContext<'a> {
        ExecutionContext {
            main_allocator: NOP_ALLOCATOR.to_ref(),
            error_allocator: NOP_ALLOCATOR.to_ref(),
            log_stream: NULL_STREAM.get(),
            log_level: LogLevel::Critical,
            logging_error_mask: 0,
        }
    }

    pub fn get_main_allocator(&self) -> AllocatorRef<'a> {
        self.main_allocator
    }

    pub fn get_error_allocator(&self) -> AllocatorRef<'a> {
        self.error_allocator
    }

    pub fn get_log_stream(&mut self) -> &mut (dyn Write + '_) {
        self.log_stream
    }

    pub fn get_log_level(&self) -> LogLevel {
        self.log_level
    }

    pub fn get_logging_error_mask(&self) -> u8 {
        self.logging_error_mask
    }

    pub fn set_logging_error(&mut self, log_level: LogLevel) {
        self.logging_error_mask |= 1_u8 << (log_level as u32);
    }

    pub fn to_box<T: Sized>(
        &self,
        v: T
    ) -> Result<Box<'_, T>, (AllocError, T)> {
        self.get_main_allocator().alloc_item(v)
    }

    pub fn string(&self) -> String<'a> {
        String::new(self.get_main_allocator())
    }
}

#[macro_export]
macro_rules! xc_err {
    ( $xc:expr, $err_data:expr, $oom_msg:expr, $( $x:tt )+ ) => {
        {
            use core::fmt::Write;
            use crate::mm::String;
            use crate::error::Error;
            let mut msg = String::new($xc.get_error_allocator());
            if let Err(_) = write!(msg, $( $x )*) {
                msg = String::map_str($oom_msg);
            }
            Error::new($err_data, msg)
        }
    }
}

#[macro_export]
macro_rules! log_msg {
    ( $xc: expr, $log_level: expr, $f:literal $( $x:tt )* ) => {
        {
            use core::fmt::Write;
            if $log_level <= $xc.get_log_level() && write!($xc.get_log_stream(), concat!($f, "\n") $( $x )*).is_err() {
                $xc.set_logging_error($log_level);
            }
        }
    }
}

#[macro_export]
macro_rules! log_crit {
    ( $xc: expr, $( $x:tt )+ ) => {
        {
            use $crate::LogLevel;
            use $crate::log_msg;
            log_msg!($xc, LogLevel::Critical, $( $x )*);
        }
    }
}

#[macro_export]
macro_rules! log_error {
    ( $xc: expr, $( $x:tt )+ ) => {
        {
            use $crate::LogLevel;
            use $crate::log_msg;
            log_msg!($xc, LogLevel::Error, $( $x )*);
        }
    }
}

#[macro_export]
macro_rules! log_warn {
    ( $xc: expr, $( $x:tt )+ ) => {
        {
            use $crate::LogLevel;
            use $crate::log_msg;
            log_msg!($xc, LogLevel::Warning, $( $x )*);
        }
    }
}

#[macro_export]
macro_rules! log_info {
    ( $xc: expr, $( $x:tt )+ ) => {
        {
            use $crate::LogLevel;
            use $crate::log_msg;
            log_msg!($xc, LogLevel::Info, $( $x )*);
        }
    }
}

#[macro_export]
macro_rules! log_debug {
    ( $xc: expr, $( $x:tt )+ ) => {
        {
            use $crate::LogLevel;
            use $crate::log_msg;
            if cfg!(debug_assertions) {
                log_msg!($xc, LogLevel::Debug, $( $x )*);
            }
        }
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
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log, LogLevel::Critical);
        assert!(xc.get_main_allocator().name().contains("bump"));
        assert!(xc.get_error_allocator().name().contains("bump"));
        let mut nop_xc = ExecutionContext::nop();
        assert_eq!(xc.get_log_stream().write(b"abc", &mut nop_xc).unwrap(), 3);
    }

    #[test]
    fn to_box_happy_case() {
        let mut buf = [0_u8; 0x100];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log, LogLevel::Critical);
        let b = xc.to_box(0x12345_u32).unwrap();
        assert_eq!(*b, 0x12345_u32);
    }

    #[test]
    fn to_box_fails() {
        let mut buf = [0_u8; 3];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log, LogLevel::Critical);
        let (e, v) = xc.to_box(0x12345_u32).unwrap_err();
        assert_eq!(e, AllocError::NotEnoughMemory);
        assert_eq!(v, 0x12345_u32);
    }

    #[test]
    fn make_err_on_nop_exectx() {
        let xc = ExecutionContext::nop();
        let e = xc_err!(&xc, 123, "oom-error-text", "look:{}", 123);
        assert_eq!(*e.get_msg(), *"oom-error-text");
    }

    #[test]
    fn log_crit_marks_logging_error_on_write_error() {
        use crate::io::stream::Zero;
        let mut log = Zero::new();
        let mut xc = ExecutionContext::new(
            NOP_ALLOCATOR.to_ref(),
            NOP_ALLOCATOR.to_ref(),
            &mut log,
            LogLevel::Critical,
        );
        log_crit!(xc, "bla bla bla");
        assert_eq!(xc.get_logging_error_mask(), 1);
    }

    #[test]
    fn log_crit() {
        use crate::io::stream::buffer::BufferAsRWStream;
        let mut log_buffer = [0xAA_u8; 0x100];
        let mut log = BufferAsRWStream::new(&mut log_buffer, 0);
        let mut xc = ExecutionContext::new(
            NOP_ALLOCATOR.to_ref(),
            NOP_ALLOCATOR.to_ref(),
            &mut log,
            LogLevel::Critical,
        );
        log_crit!(xc, "CRITICAL: this is not perl: {} != {:?}!!!", 123, "123");
        let expected = b"CRITICAL: this is not perl: 123 != \"123\"!!!\n\xAA";
        assert_eq!(xc.get_logging_error_mask(), 0);
        assert_eq!(log_buffer[..expected.len()], *expected);
    }

    #[test]
    fn obtain_string() {
        use core::fmt::Write;
        let mut buf = [0_u8; 0x100];
        let a = BumpAllocator::new(&mut buf);
        let mut log = NullStream::new();
        let xc = ExecutionContext::new(a.to_ref(), a.to_ref(), &mut log, LogLevel::Critical);
        let mut s = xc.string();
        write!(s, "this is the {} of the universe: {}", "meaning", 42).unwrap();
        assert_eq!(s.as_str(), "this is the meaning of the universe: 42");
    }


}
