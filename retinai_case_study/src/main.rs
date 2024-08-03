use clap::Parser;
use dicom::{dictionary_std::tags, object::open_file};
use jwalk::Parallelism;
use rayon::prelude::*;
use std::{collections::HashMap, path::PathBuf};

/// Application which catalogs DICOM files
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[arg(short, long)]
    path: PathBuf,
    // TODO: Add meaningful subcommands as well
}

#[derive(Debug, Hash, Eq, PartialEq)]
struct Person {
    name: String,
    id: String,
}

struct App;

impl App {
    fn start(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let v: Vec<(Person, PathBuf)> = jwalk::WalkDir::new(path)
            .parallelism(Parallelism::RayonNewPool(8))
            .into_iter()
            .par_bridge()
            .filter_map(|dir_entry_result| {
                let dir_entry = dir_entry_result.ok()?;
                if dir_entry.file_type().is_file() {
                    let path = dir_entry.path();

                    let Ok(obj) = open_file(&path) else {
                        return None;
                    };

                    let file_name = path.as_os_str().to_str().unwrap();
                    let patient_name = obj.element(tags::PATIENT_NAME).unwrap().to_str().unwrap();
                    let patient_id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap();

                    let person = Person {
                        name: patient_name.into(),
                        id: patient_id.into(),
                    };

                    Some((person, PathBuf::from(file_name)))
                } else {
                    None
                }
            })
            .collect();

        let mut map: HashMap<Person, Vec<PathBuf>> =
            v.into_iter()
                .fold(HashMap::new(), |mut acc, (person, file)| {
                    acc.entry(person).or_default().push(file);
                    acc
                });

        map.par_iter_mut().for_each(|(_, files)| {
            files.sort();
        });

        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args { path } = Args::parse();
    App::start(path)?;

    Ok(())
}
