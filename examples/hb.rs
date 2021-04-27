extern crate clap;

use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::fmt;
use core::fmt::Write as FmtWrite;

use std::io::stderr;
use std::io::stdout;
use std::string::String as StdString;

use halfbit::ExecutionContext;
use halfbit::LogLevel;
use halfbit::data_cell::DataCell;
use halfbit::data_cell::DataCellOps;
use halfbit::data_cell::DataCellOpsMut;
use halfbit::data_cell::Error;
use halfbit::data_cell::content_stream::ContentStream;
use halfbit::data_cell::eval::Eval;
use halfbit::data_cell::expr::BasicTokenType;
use halfbit::data_cell::expr::Expr;
use halfbit::data_cell::expr::Parser;
use halfbit::data_cell::expr::Source;
use halfbit::data_cell;
use halfbit::dyn_rc;
use halfbit::io::ErrorCode as IOErrorCode;
use halfbit::io::IOError;
use halfbit::io::stream::Write;
use halfbit::log_crit;
use halfbit::log_debug;
use halfbit::log_error;
use halfbit::log_info;
use halfbit::log_warn;
use halfbit::mm::Allocator;
use halfbit::mm::Malloc;
use halfbit::mm::Rc;
use halfbit::mm::Vector;

const HB_VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Copy, Clone, Debug)]
struct ExitCode(u8);

#[derive(Debug)]
struct Invocation {
    verbose: bool,
    item_paths: Vec<StdString>,
    item_raw_strings: Vec<StdString>,
    expressions: Vec<StdString>,
}

struct Item<'a> {
    name: &'a str,
    file: Rc<'a, RefCell<std::fs::File>>,
}

dyn_rc!(make_data_cell_ops_rc, DataCellOps);

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
        let mut x = self.file.as_ref().borrow_mut();
        let mut f: &mut std::fs::File = x.borrow_mut();
        let mut cs = ContentStream::new(&mut f);
        cs.get_property_mut(property_name, xc)
    }
}

/* process_args *************************************************************/
fn process_args(args: Vec<StdString>) -> Invocation {
    let m = clap::App::new("halfbit")
        .version(HB_VERSION)
        .author("by Costin Ionescu <costin.ionescu@gmail.com>")
        .about("examines given items and generates a report")
        .arg(clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("prints what it does verbosely"))
        .arg(clap::Arg::with_name("items")
                .help("item(s) to process (as file paths by default)")
                .multiple(true))
        .arg(clap::Arg::with_name("eval")
                .short("e")
                .long("eval")
                .help("computes given comma-separated expressions on each item")
                .takes_value(true)
                .multiple(true)
                .number_of_values(1))
        .arg(clap::Arg::with_name("raw_string")
                .short("r")
                .long("raw-string")
                .help("treat following arguments as file content for items")
                .takes_value(true)
                .multiple(true))
        .arg(clap::Arg::with_name("file_path")
                .short("p")
                .long("file-path")
                .help("treat following arguments as file paths for items"))
        .after_help("
Item properties:
    first_byte          first content byte
    first_8_bytes       byte array with first 8 bytes (or entire content if shorter)
    tof_ids             array of identifiers with matching top-of-file exact data formats
    elf_header          treat content as ELF file header record
")
        .setting(clap::AppSettings::ArgRequiredElseHelp)
        .get_matches_from(args);

    let inv = Invocation {
        verbose: m.is_present("verbose"),
        item_paths:
            if let Some(values) = m.values_of("items") {
                values.map(|x| StdString::from(x)).collect()
            } else {
                Vec::new()
            },
        item_raw_strings:
            m.values_of("raw_strings")
                .map_or_else(
                    || Vec::new(),
                    |v| v.map(|x| StdString::from(x)).collect()),
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
        .and_then(|_| value.output_as_human_readable(out, xc))
        .and_then(|_| out.write_all(b"\n", xc).map_err(|e| Error::Output(e.to_error())))
}

fn process_item<'x>(
    item_name: &'x str,
    eval_expr_list: &[Expr<'x>],
    out: &mut (dyn Write + '_),
    xc: &mut ExecutionContext<'x>,
) -> ProcessingStatus {
    let mut status = ProcessingStatus::new();

    log_info!(xc, "info:{:?}: evaluating {:?}", item_name, eval_expr_list);
    let f = match std::fs::File::open(item_name) {
        Ok(f) => {
            status.accessible_items = 1;
            f
        },
        Err(e) => {
            log_error!(xc, "error:{:?}: open file failed: {}", item_name, e);
            return status;
        }
    };

    let item = Item {
        name: item_name,
        file: match xc.rc(RefCell::new(f)) {
            Ok(f) => f,
            Err((e, _f)) => {
                log_error!(xc, "error:{:?}: {}", item_name, e);
                status.attributes_failed_to_compute += eval_expr_list.len();
                return status;
            }
        }
    };
    let root = match xc.rc(item) {
        Ok(b) => make_data_cell_ops_rc(b),
        Err((e, item)) => {
            log_error!(xc, "error:{:?}: {}", item.name, e);
            status.attributes_failed_to_compute += eval_expr_list.len();
            return status;
        }
    };
    let mut root = DataCell::Dyn(root);

    for expr in eval_expr_list {
        log_info!(xc, "info:{:?}: computing expression {}", item_name, expr);
        if expr.eval_on_cell(&mut root, xc)
            .and_then(|v| output_expr_value(item_name, expr, &v, out, xc))
            .map(|_| { status.attributes_computed_ok += 1; })
            .or_else(|e| match e {
                Error::NotApplicable => {
                    status.attributes_not_applicable += 1;
                    log_warn!(xc, "warning:{:?}:{}: {}", item_name, expr, e);
                    Ok(())
                },
                Error::Output(oe) => {
                    status.output_error = true;
                    log_crit!(xc, "fatal:{:?}:{}: {}", item_name, expr, oe);
                    Err(())
                },
                _ => {
                    status.attributes_failed_to_compute += 1;
                    log_error!(xc, "error:{:?}:{}: {}", item_name, expr, e);
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

    for item in &invocation.item_paths {
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

