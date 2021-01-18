extern crate clap;

use core::fmt::Write as FmtWrite;
use std::string::String as StdString;
use std::io::stderr;

use halfbit::DataCell;
use halfbit::ExecutionContext;
use halfbit::LogLevel;
use halfbit::mm::Allocator;
use halfbit::mm::AllocError;
use halfbit::mm::Malloc;
use halfbit::mm::Vector;
use halfbit::mm::String as HbString;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOError;
use halfbit::io::IOPartialError;
use halfbit::io::IOPartialResult;
use halfbit::io::stream::RandomAccessRead;
use halfbit::io::stream::SeekFrom;
use halfbit::conv::int_be_decode;
use halfbit::log_debug;
use halfbit::log_info;
use halfbit::log_warn;
use halfbit::log_error;

#[derive(Debug)]
struct Invocation {
    verbose: bool,
    items: Vec<StdString>,
    attributes: Vec<StdString>,
}

#[derive(Debug)]
enum AttrComputeError<'a> {
    UnknownAttribute,
    NotApplicable,
    Alloc(AllocError),
    IO(IOError<'a>),
}

impl<'a> std::fmt::Display for AttrComputeError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrComputeError::UnknownAttribute => write!(f, "unknown attribute"),
            AttrComputeError::NotApplicable => write!(f, "not applicable"),
            AttrComputeError::Alloc(ae) => write!(f, "{:?}", ae),
            AttrComputeError::IO(x) => write!(f, "I/O error: {}", x),
        }
    }
}

impl<'a> core::convert::From<IOError<'a>> for AttrComputeError<'a> {
    fn from(e: IOError<'a>) -> Self {
        AttrComputeError::IO(e)
    }
}

impl<'a> core::convert::From<IOPartialError<'a>> for AttrComputeError<'a> {
    fn from(e: IOPartialError<'a>) -> Self {
        AttrComputeError::IO(e.to_error())
    }
}

impl<'a> core::convert::From<AllocError> for AttrComputeError<'a> {
    fn from(e: AllocError) -> Self {
        AttrComputeError::Alloc(e)
    }
}

impl<'a> core::convert::From<(AllocError, DataCell<'_>)> for AttrComputeError<'a> {
    fn from(e: (AllocError, DataCell<'_>)) -> Self {
        AttrComputeError::Alloc(e.0)
    }
}

struct Item<'a> {
    name: &'a str,
    stream: &'a mut (dyn RandomAccessRead + 'a),
}

struct ProcessingStatus {
    accessible_items: usize,
    inaccessible_items: usize,
    attributes_computed_ok: usize,
    attributes_not_applicable: usize,
    attributes_failed_to_compute: usize,
}

impl ProcessingStatus {
    pub fn new () -> Self {
        ProcessingStatus {
            accessible_items: 0,
            inaccessible_items: 0,
            attributes_computed_ok: 0,
            attributes_not_applicable: 0,
            attributes_failed_to_compute: 0,
        }
    }
    pub fn add(&mut self, other: &Self) {
        self.accessible_items += other.accessible_items;
        self.inaccessible_items += other.inaccessible_items;
        self.attributes_computed_ok += other.attributes_computed_ok;
        self.attributes_not_applicable += other.attributes_not_applicable;
        self.attributes_failed_to_compute += other.attributes_failed_to_compute;
    }
}


/* process_args *************************************************************/
fn process_args(args: Vec<StdString>) -> Invocation {
    let m = clap::App::new("halfbit")
        .version("0.0")
        .author("by Costin Ionescu <costin.ionescu@gmail.com>")
        .about("examines given items and generates a report")
        .arg(clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("prints what it does verbosely"))
        .arg(clap::Arg::with_name("items")
                .help("item(s) to process (file paths for now)")
                .multiple(true))
        .arg(clap::Arg::with_name("attr")
                .short("a")
                .long("attr")
                .help("computes given attributes")
                .takes_value(true)
                .multiple(true))
        .get_matches_from(args);

    let inv = Invocation {
        verbose: m.is_present("verbose"),
        items:
            if let Some(values) = m.values_of("items") {
                values.map(|x| StdString::from(x)).collect()
            } else {
                Vec::new()
            },
        attributes:
            if let Some(values) = m.values_of("attr") {
                values.map(|x| StdString::from(x)).collect()
            } else {
                Vec::new()
            },
    };

    if cfg!(debug_assertions) && inv.verbose {
        eprintln!("cmd line: {:#?}", m);
        eprintln!("inv: {:#?}", inv);
    }

    inv
}

fn extract_first_byte <'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    item.stream.seek(SeekFrom::Start(0), xc)
    .map_err(|e| IOPartialError::from_error_and_size(e, 0))
    .and_then(|_| item.stream.read_u8(xc))
    .map(|v| DataCell::U64(v as u64))
    .map_err(|e|
        if e.get_error_code() == IOErrorCode::UnexpectedEnd {
            AttrComputeError::NotApplicable
        } else {
            AttrComputeError::IO(e.to_error())
        })
}

