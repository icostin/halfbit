use core::cell::RefCell;

use crate::ExecutionContext;
use crate::conv::int_be_decode;
use crate::data_cell::DCOVector;
use crate::data_cell::DataCell;
use crate::data_cell::DataCellOpsMut;
use crate::data_cell::Error;
use crate::data_cell::Record;
use crate::data_cell::RecordDesc;
use crate::data_cell::U64Cell;
use crate::data_cell::output_byte_slice_as_human_readable_text;
use crate::io::ErrorCode as IOErrorCode;
use crate::io::IOPartialError;
use crate::io::IOPartialResult;
use crate::io::stream::RandomAccessRead;
use crate::io::stream::SeekFrom;
use crate::io::stream::Write;
use crate::mm::Vector;
use crate::num::fmt as num_fmt;

pub const ELFCLASSNONE: u8 = 0;
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

pub const ELFDATANONE: u8 = 0;
pub const ELFDATA2LSB: u8 = 1;
pub const ELFDATA2MSB: u8 = 2;

const ELF_HEADER: RecordDesc<'static> = RecordDesc::new(
    "elf_header",
    &[
        "ei_magic", "ei_class", "ei_data", "ei_version",
        "ei_osabi", "ei_abiversion", "ei_pad",
        "e_type", "e_machine", "e_version", "e_entry", "e_phoff", "e_shoff",
    ]);

/* ContentStream ************************************************************/
#[derive(Debug)]
pub struct ContentStream<'a, T: ?Sized + RandomAccessRead> {
    stream: &'a mut T
}

impl<'a, T: ?Sized + RandomAccessRead> ContentStream<'a, T> {

