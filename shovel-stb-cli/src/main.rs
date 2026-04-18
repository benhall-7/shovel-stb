use std::ffi::OsStr;
use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use shovel_stb::Stb;

#[derive(Parser)]
#[command(
    name = "stb",
    about = "Convert Shovel Knight .stb spreadsheets to/from CSV"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Input file (auto-detects direction from extension)
    #[arg(global = true)]
    input: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Command {
    /// Convert an STB file to CSV
    ToCsv {
        /// Input .stb file
        input: PathBuf,
        /// Output .csv file (defaults to <input>.csv)
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Print CSV to stdout instead of writing a file
        #[arg(long)]
        stdout: bool,
        /// Omit the UTF-8 BOM from the CSV output
        #[arg(long)]
        no_bom: bool,
    },
    /// Convert a CSV file to STB
    ToStb {
        /// Input .csv file
        input: PathBuf,
        /// Output .stb file (defaults to <input>.stb)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Some(Command::ToCsv {
            input,
            output,
            stdout,
            no_bom,
        }) => stb_to_csv(&input, output.as_deref(), stdout, !no_bom),
        Some(Command::ToStb { input, output }) => csv_to_stb(&input, output.as_deref()),
        None => {
            let input = cli.input.ok_or("no input file provided (try --help)")?;

            match input.extension().and_then(OsStr::to_str) {
                Some("stb" | "stm") => stb_to_csv(&input, None, false, true),
                Some("csv") => csv_to_stb(&input, None),
                Some(ext) => {
                    Err(format!("unknown extension .{ext} (expected .stb, .stm, or .csv)").into())
                }
                None => Err("input file has no extension (expected .stb, .stm, or .csv)".into()),
            }
        }
    }
}

fn stb_to_csv(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
    to_stdout: bool,
    bom: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let stb = Stb::open(input).map_err(|e| format!("failed to parse {}: {e}", input.display()))?;

    if to_stdout {
        stb.write_csv(std::io::stdout().lock(), bom)?;
    } else {
        let out = match output {
            Some(path) => path.to_owned(),
            None => {
                let mut name = input.as_os_str().to_owned();
                name.push(".csv");
                PathBuf::from(name)
            }
        };
        stb.save_csv(&out, bom)?;
        eprintln!("{} -> {}", input.display(), out.display());
    }

    Ok(())
}

fn csv_to_stb(
    input: &std::path::Path,
    output: Option<&std::path::Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    let stb =
        Stb::open_csv(input).map_err(|e| format!("failed to parse {}: {e}", input.display()))?;

    let out = match output {
        Some(path) => path.to_owned(),
        None => {
            let stripped = input.with_extension("");
            let has_binary_ext = stripped
                .extension()
                .and_then(OsStr::to_str)
                .is_some_and(|e| e == "stb" || e == "stm");
            if has_binary_ext {
                stripped
            } else {
                stripped.with_extension("stb")
            }
        }
    };

    stb.save_stb(&out)
        .map_err(|e| format!("failed to write {}: {e}", out.display()))?;

    eprintln!("{} -> {}", input.display(), out.display());

    Ok(())
}
