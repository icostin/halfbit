use crate::ExecutionContext;
//use crate::mm::Vector;
use crate::dyn_box;
use crate::io::IOError;

pub mod expr;

#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    UnknownProperty,
    IO(IOError<'e>),
}

pub trait DataCellOps {
    fn get_property<'x>(
        &mut self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        Err(Error::UnknownProperty)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct NumFmt(u64);

#[derive(Debug)]
pub enum DataCell<'d> {
    Nothing,
    U64(u64, NumFmt),
    Id(&'d str),
}

dyn_box!(pub DynDataCell, DataCellOps);

impl<'d> DataCellOps for DataCell<'d> {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_get_property() {
        let mut xc = ExecutionContext::nop();
        assert_eq!(Error::UnknownProperty, DataCell::Nothing.get_property("zilch", &mut xc).unwrap_err());
    }

}

