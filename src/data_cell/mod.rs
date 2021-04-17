use core::fmt;
use core::fmt::Write as FmtWrite;
use core::ops::Deref;
use core::cell::RefCell;
use core::cell::BorrowError;
use core::cell::BorrowMutError;

use crate::ExecutionContext;
use crate::mm::HbAllocatorRef;
use crate::mm::HbAllocError;
use crate::mm::Rc;
use crate::mm::Vector;
use crate::io::IOError;
use crate::io::IOPartialError;
use crate::io::ErrorCode;
use crate::io::stream::Write;
use crate::num::fmt::MiniNumFmtPack;

pub mod expr;
pub mod eval;

/* Error ********************************************************************/
#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    NotApplicable,
    Alloc(HbAllocError),
    IO(IOError<'e>),
    Output(IOError<'e>), // used by report-generating functions like output_as_human_readable
    CellUnavailable, // borrow error on a RefCell while computing something
}

impl From<fmt::Error> for Error<'_> {
    fn from (_: fmt::Error) -> Self {
        Error::Output(
            IOError::with_str(
                ErrorCode::Unsuccessful,
                "error formatting output"))
    }
}

impl From<BorrowError> for Error<'_> {
    fn from(_: BorrowError) -> Self {
        Error::CellUnavailable
    }
}

impl From<BorrowMutError> for Error<'_> {
    fn from(_: BorrowMutError) -> Self {
        Error::CellUnavailable
    }
}

impl From<HbAllocError> for Error<'_> {
    fn from(e: HbAllocError) -> Self {
        Error::Alloc(e)
    }
}

impl<T> From<(HbAllocError, T)> for Error<'_> {
    fn from(e: (HbAllocError, T)) -> Self {
        Error::Alloc(e.0)
    }
}

impl<'a> From<IOPartialError<'a>> for Error<'a> {
    fn from(src: IOPartialError<'a>) -> Self {
        Error::IO(src.to_error())
    }
}

/* DataCellOpsMut ***********************************************************/
pub trait DataCellOpsMut: fmt::Debug {

    fn get_property_mut<'x>(
        &mut self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::NotApplicable)
    }

    fn output_as_human_readable_mut<'w, 'x>(
        &mut self,
        _out: &mut (dyn Write + 'w),
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        Err(Error::NotApplicable)
    }

}

/* DataCellOps **************************************************************/
pub trait DataCellOps: fmt::Debug {

    fn get_property<'x>(
        &self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::NotApplicable)
    }

    fn output_as_human_readable<'w, 'x>(
        &self,
        _out: &mut (dyn Write + 'w),
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        Err(Error::NotApplicable)
    }

}

impl<T> DataCellOps for RefCell<T>
where T: DataCellOpsMut {

    fn get_property<'x>(
        &self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        let mut c = self.try_borrow_mut()?;
        c.get_property_mut(property_name, xc)
    }

    fn output_as_human_readable<'w, 'x>(
        &self,
        out: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        let mut c = self.try_borrow_mut()?;
        c.output_as_human_readable_mut(out, xc)
    }

}

impl<'a, T> DataCellOps for Rc<'a, T>
where T: DataCellOps {

    fn get_property<'x>(
        &self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        let c = self.as_ref();
        c.get_property(property_name, xc)
    }

    fn output_as_human_readable<'w, 'x>(
        &self,
        out: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        let c = self.as_ref();
        c.output_as_human_readable(out, xc)
    }

}

/* U64Cell ******************************************************************/
#[derive(Debug)]
pub struct U64Cell {
    pub n: u64,
    pub fmt_pack: MiniNumFmtPack,
}

impl DataCellOps for U64Cell {

    fn output_as_human_readable<'w, 'x>(
        &self,
        w: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        let mut buf = [0_u8; 256];
        w.write_all(
            self.fmt_pack.int_fmt(self.n, &mut buf).unwrap().as_bytes(),
            xc
        ).map_err(|e| Error::Output(e.to_error()))
    }

}

/* ByteVector ***************************************************************/
#[derive(Debug)]
pub struct ByteVector<'a>(Vector<'a, u8>);

impl<'a> DataCellOpsMut for ByteVector<'a> {

    fn get_property_mut<'x>(
        &mut self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::NotApplicable)
    }

    fn output_as_human_readable_mut<'w, 'x>(
        &mut self,
        out: &mut (dyn Write + 'w),
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        write!(out, "b\"")?;
        for &b in self.0.as_slice() {
            if b == 0x22 || b == 0x5C {
                write!(out, "\\{}", b as char)?;
            } else if b >= 0x20_u8 && b <= 0x7E_u8 {
                write!(out, "{}", b as char)?;
            } else {
                write!(out, "\\x{:02X}", b)?;
            }
        }
        write!(out, "\"")?;
        Ok(())
    }

}

/* ByteVectorCell ***********************************************************/
pub type ByteVectorCell<'a> = Rc<'a, RefCell<ByteVector<'a>>>;
impl<'a> ByteVectorCell<'a> {

    pub fn from_bytes(
        allocator: HbAllocatorRef<'a>,
        data: &[u8]
    ) -> Result<Self, HbAllocError> {
        let bv = ByteVector(Vector::from_slice(allocator, data)?);
        Ok(Rc::new(allocator, RefCell::new(bv))?)
    }

}

/* DataCell *****************************************************************/
#[derive(Debug)]
pub enum DataCell<'d> {
    Nothing,
    U64(U64Cell),
    ByteVector(ByteVectorCell<'d>),
    StaticId(&'d str),
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

    fn output_as_human_readable<'w, 'x>(
        &self,
        w: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        match self {
            DataCell::Nothing => Ok(()),
            DataCell::U64(v) => v.output_as_human_readable(w, xc),
            DataCell::ByteVector(v) => v.output_as_human_readable(w, xc),
            DataCell::StaticId(s) => {
                w.write_all(s.as_bytes(), xc)
                    .map_err(|e| Error::Output(e.to_error()))
            },
            DataCell::Dyn(v) => v.deref().output_as_human_readable(w, xc),
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

