extern crate clap;

use core::cell::RefCell;
use core::fmt;

use std::string::String as StdString;
use std::io::stdout;
use std::io::stderr;
use std::fmt::Write as FmtWrite;

use halfbit::ExecutionContext;
use halfbit::LogLevel;
use halfbit::num::fmt as num_fmt;
use halfbit::mm::Allocator;
//use halfbit::mm::AllocError;
use halfbit::mm::Malloc;
use halfbit::mm::Vector;
use halfbit::mm::Rc;
//use halfbit::mm::String as HbString;
//use halfbit::mm::Vector as HbVector;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOPartialError;
use halfbit::io::IOError;
//use halfbit::io::IOPartialResult;
//use halfbit::io::stream::RandomAccessRead;
use halfbit::io::stream::SeekFrom;
use halfbit::io::stream::Seek;
use halfbit::io::stream::Read;
use halfbit::io::stream::Write;
//use halfbit::conv::int_be_decode;
use halfbit::log_debug;
use halfbit::log_info;
use halfbit::log_warn;
use halfbit::log_error;
use halfbit::log_crit;

use halfbit::data_cell;
use halfbit::data_cell::U64Cell;
use halfbit::data_cell::DataCell;
use halfbit::data_cell::DataCellOps;
use halfbit::data_cell::Error;
use halfbit::data_cell::expr::Source;
use halfbit::data_cell::expr::Parser;
use halfbit::data_cell::expr::Expr;
use halfbit::data_cell::expr::BasicTokenType;
//use halfbit::data_cell::expr::ParseError;
use halfbit::data_cell::eval::Eval;

/*
use halfbit::data_cell_v0::DataCell;
use halfbit::data_cell_v0::DataCellOps;
//use halfbit::data_cell_v0::DataCellOpsExtra;
use halfbit::data_cell_v0::Error;
//use halfbit::data_cell_v0::expr::ParseError;
use halfbit::data_cell_v0::Eval;
*/

#[derive(Copy, Clone, Debug)]
struct ExitCode(u8);

#[derive(Debug)]
struct Invocation {
    verbose: bool,
    items: Vec<StdString>,
    expressions: Vec<StdString>,
}

struct Item<'a> {
    name: &'a str,
    file: RefCell<std::fs::File>,
}

struct ProcessingStatus {
    accessible_items: usize,
    inaccessible_items: usize,
    attributes_computed_ok: usize,
    attributes_not_applicable: usize,
    attributes_failed_to_compute: usize,
    output_error: bool,
}

impl ExitCode {
    pub fn new(code: u8) -> Self {
        Self(code)
    }
    pub fn to_result(&self) -> Result<(), ExitCode> {
        if self.0 == 0 {
            Ok(())
        } else {
            Err(*self)
        }
    }
}

impl<'a> fmt::Debug for Item<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Item({:?})", self.name)
    }
}

impl ProcessingStatus {
    pub fn new () -> Self {
        ProcessingStatus {
            accessible_items: 0,
            inaccessible_items: 0,
            attributes_computed_ok: 0,
            attributes_not_applicable: 0,
            attributes_failed_to_compute: 0,
            output_error: false,
        }
    }
    pub fn add(&mut self, other: &Self) {
        self.accessible_items += other.accessible_items;
        self.inaccessible_items += other.inaccessible_items;
        self.attributes_computed_ok += other.attributes_computed_ok;
        self.attributes_not_applicable += other.attributes_not_applicable;
        self.attributes_failed_to_compute += other.attributes_failed_to_compute;
        self.output_error |= other.output_error;
    }
}

impl<'a> fmt::Display for Item<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "file({:?})", self.name)
    }
}

impl<'a> DataCellOps for Item<'a> {
    fn get_property<'x>(
        &self,
        property_name: &str,
        xc: &mut ExecutionContext<'x>,
    ) -> Result<DataCell<'x>, data_cell::Error<'x>> {
        match property_name {
            "fourty_two" => Ok(DataCell::U64(
                    U64Cell{
                        n: 42,
                        fmt_pack: num_fmt::MiniNumFmtPack::default()
                    })),
            "first_byte" => extract_first_byte(&self, xc),
            _ => Err(data_cell::Error::NotApplicable),
        }
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
        .arg(clap::Arg::with_name("eval")
                .short("e")
                .long("eval")
                .help("computes given expressions for each item")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1))
        .get_matches_from(args);

    let inv = Invocation {
        verbose: m.is_present("verbose"),
        items:
            if let Some(values) = m.values_of("items") {
                values.map(|x| StdString::from(x)).collect()
            } else {
                Vec::new()
            },
        expressions:
            if let Some(values) = m.values_of("eval") {
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
    item: &Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, data_cell::Error<'x>> {
    let mut f = item.file.borrow_mut();
    f.seek(SeekFrom::Start(0), xc)
    .map_err(|e| IOPartialError::from_error_and_size(e, 0))
    .and_then(|_| f.read_u8(xc))
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
            data_cell::Error::NotApplicable
        } else {
            data_cell::Error::IO(e.to_error())
        })
}

/*
fn first_8_bytes<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, Error<'x>> {
    let mut buf = [0_u8; 8];
    let n = item.stream.seek_read(0, &mut buf, xc)?;
    Ok(DataCell::ByteVector(Vector::from_slice(xc.get_main_allocator(), &buf[0..n])?))
}

fn identify_top_of_file_records<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, Error<'x>> {
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
                .map_err(|_| Error::Alloc(AllocError::NotEnoughMemory))?;
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
) -> Result<DataCell<'x>, Error<'x>> {
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
*/

