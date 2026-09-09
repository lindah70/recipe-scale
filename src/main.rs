//! recipe-scale answers one question: given a recipe written for N servings,
//! what do the quantities look like for M servings?
//!
//! Input is read and written one line at a time so an arbitrarily large
//! recipe file (or a stream piped in over stdin) never has to sit in memory
//! all at once - only the current line does.

mod quantity;

use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::process;

struct Config {
    factor: f64,
    path: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut from: Option<f64> = None;
    let mut to: Option<f64> = None;
    let mut path: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--from" => {
                i += 1;
                let v = args.get(i).ok_or("--from needs a value")?;
                from = Some(v.parse().map_err(|_| "--from must be a number")?);
            }
            "--to" => {
                i += 1;
                let v = args.get(i).ok_or("--to needs a value")?;
                to = Some(v.parse().map_err(|_| "--to must be a number")?);
            }
            other => {
                if path.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                path = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    let from = from.ok_or("missing --from <servings>")?;
    let to = to.ok_or("missing --to <servings>")?;
    if from <= 0.0 {
        return Err("--from must be greater than zero".to_string());
    }

    Ok(Config {
        factor: to / from,
        path,
    })
}

fn scale_line(line: &str, factor: f64) -> String {
    match quantity::parse_leading_quantity(line) {
        Some((value, rest)) => format!("{}{}", quantity::format_quantity(value * factor), rest),
        None => line.to_string(),
    }
}

fn run(config: Config) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    match config.path {
        Some(path) => {
            let file = File::open(&path)?;
            process(BufReader::new(file), config.factor, &mut out)
        }
        None => {
            let stdin = io::stdin();
            process(stdin.lock(), config.factor, &mut out)
        }
    }
}

fn process<R: BufRead, W: Write>(reader: R, factor: f64, out: &mut W) -> io::Result<()> {
    for line in reader.lines() {
        let line = line?;
        writeln!(out, "{}", scale_line(&line, factor))?;
    }
    out.flush()
}

fn print_usage() {
    eprintln!("usage: recipe-scale --from <servings> --to <servings> [FILE]");
    eprintln!();
    eprintln!("Reads a recipe (from FILE, or stdin if omitted) and prints it back");
    eprintln!("with every leading ingredient quantity scaled from --from servings");
    eprintln!("to --to servings.");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();

    let config = match parse_args(&args) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("recipe-scale: {msg}");
            print_usage();
            process::exit(1);
        }
    };

    if let Err(err) = run(config) {
        eprintln!("recipe-scale: {err}");
        process::exit(1);
    }
}
