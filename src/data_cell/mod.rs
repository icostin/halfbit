use core::fmt;
use core::fmt::Write as FmtWrite;
use core::ops::Deref;
use core::cell::RefCell;
use core::cell::BorrowError;
use core::cell::BorrowMutError;
use core::convert::TryInto;

use crate::ExecutionContext;
use crate::mm::AllocatorRef;
use crate::mm::AllocError;
use crate::mm::Rc;
use crate::mm::Vector;
use crate::io::IOError;
use crate::io::IOPartialError;
use crate::io::ErrorCode;
use crate::io::stream::Write;
use crate::io::stream::SeekFrom;
use crate::io::stream::Stream;
use crate::num::fmt::MiniNumFmtPack;

pub mod expr;
pub mod eval;
pub mod content_stream;

/* Error ********************************************************************/
#[derive(Debug, PartialEq)]
pub enum Error<'e> {
    NotApplicable,
    Alloc(AllocError),
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

impl From<AllocError> for Error<'_> {
    fn from(e: AllocError) -> Self {
        Error::Alloc(e)
    }
}

impl<T> From<(AllocError, T)> for Error<'_> {
    fn from(e: (AllocError, T)) -> Self {
        Error::Alloc(e.0)
    }
}

impl<'a> From<IOPartialError<'a>> for Error<'a> {
    fn from(src: IOPartialError<'a>) -> Self {
        Error::IO(src.to_error())
    }
}

impl<'a> From<IOError<'a>> for Error<'a> {
    fn from(src: IOError<'a>) -> Self {
        Error::IO(src)
    }
}

pub fn output_byte_slice_as_human_readable_text<'w, 'x>(
    data: &[u8],
    out: &mut (dyn Write + 'w),
    _xc: &mut ExecutionContext<'x>
) -> Result<(), Error<'x>> {
    for &b in data {
        if b == 0x22 || b == 0x5C {
            write!(out, "\\{}", b as char)?;
        } else if b >= 0x20_u8 && b <= 0x7E_u8 {
            write!(out, "{}", b as char)?;
        } else {
            write!(out, "\\x{:02X}", b)?;
        }
    }
    Ok(())
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

impl U64Cell {

    pub fn new(n: u64) -> Self {
        let fmt_pack = MiniNumFmtPack::default();
        U64Cell { n, fmt_pack }
    }
    pub fn with_fmt(n: u64, fmt_pack: MiniNumFmtPack) -> Self {
        U64Cell { n, fmt_pack }
    }
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
pub struct ByteVector<'a>(pub Vector<'a, u8>);

impl<'a> DataCellOpsMut for ByteVector<'a> {

    fn get_property_mut<'x>(
        &mut self,
        property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        match property_name {
            "len" | "length" | "count" | "size" => {
                let v = self.0.len().try_into().unwrap();
                Ok(DataCell::U64(U64Cell::new(v)))
            },
            _ => Err(Error::NotApplicable)
        }
    }

    fn output_as_human_readable_mut<'w, 'x>(
        &mut self,
        out: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        write!(out, "b\"")?;
        output_byte_slice_as_human_readable_text(self.0.as_slice(), out, xc)?;
        write!(out, "\"")?;
        Ok(())
    }

}

/* ByteVectorCell ***********************************************************/
pub type ByteVectorCell<'a> = Rc<'a, RefCell<ByteVector<'a>>>;
impl<'a> ByteVectorCell<'a> {

    pub fn from_bytes(
        allocator: AllocatorRef<'a>,
        data: &[u8]
    ) -> Result<Self, AllocError> {
        let bv = ByteVector(Vector::from_slice(allocator, data)?);
        Ok(Rc::new(allocator, RefCell::new(bv))?)
    }

}

/* DCOVector ****************************************************************/
#[derive(Debug)]
pub struct DCOVector<'a, T: DataCellOps>(pub Vector<'a, T>);

impl<'a, T: DataCellOps> DataCellOpsMut for DCOVector<'a, T> {

    fn get_property_mut<'x>(
        &mut self,
        property_name: &str,
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        match property_name {
            "len" | "length" | "count" => {
                let v = self.0.len().try_into().unwrap();
                Ok(DataCell::U64(U64Cell::new(v)))
            },
            _ => Err(Error::NotApplicable)
        }
    }

    fn output_as_human_readable_mut<'w, 'x>(
        &mut self,
        out: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        write!(out, "[")?;
        for cell in self.0.as_slice() {
            cell.output_as_human_readable(out, xc)?;
        }
        write!(out, "]")?;
        Ok(())
    }

}

/* Record *******************************************************************/
#[derive(Debug)]
pub struct RecordDesc<'a> {
    field_names: &'a [&'a str],
    record_name: &'a str,
}

impl<'a> RecordDesc<'a> {

    pub const fn new(
        record_name: &'a str,
        field_names: &'a [&'a str],
    ) -> RecordDesc<'a> {
        RecordDesc { field_names, record_name }
    }

    pub fn field_count(&self) -> usize {
        self.field_names.len()
    }
}

#[derive(Debug)]
pub struct Record<'a> {
    data: Vector<'a, DataCell<'a>>,
    desc: &'a RecordDesc<'a>,
}

impl<'a> Record<'a> {

    pub fn new(
        desc: &'a RecordDesc<'a>,
        allocator: AllocatorRef<'a>,
    ) -> Result<Self, AllocError> {
        let mut data: Vector<'a, DataCell<'a>> = Vector::new(allocator);
        let n = desc.field_count();
        data.reserve(n)?;
        for _i in 0..n {
            data.push(DataCell::Nothing).unwrap();
        }
        Ok(Record { data, desc })
    }

