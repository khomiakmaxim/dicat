use clap::Parser;
use std::{borrow::Cow, path::PathBuf};

/// Application which catalogs DICOM files
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    path: PathBuf,
    // TODO: Add meaningful subcommands as well
}

use dicom::{dictionary_std::tags, object::open_file};
use walkdir::WalkDir;

// use async_walkdir::WalkDir;
// use futures_lite::future::block_on;
// use futures_lite::stream::StreamExt;

#[allow(unreachable_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args { path } = Args::parse();

    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();

        if entry.file_type().is_dir() {
            continue;
        }

        let path = entry.path();
        let Ok(obj) = open_file(path) else {
            continue;
        };

        let file_name = path.as_os_str().to_str().unwrap();
        let patient_name = obj.element(tags::PATIENT_NAME)?.to_str()?;
        let patient_id = obj.element(tags::PATIENT_ID)?.to_str()?;

        let patient_clinic = match obj.element(tags::INSTITUTION_NAME) {
            Ok(inner) => inner.to_str()?,
            Err(_) => Cow::Borrowed("No clinic name"),
        };

        println!("------------------------------------------");
        println!(
            "{}, {}, {}, {}",
            file_name, patient_name, patient_clinic, patient_id
        );
    }

    Ok(())
}
