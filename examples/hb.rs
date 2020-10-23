extern crate clap;

struct Invocation {
    verbose: bool,
}

struct ToolError {
    exit_code: u8,
    msg: String
}

/* process_args *************************************************************/
fn process_args(args: Vec<String>) -> Result<Invocation, ToolError> {
    let m = clap::App::new("halfbit")
        .version("0.0")
        .author("Costin Ionescu <costin.ionescu@gmail.com>")
        .about("examines given items and generates report")
        .arg(clap::Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("prints what it does verbosely"))
        .arg(clap::Arg::with_name("ITEMS")
                .help("item(s) to process (file paths for now)")
                .multiple(true))
        .get_matches_from(args);
    println!("cmd line: {:?}", m);

    Ok(Invocation {
        verbose: m.is_present("verbose"),
    })
}

/* run **********************************************************************/
fn run(invocation: &Invocation) -> Result<(), ToolError> {
    if invocation.verbose {
        println!("lib: {}", halfbit::lib_name());
    }
    Err(ToolError {
        exit_code: 1,
        msg: String::from("not implemented"),
    })
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