    pub fn new(stream: &'a mut T) -> Self {
        ContentStream { stream }
    }
    fn extract_first_byte <'x>(
        &mut self,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        self.stream.seek(SeekFrom::Start(0), xc)
        .map_err(|e| IOPartialError::from_error_and_size(e, 0))
        .and_then(|_| self.stream.read_u8(xc))
        .map(|v| DataCell::U64(U64Cell {
            n: v as u64,
            fmt_pack: num_fmt::MiniNumFmtPack::new(
                num_fmt::Radix::new(16).unwrap(),
                num_fmt::RadixNotation::DefaultExplicitPrefix,
                num_fmt::MinDigitCount::new(2).unwrap(),
                num_fmt::PositiveSign::Hidden,
                num_fmt::ZeroSign::Hidden)
        }))
        .map_err(|e|
            if e.get_error_code() == IOErrorCode::UnexpectedEnd {
                Error::NotApplicable
            } else {
                Error::IO(e.to_error())
            })
    }

    fn first_8_bytes<'x>(
        &mut self,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        let mut buf = [0_u8; 8];
        let n = self.stream.seek_read(0, &mut buf, xc)?;
        Ok(DataCell::from_byte_slice(xc.get_main_allocator(), &buf[0..n])?)
    }

    pub fn identify_top_of_file_records<'x>(
        &mut self,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        let mut ids: Vector<'x, DataCell> = Vector::new(xc.get_main_allocator());
        let mut tof_buffer = [0_u8; 0x40];
        let tof_len = self.stream.seek_read(0, &mut tof_buffer, xc)?;
        let tof = &tof_buffer[0..tof_len];
        if tof_len == 0 {
            ids.push(DataCell::StaticId("empty"))?;
        } else if tof.starts_with(b"PK") {
            ids.push(DataCell::StaticId("zip_record"))?;
        } else if tof.starts_with(b"#!") {
            ids.push(DataCell::StaticId("shebang"))?;
        } else if tof.starts_with(b"\x7FELF") {
            ids.push(DataCell::StaticId("elf"))?;
        } else if tof.starts_with(b"MZ") {
            ids.push(DataCell::StaticId("dos_exe"))?;
        } else if tof.starts_with(b"ZM") {
            ids.push(DataCell::StaticId("dos_exe"))?;
            ids.push(DataCell::StaticId("dos_exe_zm"))?;
        } else if tof.starts_with(b"\x1F\x8B") {
            ids.push(DataCell::StaticId("gzip"))?;
        } else if tof.starts_with(b"BZh") {
            ids.push(DataCell::StaticId("bzip2"))?;
        } else if tof.starts_with(b"\xFD7zXZ\x00") {
            ids.push(DataCell::StaticId("xz"))?;
        } else if tof.starts_with(b"7z\xBC\xAF\x27\x1C") {
            ids.push(DataCell::StaticId("seven_zip"))?;
        } else if tof.starts_with(b"!<arch>\n") {
            ids.push(DataCell::StaticId("ar"))?;
        } else if tof.starts_with(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1") {
            ids.push(DataCell::StaticId("ms_cfb"))?;
        } else if tof.starts_with(b"QFI\xFB") {
            ids.push(DataCell::StaticId("qcow"))?;
            if tof_len >= 8 {
                let ver: u32 = int_be_decode(&tof[4..8]).unwrap();
                match ver {
                    1 => ids.push(DataCell::StaticId("qcow1"))?,
                    2 => ids.push(DataCell::StaticId("qcow2"))?,
                    3 => ids.push(DataCell::StaticId("qcow3"))?,
                    _ => {}
                }
            }
        } else if tof.starts_with(b"SQLite format 3\x00") {
            ids.push(DataCell::StaticId("sqlite3"))?;
        } else if tof.starts_with(b"qres\x00\x00\x00\x01") {
            ids.push(DataCell::StaticId("qt_rcc"))?;
        }
        Ok(DataCell::CellVector(xc.rc(RefCell::new(DCOVector(ids)))?))
    }

    fn extract_elf_header<'x>(
        &mut self,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {

        let a = xc.get_main_allocator();
        let mut eh = Record::new(&ELF_HEADER, a)?;


        let mut magic = [0_u8; 4];
        self.stream.seek_read(0, &mut magic, xc)?;
        eh.set_field("ei_magic", DataCell::from_byte_slice(a, &magic)?);

        let ei_class = self.stream.read_u8(xc)?;
        eh.set_field("ei_class", match ei_class {
            0 => DataCell::from_static_id("ELFCLASSNONE"),
            1 => DataCell::from_static_id("ELFCLASS32"),
            2 => DataCell::from_static_id("ELFCLASS64"),
            n => DataCell::from_u64(n.into()),
        });

        let ei_data = self.stream.read_u8(xc)?;
        eh.set_field("ei_data", match ei_data {
            0 => DataCell::from_static_id("ELFDATANONE"),
            1 => DataCell::from_static_id("ELFDATA2LSB"),
            2 => DataCell::from_static_id("ELFDATA2MSB"),
            n => DataCell::from_u64(n.into()),
        });

        let ei_version = match self.stream.read_u8(xc)? {
            0 => DataCell::from_static_id("EV_NONE"),
            1 => DataCell::from_static_id("EV_CURRENT"),
            n => DataCell::from_u64(n.into()),
        };
        eh.set_field("ei_version", ei_version);

        let ei_osabi = match self.stream.read_u8(xc)? {
            0 => DataCell::from_static_id("ELFOSABI_NONE"),
            1 => DataCell::from_static_id("ELFOSABI_HPUX"),
            2 => DataCell::from_static_id("ELFOSABI_NETBSD"),
            3 => DataCell::from_static_id("ELFOSABI_LINUX"),
            6 => DataCell::from_static_id("ELFOSABI_SOLARIS"),
            7 => DataCell::from_static_id("ELFOSABI_AIX"),
            8 => DataCell::from_static_id("ELFOSABI_IRIX"),
            9 => DataCell::from_static_id("ELFOSABI_FREEBSD"),
            10 => DataCell::from_static_id("ELFOSABI_TRU64"),
            11 => DataCell::from_static_id("ELFOSABI_MODESTO"),
            12 => DataCell::from_static_id("ELFOSABI_OPENBSD"),
            13 => DataCell::from_static_id("ELFOSABI_OPENVMS"),
            14 => DataCell::from_static_id("ELFOSABI_NSK"),
            n => DataCell::from_u64(n.into()),
        };
        eh.set_field("ei_osabi", ei_osabi);

        let ei_abiversion = self.stream.read_u8(xc)?;
        eh.set_field("ei_abiversion", DataCell::from_u64(ei_abiversion.into()));

        let mut ei_pad = [0_u8; 7];
        self.stream.read_uninterrupted(&mut ei_pad, xc)?;
        eh.set_field("ei_pad", DataCell::from_byte_slice(a, &ei_pad)?);

        fn read_u16le_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u16le(xc).map(|v| v as u64)
        }
        fn read_u16be_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u16be(xc).map(|v| v as u64)
        }
        fn read_u32le_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u32le(xc).map(|v| v as u64)
        }
        fn read_u32be_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u32be(xc).map(|v| v as u64)
        }
        fn read_u64le_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u64le(xc).map(|v| v as u64)
        }
        fn read_u64be_as_u64<'x, T: ?Sized + RandomAccessRead>(r: &mut T, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u64be(xc).map(|v| v as u64)
        }
        let read_half: &dyn Fn(&mut T, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_word: &dyn Fn(&mut T, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_addr: &dyn Fn(&mut T, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_off: &dyn Fn(&mut T, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        if ei_data == ELFDATA2LSB && ei_class == ELFCLASS32 {
            read_half = &read_u16le_as_u64;
            read_word = &read_u32le_as_u64;
            read_addr = &read_u32le_as_u64;
            read_off = &read_u32le_as_u64;
        } else if ei_data == ELFDATA2MSB && ei_class == ELFCLASS32 {
            read_half = &read_u16be_as_u64;
            read_word = &read_u32be_as_u64;
            read_addr = &read_u32be_as_u64;
            read_off = &read_u32be_as_u64;
        } else if ei_data == ELFDATA2LSB && ei_class == ELFCLASS64 {
            read_half = &read_u16le_as_u64;
            read_word = &read_u32le_as_u64;
            read_addr = &read_u64le_as_u64;
            read_off = &read_u64le_as_u64;
        } else if ei_data == ELFDATA2MSB && ei_class == ELFCLASS64 {
            read_half = &read_u16be_as_u64;
            read_word = &read_u32be_as_u64;
            read_addr = &read_u64be_as_u64;
            read_off = &read_u64be_as_u64;
        } else {
            return Ok(DataCell::Record(xc.rc(RefCell::new(eh))?))
        }

        let e_type = read_half(&mut self.stream, xc)?;
        eh.set_field("e_type", DataCell::from_u64(e_type));

        let e_machine = read_half(&mut self.stream, xc)?;
        eh.set_field("e_machine", DataCell::from_u64(e_machine));

        let e_version = read_word(&mut self.stream, xc)?;
        eh.set_field("e_version", DataCell::from_u64(e_version));

        let e_entry = read_addr(&mut self.stream, xc)?;
        eh.set_field("e_entry", DataCell::from_u64_cell(U64Cell::hex(e_entry)));

        let e_phoff = read_off(&mut self.stream, xc)?;
        eh.set_field("e_phoff", DataCell::from_u64_cell(U64Cell::hex(e_phoff)));

        let e_shoff = read_off(&mut self.stream, xc)?;
        eh.set_field("e_shoff", DataCell::from_u64_cell(U64Cell::hex(e_shoff)));

        Ok(DataCell::Record(xc.rc(RefCell::new(eh))?))
    }

}
impl<'a, T: ?Sized + RandomAccessRead> DataCellOpsMut for ContentStream<'a, T> {

    fn get_property_mut<'x>(
        &mut self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        match property_name {
            "fourty_two" => Ok(DataCell::U64(U64Cell::new(42))),
            "first_byte" => self.extract_first_byte(xc),
            "first_8_bytes" => self.first_8_bytes(xc),
            "tof_ids" => self.identify_top_of_file_records(xc),
            "elf_header" => self.extract_elf_header(xc),
            _ => Err(Error::NotApplicable),
        }
    }

    fn output_as_human_readable_mut<'w, 'x>(
        &mut self,
        out: &mut (dyn Write + 'w),
        xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        self.stream.seek(SeekFrom::Start(0), xc)?;
        let mut buffer = [0_u8; 1024];
        loop {
            let n = self.stream.read(&mut buffer, xc)?;
            if n == 0 { break; }
            output_byte_slice_as_human_readable_text(&buffer[0..n], out, xc)?;
        }
        Ok(())
    }

}
