use dicom::{dictionary_std::tags, object::open_file};
use jwalk::Parallelism;
use prettytable::{format, table};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{collections::HashMap, path::PathBuf};

use crate::{
    prompt_parser::options::{CatalogOptions, RestructOptions},
    Person,
};

struct SortedPaths(Vec<PathBuf>);

pub fn catalog(options: CatalogOptions) -> Result<(), Box<dyn std::error::Error>> {
    let CatalogOptions {
        path,
        as_csv,
        keep_structure,
    } = options;

    if as_csv {
        todo!()
    } else {
        let map = get_structure(path)?;
        let table_format = format::FormatBuilder::new()
            .column_separator('│')
            .borders('│')
            .separators(
                &[format::LinePosition::Top],
                format::LineSeparator::new('─', '┬', '┌', '┐'),
            )
            .separators(
                &[format::LinePosition::Intern],
                format::LineSeparator::new('─', '┼', '├', '┤'),
            )
            .separators(
                &[format::LinePosition::Bottom],
                format::LineSeparator::new('─', '┴', '└', '┘'),
            )
            .padding(1, 1)
            .build();

        for (person, paths) in map {
            let Person { name, id } = person;   

            let name = if name.is_empty() {
                "(NOT LISTED)".to_string() // const?
            } else {
                name
            };

            let id = format!("ID: {}", id);
            let name = format!("Full Name: {}", name);
            // let person_info = format!("ID({}) {}", id, name);

            // let hierarchy =  

            // Зробити собі щось таке тут
            let directory = r#"
path:       
|_one:      
|  |__two.d  .............................................................
|  |__1.dcm .................................................
|_two.dcm   
|_twoi.dcm  ,,,,,,,,,,,,,,,,,,,,,,,,,,
|_x:        
| |_c.dcm  ............................. 
|_c.dcm     
    "#;

            let mut table = table!([FG -> id], [FG -> name], [directory]);
            table.set_format(table_format);
            table.printstd();
        }
    }

    Ok(())
}

pub fn restruct(options: RestructOptions) -> Result<(), Box<dyn std::error::Error>> {
    let RestructOptions {
        path,
        zip,
        zip_only,
    } = options;

    let structure = get_structure(path)?;
    // Now, crate a new directory with the corresponding folders and paths

    // TODO: In the future I'd like to extend this to clinics as well????

    Ok(())
}

// Would it be smart to write to files here as well?

// TODO: Think of someting way more efficient here
fn get_structure(
    path: PathBuf,
) -> Result<HashMap<Person, SortedPaths>, Box<dyn std::error::Error>> {
    let v: Vec<(Person, PathBuf)> = jwalk::WalkDir::new(path)
        .parallelism(Parallelism::RayonNewPool(8)) // TODO: move this out to config
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry_result| {
            // Чи могли б ми тут якось надсилати повідомлення в канал, який малює інформацію про прогрес?

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

    let map: HashMap<Person, Vec<PathBuf>> =
        v.into_iter()
            .fold(HashMap::new(), |mut acc, (person, file)| {
                acc.entry(person).or_default().push(file);
                acc
            });

    let map_with_sorted_paths = map.into_iter().par_bridge().map(|(person, mut files)| {
        files.sort();
        let paths = SortedPaths(files);
        (person, paths)
    }).collect();

    Ok(map_with_sorted_paths)
}

// TODO: Write documentation 
fn get_dir_tree(paths: SortedPaths) -> String {
    todo!()
}