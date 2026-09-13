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
    from: Option<f64>,
    to: f64,
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

    let to = to.ok_or("missing --to <servings>")?;
    if let Some(f) = from {
        if f <= 0.0 {
            return Err("--from must be greater than zero".to_string());
        }
    }

    Ok(Config { from, to, path })
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
            process(BufReader::new(file), config.from, config.to, &mut out)
        }
        None => {
            let stdin = io::stdin();
            process(stdin.lock(), config.from, config.to, &mut out)
        }
    }
}

/// Scales every line of `reader` and writes the result to `out`. If `from`
/// is not given, the first line is checked for a `Serves`/`Yield` header
/// before any output is produced; if neither is available, this fails
/// rather than silently passing the whole recipe through unscaled.
fn process<R: BufRead, W: Write>(
    reader: R,
    from: Option<f64>,
    to: f64,
    out: &mut W,
) -> io::Result<()> {
    let mut lines = reader.lines();
    let first_line = match lines.next() {
        Some(line) => line?,
        None => return Ok(()),
    };

    let from = match from {
        Some(f) => f,
        None => quantity::detect_serving_count(&first_line).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "no --from given and no Serves/Yield header found on the first line",
            )
        })?,
    };
    let factor = to / from;

    writeln!(out, "{}", scale_line(&first_line, factor))?;
    for line in lines {
        let line = line?;
        writeln!(out, "{}", scale_line(&line, factor))?;
    }
    out.flush()
}

fn print_usage() {
    eprintln!("usage: recipe-scale [--from <servings>] --to <servings> [FILE]");
    eprintln!();
    eprintln!("Reads a recipe (from FILE, or stdin if omitted) and prints it back");
    eprintln!("with every leading ingredient quantity scaled from --from servings");
    eprintln!("to --to servings. If --from is omitted, it is read from a");
    eprintln!("Serves/Yield header on the first line.");
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run_process(input: &str, from: Option<f64>, to: f64) -> io::Result<String> {
        let mut out = Vec::new();
        process(input.as_bytes(), from, to, &mut out)?;
        Ok(String::from_utf8(out).unwrap())
    }

    #[test]
    fn scales_with_explicit_from() {
        let result = run_process("2 cups flour\n1/2 tsp salt\n", Some(2.0), 4.0).unwrap();
        assert_eq!(result, "4 cups flour\n1 tsp salt\n");
    }

    #[test]
    fn detects_from_from_serves_header() {
        let result = run_process("Serves 4\n2 cups flour\n", None, 8.0).unwrap();
        assert_eq!(result, "Serves 4\n4 cups flour\n");
    }

    #[test]
    fn errors_without_from_or_header() {
        let err = run_process("2 cups flour\n", None, 4.0).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn parse_args_allows_missing_from() {
        let args: Vec<String> = vec!["--to".to_string(), "4".to_string()];
        let config = parse_args(&args).unwrap();
        assert_eq!(config.from, None);
        assert_eq!(config.to, 4.0);
    }

    #[test]
    fn parse_args_requires_to() {
        let args: Vec<String> = vec!["--from".to_string(), "4".to_string()];
        assert!(parse_args(&args).is_err());
    }
}
