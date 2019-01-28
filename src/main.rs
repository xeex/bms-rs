mod bms;

use bms::parser::BmsParser;
use std::{env, fs::File};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(r#"Usage: cargo run "/path/to/BMS/file""#);
        return;
    }

    let mut f = File::open(&args[1]).expect("File not found.");
    let bp = BmsParser;
    let bms = bp.parse(&mut f);

    println!("{:#?}", bms);
}
