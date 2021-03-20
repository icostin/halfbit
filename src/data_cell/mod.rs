use core::fmt;
use core::ops::Deref;

use crate::ExecutionContext;
use crate::mm::Rc;
use crate::io::IOError;

pub mod expr;
pub mod eval;

#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    NotApplicable,
    IO(IOError<'e>),
}

pub trait DataCellOps: fmt::Debug + fmt::Display {
    fn get_property<'x>(
        &self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::NotApplicable)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NumFmt(u64);
impl NumFmt {
    pub fn default () -> NumFmt {
        NumFmt(0)
    }
}

#[derive(Debug)]
pub enum DataCell<'d> {
    Nothing,
    U64(u64, NumFmt),
    Id(&'d str),
    Dyn(Rc<'d, dyn DataCellOps + 'd>),
}

impl<'d> fmt::Display for DataCell<'d> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataCell::Nothing => Ok(()),
            DataCell::U64(v, _nf) => write!(f, "{}", v),
            DataCell::Id(s) => write!(f, "{}", s),
            DataCell::Dyn(v) => write!(f, "{}", v.deref())
        }
    }
}

impl<'d> DataCellOps for DataCell<'d> {
    fn get_property<'x>(
        &self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        match self {
            DataCell::Dyn(o) => o.get_property(property_name, xc),
            _ => Err(Error::NotApplicable)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct Abc();
    impl fmt::Display for Abc {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "<abc>")
        }
    }
    impl DataCellOps for Abc{}

    #[test]
    fn data_cell_nothing_get_property() {
        let mut xc = ExecutionContext::nop();
        assert_eq!(Error::NotApplicable, DataCell::Nothing.get_property("zilch", &mut xc).unwrap_err());
    }

    #[test]
    fn num_fmt_default() {
        let nf = NumFmt::default();
        assert_eq!(nf.0, 0);
    }

    #[test]
    fn default_get_property() {
        let mut xc = ExecutionContext::nop();
        assert_eq!(Error::NotApplicable, Abc().get_property("zilch", &mut xc).unwrap_err());
    }

    #[test]
    fn data_cell_display() {
        use crate::mm::BumpAllocator;
        use crate::mm::Allocator;
        use crate::io::stream::NULL_STREAM;
        use crate::exectx::ExecutionContext;
        use crate::exectx::LogLevel;
        use core::fmt::Write;
        let mut buf = [0_u8; 256];
        let a = BumpAllocator::new(&mut buf);
        let xc = ExecutionContext::new(
            a.to_ref(), a.to_ref(), NULL_STREAM.get(), LogLevel::Critical);
        {
            let mut s = xc.string();
            write!(s, "[{}]", DataCell::Nothing).unwrap();
            assert_eq!(s.as_str(), "[]");
        }
        {
            let mut s = xc.string();
            write!(s, "[{}]", DataCell::U64(12345, NumFmt::default())).unwrap();
            assert_eq!(s.as_str(), "[12345]");
        }
        {
            let mut s = xc.string();
            write!(s, "[{}]", DataCell::Id("abc")).unwrap();
            assert_eq!(s.as_str(), "[abc]");
        }
        {
            let mut s = xc.string();
            write!(s, "[{}]", 
                DataCell::Dyn(Rc::to_dyn(xc.rc(Abc()).unwrap())),
            ).unwrap();
            assert_eq!(s.as_str(), "[<abc>]");
        }
    }
}

