use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(code) => ExitCode::from(code),
    }
}

fn run(args: Vec<String>) -> Result<(), u8> {
    let parsed = parse(&args)?;
    let run = if parsed.cold {
        my_photos_lib::bench::run_baseline_cold
    } else {
        my_photos_lib::bench::run_baseline
    };
    let report = run(&parsed.root, parsed.count).map_err(|error| {
        eprintln!("{}", error.as_code());
        1
    })?;
    let text = format!("{}\n", serde_json::to_string_pretty(&report).expect("json"));
    print!("{text}");
    if let Some(output) = &parsed.output {
        if fs::write(output, &text).is_err() {
            eprintln!("output_failed");
            return Err(1);
        }
    }
    Ok(())
}

struct Parsed {
    root: PathBuf,
    count: u32,
    output: Option<PathBuf>,
    cold: bool,
}

fn parse(args: &[String]) -> Result<Parsed, u8> {
    let mut root = None;
    let mut count = None;
    let mut output = None;
    let mut cold = false;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--cold" {
            cold = true;
            index += 1;
            continue;
        }
        let Some(value) = args.get(index + 1) else {
            usage();
            return Err(2);
        };
        match args[index].as_str() {
            "--root" => root = Some(PathBuf::from(value)),
            "--count" => {
                let Ok(parsed) = value.parse::<u32>() else {
                    usage();
                    return Err(2);
                };
                count = Some(parsed);
            }
            "--output" => output = Some(PathBuf::from(value)),
            _ => {
                usage();
                return Err(2);
            }
        }
        index += 2;
    }
    match (root, count) {
        (Some(root), Some(count)) => Ok(Parsed {
            root,
            count,
            output,
            cold,
        }),
        _ => {
            usage();
            Err(2)
        }
    }
}

fn usage() {
    eprintln!("usage");
}