    pub fn get_fields_mut<'b>(&'b mut self) -> &'b mut [DataCell<'a>] {
        self.data.as_mut_slice()
    }
}

impl<'a> DataCellOpsMut for Record<'a> {

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
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        out.write_all(self.desc.record_name.as_bytes(), xc)?;
        out.write_all(b"(", xc)?;
        let v = self.data.as_slice();
        let mut first = true;
        for i in 0..self.desc.field_names.len() {
            if v[i].is_nothing() { continue; }
            if first {
                first = false;
            } else {
                out.write_all(b", ", xc)?;
            }
            out.write_all(self.desc.field_names[i].as_bytes(), xc)?;
            out.write_all(b": ", xc)?;
            v[i].output_as_human_readable(out, xc)?;
        }
        out.write_all(b")", xc)?;
        Ok(())
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
    CellVector(Rc<'d, RefCell<DCOVector<'d, DataCell<'d>>>>),
    Record(Rc<'d, RefCell<Record<'d>>>),
    ByteStream(Rc<'d, RefCell<dyn Stream + 'd>>),
}

impl<'d> DataCell<'d> {

    pub fn is_nothing(&self) -> bool {
        match self {
            DataCell::Nothing => true,
            _ => false
        }
    }

    pub fn new() -> Self {
        DataCell::Nothing
    }

    pub fn from_u64_cell(n: U64Cell) -> Self {
        DataCell::U64(n)
    }
    pub fn from_u64(n: u64) -> Self {
        Self::from_u64_cell(U64Cell::new(n))
    }

    pub fn from_static_id(s: &'d str) -> Self {
        DataCell::StaticId(s)
    }
}

impl<'d> DataCellOps for DataCell<'d> {

    fn get_property<'x>(
        &self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        match self {
            DataCell::U64(v) => v.get_property(property_name, xc),
            DataCell::ByteVector(v) => v.get_property(property_name, xc),
            DataCell::CellVector(v) => v.get_property(property_name, xc),
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
            DataCell::CellVector(v) => v.deref().output_as_human_readable(w, xc),
            DataCell::Record(v) => v.deref().output_as_human_readable(w, xc),
            DataCell::ByteStream(_v) => panic!(),
        }
    }

}

impl<T: Stream> DataCellOpsMut for T {

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
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        self.seek(SeekFrom::Start(0), xc)?;
        out.write_all(b"b\"", xc)?;
        let mut buf = [0_u8; 1024];
        loop {
            let chunk_size = self.read_uninterrupted(&mut buf, xc)?;
            if chunk_size == 0 { break; }
            output_byte_slice_as_human_readable_text(&buf[0..chunk_size], out, xc)?;
        }
        out.write_all(b"\"", xc)?;

        Ok(())
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

    #[test]
    fn record_human_readable() {
        use crate::mm::{ Allocator, BumpAllocator };
        let mut buffer = [0_u8; 1000];
        let a = BumpAllocator::new(&mut buffer);
        let mut xc = ExecutionContext::with_allocator_and_logless(a.to_ref());
        let desc = RecordDesc::new("Rectangle", &["width", "height", "mode"]);
        let mut r = Record::new(&desc, a.to_ref()).unwrap();

        {
            let mut o = xc.byte_vector();
            r.output_as_human_readable_mut(&mut o, &mut xc).unwrap();
            assert_eq!(core::str::from_utf8(o.as_slice()).unwrap(), "Rectangle()");
        }

        {
            r.data.as_mut_slice()[1] = DataCell::from_u64(5);
            let mut o = xc.byte_vector();
            r.output_as_human_readable_mut(&mut o, &mut xc).unwrap();
            assert_eq!(core::str::from_utf8(o.as_slice()).unwrap(),
                       "Rectangle(height: 5)");
        }

        {
            r.data.as_mut_slice()[1] = DataCell::from_u64(7);
            r.data.as_mut_slice()[2] = DataCell::from_static_id("FUNKY");
            let mut o = xc.byte_vector();
            r.output_as_human_readable_mut(&mut o, &mut xc).unwrap();
            assert_eq!(core::str::from_utf8(o.as_slice()).unwrap(),
                       "Rectangle(height: 7, mode: FUNKY)");
        }

        {
            r.data.as_mut_slice()[0] = DataCell::from_u64(8);
            r.data.as_mut_slice()[1] = DataCell::new();
            r.data.as_mut_slice()[2] = DataCell::from_static_id("CHECKERED");
            let mut o = xc.byte_vector();
            r.output_as_human_readable_mut(&mut o, &mut xc).unwrap();
            assert_eq!(core::str::from_utf8(o.as_slice()).unwrap(),
                       "Rectangle(width: 8, mode: CHECKERED)");
        }

        {
            r.data.as_mut_slice()[0] = DataCell::from_u64(9);
            use crate::num::fmt as num_fmt;
            let nf = num_fmt::MiniNumFmtPack::new(
                num_fmt::Radix::new(16).unwrap(),
                num_fmt::RadixNotation::DefaultPrefix,
                num_fmt::MinDigitCount::new(2).unwrap(),
                num_fmt::PositiveSign::Plus,
                num_fmt::ZeroSign::Space);
            r.data.as_mut_slice()[1] = DataCell::from_u64_cell(U64Cell::with_fmt(10, nf));
            r.data.as_mut_slice()[2] = DataCell::from_static_id("WEIRDO");
            let mut o = xc.byte_vector();
            r.output_as_human_readable_mut(&mut o, &mut xc).unwrap();
            assert_eq!(core::str::from_utf8(o.as_slice()).unwrap(),
                       "Rectangle(width: 9, height: +0x0A, mode: WEIRDO)");
        }
    }
}

