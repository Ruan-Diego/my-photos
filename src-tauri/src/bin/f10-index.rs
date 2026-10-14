//! F10 local validation: indexes `E:\` into `E:\.myphotos\library.db` and prints the summary.
//! Reads the library only; writes only the database (AD-011).

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut progress = |progress: my_photos_lib::index::Progress| {
        eprintln!("{:?} {}/{}", progress.stage, progress.done, progress.total);
    };
    match my_photos_lib::index::index_default_library(&mut progress) {
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
