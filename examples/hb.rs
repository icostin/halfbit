extern crate clap;

use std::io::stderr;

use halfbit::DataCell;
use halfbit::ExecutionContext;
use halfbit::LogLevel;
use halfbit::mm::Allocator;
use halfbit::mm::AllocError;
use halfbit::mm::Malloc;
use halfbit::mm::Vector;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOError;
use halfbit::io::IOPartialError;
use halfbit::io::stream::RandomAccessRead;
use halfbit::io::stream::SeekFrom;
use halfbit::log_debug;
use halfbit::log_info;
use halfbit::log_warn;
use halfbit::log_error;

#[derive(Debug)]
struct Invocation {
    verbose: bool,
    items: Vec<String>,
    attributes: Vec<String>,
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
fn process_args(args: Vec<String>) -> Invocation {
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
                .help("computes given attributes (comma separated)")
                .takes_value(true)
                .multiple(true)
                .require_delimiter(true))
        .get_matches_from(args);

    let inv = Invocation {
        verbose: m.is_present("verbose"),
        items:
            if let Some(values) = m.values_of("items") {
                values.map(|x| String::from(x)).collect()
            } else {
                Vec::new()
            },
        attributes:
            if let Some(values) = m.values_of("attr") {
                values.map(|x| String::from(x)).collect()
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
    .and_then(|_| item.stream.read_byte(xc))
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
    _item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell<'x>, AttrComputeError<'x>> {
    let ids: Vector<'x, DataCell> = Vector::new(xc.get_main_allocator());
    Ok(DataCell::CellVector(ids))
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
        | if summary.inaccessible_items != 0 { 4 } else { 0 }
        | if summary.attributes_failed_to_compute != 0 { 2 } else { 0 }
        | if summary.attributes_not_applicable != 0 { 1 } else { 0 }
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

