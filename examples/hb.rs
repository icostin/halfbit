extern crate clap;
use halfbit::DataCell;
use halfbit::mm::Allocator;
use halfbit::mm::Malloc;
use halfbit::ExecutionContext;
use halfbit::io::stream::Stream;
use halfbit::io::stream::NULL_STREAM;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOError;

#[derive(Debug)]
struct Invocation {
    verbose: bool,
    items: Vec<String>,
    attributes: Vec<String>,
}

struct ToolError {
    exit_code: u8,
    msg: String
}

#[derive(Debug)]
enum AttrComputeError<'a> {
    UnknownAttribute,
    NotApplicable,
    IO(IOError<'a>),
}

impl<'a> std::fmt::Display for AttrComputeError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrComputeError::UnknownAttribute => write!(f, "unknown attribute"),
            AttrComputeError::NotApplicable => write!(f, "not applicable"),
            AttrComputeError::IO(x) => write!(f, "I/O error: {}", x),
        }
    }
}

struct Item<'a> {
    name: &'a str,
    stream: &'a mut (dyn Stream + 'a),
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
fn process_args(args: Vec<String>) -> Result<Invocation, ToolError> {
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

    if inv.verbose {
        eprintln!("cmd line: {:#?}", m);
        eprintln!("inv: {:#?}", inv);
    }

    Ok(inv)
}

fn extract_first_byte<'a, 'x>(
    item: &mut Item<'a>,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell, AttrComputeError<'x>> {

    item.stream.read_byte(xc)
    .map(|v| DataCell::U64(v as u64))
    .map_err(|e|
        if *e.get_data() == IOErrorCode::UnexpectedEnd {
            AttrComputeError::NotApplicable
        } else {
            AttrComputeError::IO(e)
        })
}

fn process_item_attribute<'a, 'x>(
    item: &mut Item<'a>,
    attr: &str,
    xc: &mut ExecutionContext<'x>,
) -> Result<DataCell, AttrComputeError<'x>> {
    match attr {
        "first_byte" => extract_first_byte(item, xc),
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
        eprintln!("processing {:?}", item_name);
    }
    let mut f = match std::fs::File::open(item_name) {
        Ok(f) => {
            status.accessible_items = 1;
            f
        },
        Err(e) => {
            eprintln!("error opening file {:?}: {}", item_name, e);
            return status;
        }
    };
    let mut item = Item {
        name: item_name,
        stream: &mut f,
    };

    for attr in &invocation.attributes {
        if invocation.verbose {
            eprintln!("computing attribute {:?} for item {:?}",
                      attr, item_name);
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
                        eprintln!("warning:{:?}:{:?}:{}", item.name, attr, e);
                    },
                    _ => {
                        status.attributes_failed_to_compute += 1;
                        eprintln!("error:{:?}:{:?}:{}", item.name, attr, e);
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
) -> Result<(), ToolError> {
    if invocation.verbose {
        println!("lib: {}", halfbit::lib_name());
    }
    let mut summary = ProcessingStatus::new();
    for item in &invocation.items {
        summary.add(&process_item(item, invocation, xc));
    }
    if invocation.verbose {
        println!("accessible items: {}", summary.accessible_items);
        println!("inaccessible items: {}", summary.inaccessible_items);
        println!("attributes computed ok: {}", summary.attributes_computed_ok);
        println!("attributes not applicable: {}", summary.attributes_not_applicable);
        println!("attributes failed to compute: {}", summary.attributes_failed_to_compute);
    }
    let rc = 0_u8
        | if summary.inaccessible_items != 0 { 4 } else { 0 }
        | if summary.attributes_failed_to_compute != 0 { 2 } else { 0 }
        | if summary.attributes_not_applicable != 0 { 1 } else { 0 }
        | 0_u8;

    if rc == 0 {
        Ok(())
    } else {
        Err(ToolError {
            exit_code: rc,
            msg: String::from("completed with errors"),
        })
    }
}

/* main *********************************************************************/
fn main() {
    process_args(std::env::args().collect())
    .and_then(|invocation| {
        let a = Malloc::new();
        let mut xc = ExecutionContext::new(a.to_ref(), a.to_ref(), NULL_STREAM.get());
        run(&invocation, &mut xc)
    })
    .unwrap_or_else(|e| {
        eprintln!("{}", e.msg);
        std::process::exit(e.exit_code as i32);
    });
}

