use nl_compiler::from_vast;
use safety_equiv::{build_miter, miter_to_cnf};
use safety_net::Netlist;
use safety_pass::Cell;

use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn read_netlist(path: &Path) -> Result<Rc<Netlist<Cell>>, Box<dyn Error>> {
    let text = fs::read_to_string(path)?;

    let defines = HashMap::new();
    let includes: Vec<PathBuf> = Vec::new();

    let (ast, _) =
        sv_parser::parse_sv_str(&text, path.to_path_buf(), &defines, &includes, true, false)?;

    let mut netlists = from_vast::<Cell>(&ast)?;

    if netlists.len() != 1 {
        return Err(std::io::Error::other("expected exactly one module in each file").into());
    }

    Ok(netlists.remove(0))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("usage: cnf_demo <gold.v> <gate.v>");
        std::process::exit(1);
    }

    let gold = read_netlist(Path::new(&args[1]))?;
    let gate = read_netlist(Path::new(&args[2]))?;

    let miter = build_miter(&gold, &gate)?;
    let cnf = miter_to_cnf(&miter)?;

    print!("{}", cnf.to_dimacs());

    Ok(())
}
