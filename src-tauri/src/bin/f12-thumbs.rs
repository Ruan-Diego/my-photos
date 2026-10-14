//! F12 local validation: makes the missing thumbnails of the library disk (F11) under
//! `.myphotos\thumbs\` and prints the summary. `f12-thumbs [small|medium|large]`.
//! Reads the originals only; writes thumbnails and the database (AD-008, AD-011).

use std::process::ExitCode;
use std::time::Instant;

use my_photos_lib::thumbs::{generate_default_library, ThumbSize};

fn main() -> ExitCode {
    let argument = std::env::args().nth(1).unwrap_or_else(|| "small".to_string());
    let Some(size) = ThumbSize::from_column(&argument) else {
        eprintln!("tamanho desconhecido: {argument} (small, medium ou large)");
        return ExitCode::from(2);
    };
    let started = Instant::now();
    let mut progress = |done: u64, queued: u64| eprintln!("{done}/{queued}");
    match generate_default_library(size, &mut progress) {
        Ok(summary) => {
            println!("{}", serde_json::to_string_pretty(&summary).expect("json"));
            eprintln!("{:.1}s", started.elapsed().as_secs_f64());
            ExitCode::SUCCESS
        }
        Err(code) => {
            eprintln!("{code}");
            ExitCode::from(1)
        }
    }
}