fn first_8_bytes<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    let mut buf = [0_u8; 8];
    let n = item.stream.seek_read(0, &mut buf, xc)?;
    Ok(DataCell::ByteVector(Vector::from_slice(xc.get_main_allocator(), &buf[0..n])?))
}

fn identify_top_of_file_records<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    let mut ids: Vector<'x, DataCell> = Vector::new(xc.get_main_allocator());
    let mut tof_buffer = [0_u8; 0x40];
    let tof_len = item.stream.seek_read(0, &mut tof_buffer, xc)?;
    let tof = &tof_buffer[0..tof_len];
    if tof.starts_with(b"PK") {
        ids.push(DataCell::Identifier(HbString::map_str("zip_record")))?;
    } else if tof.starts_with(b"#!") {
        ids.push(DataCell::Identifier(HbString::map_str("shebang")))?;
    } else if tof.starts_with(b"\x7FELF") {
        ids.push(DataCell::Identifier(HbString::map_str("elf")))?;
    } else if tof.starts_with(b"MZ") {
        ids.push(DataCell::Identifier(HbString::map_str("dos_exe")))?;
    } else if tof.starts_with(b"ZM") {
        ids.push(DataCell::Identifier(HbString::map_str("dos_exe")))?;
    } else if tof.starts_with(b"\x1F\x8B") {
        ids.push(DataCell::Identifier(HbString::map_str("gzip")))?;
    } else if tof.starts_with(b"BZ") {
        ids.push(DataCell::Identifier(HbString::map_str("bzip2")))?;
    } else if tof.starts_with(b"\xFD7zXZ\x00") {
        ids.push(DataCell::Identifier(HbString::map_str("xz")))?;
    } else if tof.starts_with(b"!<arch>\n") {
        ids.push(DataCell::Identifier(HbString::map_str("ar")))?;
    } else if tof.starts_with(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1") {
        ids.push(DataCell::Identifier(HbString::map_str("ms_cfb")))?;
    } else if tof.starts_with(b"QFI\xFB") {
        ids.push(DataCell::Identifier(HbString::map_str("qcow")))?;
        if tof_len >= 8 {
            let ver: u32 = int_be_decode(&tof[4..8]).unwrap();
            let mut id = xc.string();
            write!(id, "qcow{}", ver)
                .map_err(|_| AttrComputeError::Alloc(AllocError::NotEnoughMemory))?;
            ids.push(DataCell::Identifier(id))?;
        }
    }
    Ok(DataCell::CellVector(ids))
}

pub const ELFCLASSNONE: u8 = 0;
pub const ELFCLASS32: u8 = 1;
pub const ELFCLASS64: u8 = 2;

pub const ELFDATANONE: u8 = 0;
pub const ELFDATA2LSB: u8 = 1;
pub const ELFDATA2MSB: u8 = 2;

const ELF_HEADER_FIELDS: &[&'static str] = &[
    "ei_magic", "ei_class", "ei_data", "ei_version",
    "ei_osabi", "ei_abiversion", "ei_pad",
    "e_type", "e_machine", "e_version", "e_entry", "e_phoff", "e_shoff",
];

