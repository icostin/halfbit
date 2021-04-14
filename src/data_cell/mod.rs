use core::fmt;
use core::ops::Deref;

use crate::ExecutionContext;
use crate::mm::Rc;
use crate::io::IOError;
use crate::io::stream::Write;
use crate::num::fmt::MiniNumFmtPack;

pub mod expr;
pub mod eval;

#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    NotApplicable,
    IO(IOError<'e>),
    Output(IOError<'e>), // used by report-generating functions like to_human_readable
}

pub trait DataCellOps: fmt::Debug {

    fn get_property<'x>(
        &self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::NotApplicable)
    }

    fn to_human_readable<'w, 'x>(
        &self,
        _out: &mut (dyn Write + 'w),
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        Err(Error::NotApplicable)
    }

}

#[derive(Debug)]
pub enum DataCell<'d> {
    Nothing,
    U64(u64, MiniNumFmtPack),
    Id(&'d str),
    Dyn(Rc<'d, dyn DataCellOps + 'd>),
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

    fn to_human_readable<'w, 'x>(
        &self,
        w: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        match self {
            DataCell::Nothing => Ok(()),
            DataCell::U64(v, nf) => {
                let mut buf = [0_u8; 256];
                w.write_all(nf.int_fmt(*v, &mut buf).unwrap().as_bytes(), xc)
                    .map_err(|e| Error::IO(e.to_error()))
            },
            DataCell::Id(s) => {
                w.write_all(s.as_bytes(), xc)
                    .map_err(|e| Error::IO(e.to_error()))
            },
            DataCell::Dyn(v) => v.deref().to_human_readable(w, xc),
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
    fn default_get_property() {
        let mut xc = ExecutionContext::nop();
        assert_eq!(Error::NotApplicable, Abc().get_property("zilch", &mut xc).unwrap_err());
    }

}