fn output_expr_value<'x>(
    item_name: &str,
    expr: &Expr<'x>,
    value: &DataCell<'x>,
    out: &mut (dyn Write + '_),
    xc: &mut ExecutionContext<'x>,
) -> Result<(), Error<'x>> {
    write!(out, "{:?}\t{}\t", item_name, expr)
        .map_err(|_| Error::Output(
                    IOError::with_str(IOErrorCode::Unsuccessful, "output error")))
        .and_then(|_| value.to_human_readable(out, xc))
        .and_then(|_| out.write_all(b"\n", xc).map_err(|e| Error::Output(e.to_error())))
}

fn process_item<'x>(
    item_name: &'x str,
    eval_expr_list: &[Expr<'x>],
    out: &mut (dyn Write + '_),
    xc: &mut ExecutionContext<'x>,
) -> ProcessingStatus {
    let mut status = ProcessingStatus::new();

    log_info!(xc, "processing {:?}: evaluating {:?}", item_name, eval_expr_list);
    let f = match std::fs::File::open(item_name) {
        Ok(f) => {
            status.accessible_items = 1;
            f
        },
        Err(e) => {
            log_error!(xc, "error opening file {:?}: {}", item_name, e);
            return status;
        }
    };

    let item = Item {
        name: item_name,
        file: RefCell::new(f),
    };
    let root = match xc.rc(item) {
        Ok(b) => Rc::to_dyn::<dyn DataCellOps + 'x>(b),
        Err((e, item)) => {
            log_error!(xc, "error:{:?}:{:?}", item.name, e);
            status.attributes_failed_to_compute += eval_expr_list.len();
            return status;
        }
    };
    let mut root = DataCell::Dyn(root);

    for expr in eval_expr_list {
        log_info!(xc, "computing expression {:?} for item {:?}", expr, item_name);
        if expr.eval_on_cell(&mut root, xc)
            .and_then(|v| output_expr_value(item_name, expr, &v, out, xc))
            .map(|_| { status.attributes_computed_ok += 1; })
            .or_else(|e| match e {
                Error::NotApplicable => {
                    status.attributes_not_applicable += 1;
                    log_warn!(xc, "warning:{:?}:{:?}:{:?}", item_name, expr, e);
                    Ok(())
                },
                Error::Output(oe) => {
                    status.output_error = true;
                    log_crit!(xc, "fatal:{:?}:{:?}:{:?}", item_name, expr, oe);
                    Err(())
                },
                _ => {
                    status.attributes_failed_to_compute += 1;
                    log_error!(xc, "error:{:?}:{:?}:{:?}", item_name, expr, e);
                    Ok(())
                }
            }).is_err() {
            break;
        }
    }
    status
}

fn parse_eval_expr_list<'a>(
    text: &str,
    xc: &mut ExecutionContext<'a>,
) -> Result<Vector<'a, Expr<'a>>, ExitCode> {
    let s = Source::new(text, "eval-expression-arg");
    let mut p = Parser::new(&s, &xc);
    p.parse_expr_list()
        .and_then(|x|
            p.expect_token(BasicTokenType::End.to_bitmap())
                .map(|_e| x.unwrap_data().unwrap_items()))
        .map_err(|e| {
            log_error!(xc, "error in expression: {}\nerror: {}", text, e.get_msg());
            ExitCode::new(64)
        })
}

/* run **********************************************************************/
fn run<'x>(
    invocation: &'x Invocation,
    out: &mut (dyn Write + '_),
    xc: &mut ExecutionContext<'x>
) -> Result<(), ExitCode> {
    if invocation.verbose {
        log_info!(xc, "lib: {}", halfbit::lib_name());
    }
    let mut summary = ProcessingStatus::new();
    let mut expressions = xc.vector();
    for expr_text in &invocation.expressions[..] {
        if let Err(ae) = expressions.append_vector(parse_eval_expr_list(expr_text.as_str(), xc)?) {
            log_error!(xc, "failed to allocate memory for parsing eval expressions: {:?}", ae);
            return Err(ExitCode::new(16));
        }
    }
    log_debug!(xc, "expressions: {:?}", expressions);

    for item in &invocation.items {
        summary.add(&process_item(item, expressions.as_slice(), out, xc));
        if summary.output_error {
            break;
        }
    }
    if invocation.verbose {
        log_info!(xc, "accessible items: {}", summary.accessible_items);
        log_info!(xc, "inaccessible items: {}", summary.inaccessible_items);
        log_info!(xc, "expressions computed ok: {}", summary.attributes_computed_ok);
        log_info!(xc, "expressions not applicable: {}", summary.attributes_not_applicable);
        log_info!(xc, "expressions failed to compute: {}", summary.attributes_failed_to_compute);
    }
    let rc = 0_u8
        | if summary.attributes_not_applicable != 0 { 1 } else { 0 }
        | if summary.attributes_failed_to_compute != 0 { 2 } else { 0 }
        | if summary.inaccessible_items != 0 { 4 } else { 0 }
        | if xc.get_logging_error_mask() != 0 { 8 } else { 0 }
        | 0_u8;

    if rc != 0 {
        log_error!(xc, "completed with errors");
    }
    ExitCode::new(rc).to_result()
}

/* main *********************************************************************/
fn main() {
    let invocation = process_args(std::env::args().collect());
    let a = Malloc::new();
    let err = stderr();
    let mut log = err.lock();
    let out = stdout();
    let mut out = out.lock();
    let mut xc = ExecutionContext::new(
        a.to_ref(),
        a.to_ref(),
        &mut log,
        if invocation.verbose { LogLevel::Debug } else { LogLevel::Warning },
    );
    run(&invocation, &mut out, &mut xc)
        .unwrap_or_else(|e| {
            log_debug!(xc, "* exiting with code {}", e.0);
            std::process::exit(e.0 as i32);
        });
}

