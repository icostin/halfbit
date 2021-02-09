extern crate clap;

use core::fmt::Write as FmtWrite;
use core::fmt::Debug;
use core::fmt::Formatter;
use core::fmt::UpperHex;
use core::fmt::Display;
use std::string::String as StdString;
use std::io::stderr;

use halfbit::ExecutionContext;
use halfbit::LogLevel;
use halfbit::mm::Allocator;
use halfbit::mm::AllocError;
use halfbit::mm::Malloc;
use halfbit::mm::Vector;
use halfbit::mm::String as HbString;
//use halfbit::mm::Vector as HbVector;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOPartialError;
use halfbit::io::IOPartialResult;
use halfbit::io::stream::RandomAccessRead;
use halfbit::io::stream::SeekFrom;
use halfbit::conv::int_be_decode;
use halfbit::log_debug;
use halfbit::log_info;
use halfbit::log_warn;
use halfbit::log_error;

use halfbit::data_cell::DataCell;
use halfbit::data_cell::DynDataCell;
use halfbit::data_cell::DataCellOps;
//use halfbit::data_cell::DataCellOpsExtra;
use halfbit::data_cell::AttrComputeError;
use halfbit::data_cell::expr::Source;
use halfbit::data_cell::expr::Parser;
use halfbit::data_cell::expr::Expr;
use halfbit::data_cell::expr::BasicTokenType;
//use halfbit::data_cell::expr::ParseError;
use halfbit::data_cell::Eval;

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
    stream: &'a mut (dyn RandomAccessRead + 'a),
}
struct ItemCell<'a, 'b> {
    item: &'b mut Item<'a>
}

struct ProcessingStatus {
    accessible_items: usize,
    inaccessible_items: usize,
    attributes_computed_ok: usize,
    attributes_not_applicable: usize,
    attributes_failed_to_compute: usize,
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

impl<'a, 'b> Debug for ItemCell<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "ItemCell({:?})", self.item.name)
    }
}
impl<'a, 'b> Display for ItemCell<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt("todo", f)
    }
}
impl<'a, 'b> UpperHex for ItemCell<'a, 'b> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        Display::fmt("todo", f)
    }
}
impl<'a, 'b> DataCellOps for ItemCell<'a, 'b> {
    fn type_name(&self) -> &'static str {
        "stream_data"
    }
    fn compute_attr<'d, 'x, 'o> (
        &mut self,
        attr_name: &str,
        xc: &mut ExecutionContext<'x>
    ) -> Result<DataCell<'o>, AttrComputeError<'x>>
    where Self: 'd, 'd: 'o, 'x: 'o {
        log_debug!(xc, "item queried for {:?}", attr_name);
        match attr_name {
            "first_byte" => extract_first_byte(self.item, xc),
            "first_8_bytes" => first_8_bytes(self.item, xc),
            "tof_ids" => identify_top_of_file_records(self.item, xc),
            "elf_header" => elf_header(self.item, xc),
            _ => Err(AttrComputeError::UnknownAttribute)
        }
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

fn process_item<'a, 'x>(
    item_name: &str,
    eval_expr_list: &[Expr<'a>],
    xc: &mut ExecutionContext<'x>,
) -> ProcessingStatus {
    let mut status = ProcessingStatus::new();

    log_info!(xc, "processing {:?}: evaluating {:?}", item_name, eval_expr_list);
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
    let root: DynDataCell = match xc.to_box(ItemCell{ item: &mut item }) {
        Ok(b) => b.into(),
        Err((e, cell)) => {
            log_error!(xc, "error:{:?}:{:?}", cell.item.name, e);
            status.attributes_failed_to_compute += eval_expr_list.len();
            return status;
        }
    };
    let mut root = DataCell::Dyn(root);

    for expr in eval_expr_list {
        log_info!(xc, "computing expression {:?} for item {:?}", expr, item_name);
        match expr.eval_on_cell(&mut root, xc) {
            Ok(av) => {
                println!("{:?}\t{}\t{}", item_name, expr, av);
                status.attributes_computed_ok += 1;
            },
            Err(e) => {
                match e {
                    AttrComputeError::NotApplicable => {
                        status.attributes_not_applicable += 1;
                        log_warn!(xc, "warning:{:?}:{:?}:{}", item_name, expr, e);
                    },
                    _ => {
                        status.attributes_failed_to_compute += 1;
                        log_error!(xc, "error:{:?}:{:?}:{}", item_name, expr, e);
                    }
                }
            },
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
fn run(
    invocation: &Invocation,
    xc: &mut ExecutionContext<'_>
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
        summary.add(&process_item(item, expressions.as_slice(), xc));
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
    let mut xc = ExecutionContext::new(
        a.to_ref(),
        a.to_ref(),
        &mut log,
        if invocation.verbose { LogLevel::Debug } else { LogLevel::Warning },
    );
    run(&invocation, &mut xc)
    .unwrap_or_else(|e| {
        log_debug!(xc, "* exiting with code {}", e.0);
        std::process::exit(e.0 as i32);
    });
}

