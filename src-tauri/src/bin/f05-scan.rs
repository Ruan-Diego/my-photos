//! F05 local validation: scans `E:\` into `E:\.myphotos\library.db` and prints the summary.
//! Reads the library only; writes only the database (AD-011).

use std::process::ExitCode;

fn main() -> ExitCode {
    match my_photos_lib::scan::scan_default_library() {
        Ok(summary) => {
            println!("{}", serde_json::to_string_pretty(&summary).expect("json"));
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(1)
        }
    }
}
