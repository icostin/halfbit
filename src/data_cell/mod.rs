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

    #[test]
    fn default_get_property() {
        let mut xc = ExecutionContext::nop();
        assert_eq!(Error::NotApplicable, DataCell::Nothing.get_property("zilch", &mut xc).unwrap_err());
    }

}

