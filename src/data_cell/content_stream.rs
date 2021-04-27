use core::cell::RefCell;

use crate::ExecutionContext;
use crate::conv::int_be_decode;
use crate::data_cell::ByteVector;
use crate::data_cell::DCOVector;
use crate::data_cell::DataCell;
use crate::data_cell::DataCellOpsMut;
use crate::data_cell::Error;
use crate::data_cell::Record;
use crate::data_cell::RecordDesc;
use crate::data_cell::U64Cell;
use crate::io::ErrorCode as IOErrorCode;
use crate::io::IOPartialError;
use crate::io::stream::RandomAccessRead;
use crate::io::stream::SeekFrom;
use crate::io::stream::Write;
use crate::mm::Vector;
use crate::num::fmt as num_fmt;

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

    fn extract_elf_header<'x>(
        &mut self,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, Error<'x>> {
        let mut eh = Record::new(&Self::ELF_HEADER, xc.get_main_allocator())?;
        let ehf = eh.get_fields_mut();
        let mut magic = [0_u8; 4];
        self.stream.seek_read(0, &mut magic, xc)?;
        ehf[0] = DataCell::ByteVector(xc.rc(RefCell::new(ByteVector(xc.byte_vector_clone(&magic)?)))?);
        Ok(DataCell::Record(xc.rc(RefCell::new(eh))?))
        /*
        let mut eh: Vector<'x, DataCell<'x>> = xc.vector();
        let mut magic = [0_u8; 4];
        item.stream.seek_read(0, &mut magic, xc)?;
        eh.push(DataCell::ByteVector(Vector::from_slice(xc.get_main_allocator(), &magic)?))?;
        let ei_class = item.stream.read_u8(xc)?;
        eh.push(match ei_class {
            0 => DataCell::Identifier(String::map_str("ELFCLASSNONE")),
            1 => DataCell::Identifier(String::map_str("ELFCLASS32")),
            2 => DataCell::Identifier(String::map_str("ELFCLASS64")),
            _ => DataCell::U64(ei_class.into()),
        })?;
        let ei_data = item.stream.read_u8(xc)?;
        eh.push(match ei_data {
            0 => DataCell::Identifier(String::map_str("ELFDATANONE")),
            1 => DataCell::Identifier(String::map_str("ELFDATA2LSB")),
            2 => DataCell::Identifier(String::map_str("ELFDATA2MSB")),
            _ => DataCell::U64(ei_data.into()),
        })?;
        let ei_version = item.stream.read_u8(xc)?;
        eh.push(match ei_version {
            0 => DataCell::Identifier(String::map_str("EV_NONE")),
            1 => DataCell::Identifier(String::map_str("EV_CURRENT")),
            _ => DataCell::U64(ei_version.into()),
        })?;
        let ei_osabi = item.stream.read_u8(xc)?;
        eh.push(match ei_osabi {
            0 => DataCell::Identifier(String::map_str("ELFOSABI_NONE")),
            1 => DataCell::Identifier(String::map_str("ELFOSABI_HPUX")),
            2 => DataCell::Identifier(String::map_str("ELFOSABI_NETBSD")),
            3 => DataCell::Identifier(String::map_str("ELFOSABI_LINUX")),
            6 => DataCell::Identifier(String::map_str("ELFOSABI_SOLARIS")),
            7 => DataCell::Identifier(String::map_str("ELFOSABI_AIX")),
            8 => DataCell::Identifier(String::map_str("ELFOSABI_IRIX")),
            9 => DataCell::Identifier(String::map_str("ELFOSABI_FREEBSD")),
            10 => DataCell::Identifier(String::map_str("ELFOSABI_TRU64")),
            11 => DataCell::Identifier(String::map_str("ELFOSABI_MODESTO")),
            12 => DataCell::Identifier(String::map_str("ELFOSABI_OPENBSD")),
            13 => DataCell::Identifier(String::map_str("ELFOSABI_OPENVMS")),
            14 => DataCell::Identifier(String::map_str("ELFOSABI_NSK")),
            _ => DataCell::U64(ei_osabi.into()),
        })?;
        eh.push(DataCell::U64(item.stream.read_u8(xc)?.into()))?;
        let mut ei_pad = [0_u8; 7];
        item.stream.read_exact(&mut ei_pad, xc)?;
        eh.push(DataCell::ByteVector(Vector::from_slice(xc.get_main_allocator(), &ei_pad)?))?;

        fn read_u16le_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u16le(xc).map(|v| v as u64)
        }
        fn read_u16be_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u16be(xc).map(|v| v as u64)
        }
        fn read_u32le_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u32le(xc).map(|v| v as u64)
        }
        fn read_u32be_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u32be(xc).map(|v| v as u64)
        }
        fn read_u64le_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u64le(xc).map(|v| v as u64)
        }
        fn read_u64be_as_u64<'x>(r: &mut dyn RandomAccessRead, xc: &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64> {
            r.read_u64be(xc).map(|v| v as u64)
        }
        let read_half: &dyn Fn(&mut dyn RandomAccessRead, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_word: &dyn Fn(&mut dyn RandomAccessRead, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_addr: &dyn Fn(&mut dyn RandomAccessRead, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
        let read_off: &dyn Fn(&mut dyn RandomAccessRead, &mut ExecutionContext<'x>) -> IOPartialResult<'x, u64>;
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
            return Ok(DataCell::Record(eh, ELF_HEADER_FIELDS));
        }
        let e_type = read_half(item.stream, xc)?;
        let e_machine = read_half(item.stream, xc)?;
        let e_version = read_word(item.stream, xc)?;
        let e_entry = read_addr(item.stream, xc)?;
        let e_phoff = read_off(item.stream, xc)?;
        let e_shoff = read_off(item.stream, xc)?;
        eh.push(DataCell::U64(e_type))?;
        eh.push(DataCell::U64(e_machine))?;
        eh.push(DataCell::U64(e_version))?;
        eh.push(DataCell::U64(e_entry))?;
        eh.push(DataCell::U64(e_phoff))?;
        eh.push(DataCell::U64(e_shoff))?;

    */
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
        _out: &mut (dyn Write + 'w),
        _xc: &mut ExecutionContext<'x>,
    ) -> Result<(), Error<'x>> {
        Err(Error::NotApplicable)
    }

}
