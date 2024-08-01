use std::borrow::Cow;

use dicom::{dictionary_std::tags, object::open_file};
use walkdir::WalkDir;

// use async_walkdir::WalkDir;
// use futures_lite::future::block_on;
// use futures_lite::stream::StreamExt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for entry in WalkDir::new("test_dicom_files") {
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
        let patient_clinic = match obj.element(tags::INSTITUTION_NAME) {
            Ok(inner) => inner.to_str()?,
            Err(_) => Cow::Borrowed("No clinic name"),
        };
        let patient_id = obj.element(tags::PATIENT_ID)?.to_str()?;

        println!("------------------------------------------");
        println!(
            "{}, {}, {}, {}",
            file_name, patient_name, patient_clinic, patient_id
        );
    }

    Ok(())
}