fn elf_header<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    let mut eh: Vector<'x, DataCell<'x>> = xc.vector();
    let mut magic = [0_u8; 4];
    item.stream.seek_read(0, &mut magic, xc)?;
    eh.push(DataCell::ByteVector(Vector::from_slice(xc.get_main_allocator(), &magic)?))?;
    let ei_class = item.stream.read_u8(xc)?;
    eh.push(match ei_class {
        0 => DataCell::Identifier(HbString::map_str("ELFCLASSNONE")),
        1 => DataCell::Identifier(HbString::map_str("ELFCLASS32")),
        2 => DataCell::Identifier(HbString::map_str("ELFCLASS64")),
        _ => DataCell::U64(ei_class.into()),
    })?;
    let ei_data = item.stream.read_u8(xc)?;
    eh.push(match ei_data {
        0 => DataCell::Identifier(HbString::map_str("ELFDATANONE")),
        1 => DataCell::Identifier(HbString::map_str("ELFDATA2LSB")),
        2 => DataCell::Identifier(HbString::map_str("ELFDATA2MSB")),
        _ => DataCell::U64(ei_data.into()),
    })?;
    let ei_version = item.stream.read_u8(xc)?;
    eh.push(match ei_version {
        0 => DataCell::Identifier(HbString::map_str("EV_NONE")),
        1 => DataCell::Identifier(HbString::map_str("EV_CURRENT")),
        _ => DataCell::U64(ei_version.into()),
    })?;
    let ei_osabi = item.stream.read_u8(xc)?;
    eh.push(match ei_osabi {
        0 => DataCell::Identifier(HbString::map_str("ELFOSABI_NONE")),
        1 => DataCell::Identifier(HbString::map_str("ELFOSABI_HPUX")),
        2 => DataCell::Identifier(HbString::map_str("ELFOSABI_NETBSD")),
        3 => DataCell::Identifier(HbString::map_str("ELFOSABI_LINUX")),
        6 => DataCell::Identifier(HbString::map_str("ELFOSABI_SOLARIS")),
        7 => DataCell::Identifier(HbString::map_str("ELFOSABI_AIX")),
        8 => DataCell::Identifier(HbString::map_str("ELFOSABI_IRIX")),
        9 => DataCell::Identifier(HbString::map_str("ELFOSABI_FREEBSD")),
        10 => DataCell::Identifier(HbString::map_str("ELFOSABI_TRU64")),
        11 => DataCell::Identifier(HbString::map_str("ELFOSABI_MODESTO")),
        12 => DataCell::Identifier(HbString::map_str("ELFOSABI_OPENBSD")),
        13 => DataCell::Identifier(HbString::map_str("ELFOSABI_OPENVMS")),
        14 => DataCell::Identifier(HbString::map_str("ELFOSABI_NSK")),
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

    Ok(DataCell::Record(eh, ELF_HEADER_FIELDS))
}

fn process_item_attribute<'a, 'x>(
    item: &mut Item<'a>,
    attr: &str,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    match attr {
        "first_byte" => extract_first_byte(item, xc),
        "first_8_bytes" => first_8_bytes(item, xc),
        "tof_ids" => identify_top_of_file_records(item, xc),
        "elf_header" => elf_header(item, xc),
        _ => Err(AttrComputeError::UnknownAttribute)
    }
}

fn process_item<'x>(
    item_name: &str,
    invocation: &Invocation,
    xc: &mut ExecutionContext<'x>,
) -> ProcessingStatus {
    let mut status = ProcessingStatus::new();

    if invocation.verbose {
        log_info!(xc, "processing {:?}", item_name);
    }
    let mut f = match std::fs::File::open(item_name) {
        Ok(f) => {
            status.accessible_items = 1;
            f
        },
        Err(e) => {
            log_error!(xc, "error opening file {:?}: {}", item_name, e);
            return status;
        }
    };
    let mut item = Item {
        name: item_name,
        stream: &mut f,
    };

    for attr in &invocation.attributes {
        if invocation.verbose {
            log_info!(xc, "computing attribute {:?} for item {:?}", attr, item_name);
        }
        match process_item_attribute(&mut item, attr, xc) {
            Ok(av) => {
                println!("{:?}\t{}\t{}", item_name, attr, av);
                status.attributes_computed_ok += 1;
            },
            Err(e) => {
                match e {
                    AttrComputeError::NotApplicable => {
                        status.attributes_not_applicable += 1;
                        log_warn!(xc, "warning:{:?}:{:?}:{}", item.name, attr, e);
                    },
                    _ => {
                        status.attributes_failed_to_compute += 1;
                        log_error!(xc, "error:{:?}:{:?}:{}", item.name, attr, e);
                    }
                }
            },
        }
    }
    status
}

/* run **********************************************************************/
fn run(
    invocation: &Invocation,
    xc: &mut ExecutionContext<'_>
) -> Result<(), u8> {
    if invocation.verbose {
        log_info!(xc, "lib: {}", halfbit::lib_name());
    }
    let mut summary = ProcessingStatus::new();
    for item in &invocation.items {
        summary.add(&process_item(item, invocation, xc));
    }
    if invocation.verbose {
        log_info!(xc, "accessible items: {}", summary.accessible_items);
        log_info!(xc, "inaccessible items: {}", summary.inaccessible_items);
        log_info!(xc, "attributes computed ok: {}", summary.attributes_computed_ok);
        log_info!(xc, "attributes not applicable: {}", summary.attributes_not_applicable);
        log_info!(xc, "attributes failed to compute: {}", summary.attributes_failed_to_compute);
    }
    let rc = 0_u8
        | if summary.attributes_not_applicable != 0 { 1 } else { 0 }
        | if summary.attributes_failed_to_compute != 0 { 2 } else { 0 }
        | if summary.inaccessible_items != 0 { 4 } else { 0 }
        | if xc.get_logging_error_mask() != 0 { 8 } else { 0 }
        | 0_u8;

    if rc == 0 {
        Ok(())
    } else {
        log_error!(xc, "completed with errors");
        Err(rc)
    }
}

/* main *********************************************************************/
fn main() {
    let invocation = process_args(std::env::args().collect());
    let a = Malloc::new();
    let err = stderr();
    let mut log = err.lock();
    let mut xc = ExecutionContext::new(
        a.to_ref(),
        a.to_ref(),
        &mut log,
        if invocation.verbose { LogLevel::Debug } else { LogLevel::Warning },
    );
    run(&invocation, &mut xc)
    .unwrap_or_else(|e| {
        log_debug!(xc, "* exiting with code {}", e);
        std::process::exit(e as i32);
    });
}

