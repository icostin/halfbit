use crate::ExecutionContext;
use crate::mm::Vector;
use crate::dyn_box;
use crate::io::IOError;

pub mod expr;

#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    UnknownProperty,
    IO(IOError<'e>),
}

pub trait DataCellOps {
    fn get_property<'x, 'o>(
        &mut self,
        _property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'o>, Error<'x>>
    where Self: 'o, 'x: 'o {
        Err(Error::UnknownProperty)
    }
}

#[derive(Debug)]
pub enum DataCell<'d> {
    Nothing,
    Bytes(Vector<'d, u8>),
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

