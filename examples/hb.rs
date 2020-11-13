extern crate clap;
use std::io::Read;
use std::fmt::Write;

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
enum AttrValue {
    Nothing,
    Bool(bool),
    U64(u64),
    I64(i64),
}
impl std::fmt::Display for AttrValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrValue::Nothing => {
                write!(f, "null")
            },
            AttrValue::Bool(v) => {
                write!(f, "{}", if *v { "true" } else { "false" })
            },
            AttrValue::U64(v) => {
                write!(f, "{}", v)
            },
            AttrValue::I64(v) => {
                write!(f, "{}", v)
            },
        }
    }
}

#[derive(Debug)]
enum AttrComputeError {
    UnknownAttribute,
    NotApplicable,
    IO(String),
}

impl std::convert::From<std::io::Error> for AttrComputeError {
    fn from(error: std::io::Error) -> Self {
        let mut msg = String::new();
        if write!(msg, "{}", error).is_err() {
            msg = String::from("failed");
        }
        AttrComputeError::IO(msg)
    }
}

impl std::fmt::Display for AttrComputeError {
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
    reader: std::io::BufReader<std::fs::File>,
}

enum ItemProcessingStatus {
    Success,
    InaccessibleItem,
    DoneWithErrors(isize),
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

fn extract_first_byte<'a>(
    item: &mut Item<'a>,
) -> Result<AttrValue, AttrComputeError> {
    let mut buf = [0u8, 1];
    let r = item.reader.read(&mut buf)?;
    if r == 0 {
        Err(AttrComputeError::NotApplicable)
    } else {
        Ok(AttrValue::U64(buf[0] as u64))
    }
}

fn process_item_attribute<'a>(
    item: &mut Item<'a>,
    attr: &str,
) -> Result<AttrValue, AttrComputeError> {
    match attr {
        "first_byte" => extract_first_byte(item),
        _ => Err(AttrComputeError::UnknownAttribute)
    }
}

fn process_item(
    item_name: &str,
    invocation: &Invocation,
) -> ItemProcessingStatus {
    if invocation.verbose {
        eprintln!("processing {:?}", item_name);
    }
    let f = std::fs::File::open(item_name);
    if f.is_err() {
        eprintln!("error opening file {:?}: {}", item_name, f.unwrap_err());
        return ItemProcessingStatus::InaccessibleItem;
    }
    let mut item = Item {
        name: item_name,
        reader: std::io::BufReader::new(f.unwrap()),
    };

    let mut error_count = 0isize;

    for attr in &invocation.attributes {
        if invocation.verbose {
            eprintln!("computing attribute {:?} for item {:?}",
                      attr, item_name);
        }
        match process_item_attribute(&mut item, attr) {
            Ok(av) => {
                println!("{:?}\t{}\t{}", item_name, attr, av);
            },
            Err(e) => {
            error_count += 1;
            eprintln!("error:{:?}:{:?}:{}", item.name, attr, e);
            },
        }
    }

    if error_count > 0 {
        ItemProcessingStatus::DoneWithErrors(error_count)
    } else {
        ItemProcessingStatus::Success
    }
}

/* run **********************************************************************/
fn run(invocation: &Invocation) -> Result<(), ToolError> {
    if invocation.verbose {
        println!("lib: {}", halfbit::lib_name());
    }
    let mut rc = 0u8;
    for item in &invocation.items {
        match process_item(item, invocation) {
            ItemProcessingStatus::Success => {},
            ItemProcessingStatus::InaccessibleItem => rc |= 2,
            ItemProcessingStatus::DoneWithErrors(_) => rc |= 1,
        }
    }
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
    let result = match process_args(std::env::args().collect()) {
        Ok(invocation) => run(&invocation),
        Err(te) => Err(te)
    };

    if let Err(e) = result {
        eprintln!("{}", e.msg);
        std::process::exit(e.exit_code as i32);
    }
}

