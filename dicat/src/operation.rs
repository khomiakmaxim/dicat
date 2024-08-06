use dicom::{dictionary_std::tags, object::open_file};
use prettytable::{format, table};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    errors::{CliError, CliResult},
    prompt_parser::options::{CatalogOptions, RestructOptions},
    utils::{Person, SortedPaths},
};

fn traverse_sequentially_and_print_csv<A: AsRef<Path>>(
    path: A,
    person_ids: Option<Vec<OsString>>,
) -> CliResult<()> {
    let walkdir = walkdir::WalkDir::new(path.as_ref());

    let person_ids = person_ids.unwrap_or_default();
    let person_ids: HashSet<Cow<'_, str>> =
        person_ids.iter().map(|x| x.to_string_lossy()).collect();

    println!("Full Name,ID,Path");
    for entry in walkdir {
        let dir_entry = entry.ok().unwrap();
        if dir_entry.file_type().is_file() {
            let path = dir_entry.path();

            let Ok(obj) = open_file(path) else {
                continue;
            };

            let patient_id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap();
            if person_ids.is_empty() || person_ids.contains(patient_id.as_ref()) {
                let patient_name = obj.element(tags::PATIENT_NAME).unwrap().to_str().unwrap();

                println!("{},{},{}", patient_name, patient_id, path.to_string_lossy());
            }
        }
    }

    Ok(())
}

/// Prints 
pub fn catalog(options: CatalogOptions) -> Result<(), CliError> {
    let CatalogOptions { path, as_csv, ids } = options;

    let start_time = std::time::SystemTime::now();
    if as_csv {
        // TODO: It would be reasonable to log inner errors as well
        traverse_sequentially_and_print_csv(path, ids).map_err(|_err| CliError::GeneralError)?;
    } else {
        // Taken from <https://github.com/phsym/prettytable-rs/blob/4d66e6ebddcd52b641369042b68959ad323d9ad0/examples/formatting.rs#L75>
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

        let catalog = scaffold_catalog(path, ids)?;

        for (person, paths) in catalog {
            const NOT_LISTED: &str = "[NOT LISTED]";
            let Person { name, id } = person;

            let name = if name.is_empty() {
                NOT_LISTED.into()
            } else {
                name
            };

            let id = if id.is_empty() { NOT_LISTED.into() } else { id };

            let id = format!("ID: {}", id.to_string_lossy());
            let name = format!("Full Name: {}", name.to_string_lossy());

            let mut table = table!([FG -> id], [FG -> name], [paths]);
            table.set_format(table_format);
            table.printstd();
        }
    }

    let end_time = std::time::SystemTime::now();
    let duration = end_time
        .duration_since(start_time)
        .expect("Time went backwards");

    println!("Catalog bench: {:?}", duration);

    Ok(())
}

/// Asynchronously in [`num_tasks`] tokio tasks copies .DICOM files into a new `dicat_(timestamp)/(person.id)` directory.
async fn copy_files_in_tasks(
    file_map: HashMap<Person, Vec<PathBuf>>,
    num_tasks: usize,
    root_path: &Path,
) {
    let mut task_handles = Vec::with_capacity(num_tasks);
    let mut chunk_sizes = vec![0; num_tasks];

    let files_amount = file_map
        .iter()
        .fold(0, |acc, (_person, paths)| acc + paths.len());

    let files_per_task = files_amount / num_tasks;
    let remainder = files_amount % num_tasks;

    // Distribute file chunks evenly between tasks
    for i in 0..num_tasks {
        chunk_sizes[i] = files_per_task + if i < remainder { 1 } else { 0 };
    }

    let mut pairs_iter = file_map
        .into_iter()
        .flat_map(|(to, from)| from.into_iter().zip(std::iter::repeat(to)));

    // Spawn new `tokio` task per chunk of paths and copy them to newely created locations
    for chunk_size in chunk_sizes {
        let root_path_owned = PathBuf::from(root_path);
        let files_to_copy: Vec<(PathBuf, Person)> = pairs_iter.by_ref().take(chunk_size).collect();

        let handle = tokio::spawn(async move {
            for (path_buf, person) in files_to_copy {
                let mut persons_path = root_path_owned.clone();
                persons_path.push(&person.id);

                let filename = path_buf.file_name().unwrap().to_string_lossy(); // TODO: Remove unwrap()
                let filename_pathbuf = PathBuf::from(filename.as_ref());
                persons_path.push(&filename_pathbuf);

                tokio::fs::copy(&path_buf, &persons_path).await.unwrap(); // TODO: Remove unwrap()
            }
        });
        task_handles.push(handle);
    }

    futures::future::join_all(task_handles).await;
}

pub fn restruct(options: RestructOptions) -> CliResult<()> {
    // Amount of tasks spawned for asynchronous copying. Has been picked experimentally at this moment.
    // On 'SK hynix PC601 HFS512GD9TNG-L2A0A' SSD less than 4 tasks occupy < 100% of possible throughput
    const TASKS_AMOUNT: usize = 4;

    let RestructOptions { path, ids } = options;

    // TODO: Add loading animation and more logs here
    // println!("Started restructuring...");
    let structure = scaffold_catalog(path, ids)?;
    let structure: HashMap<Person, Vec<PathBuf>> = structure
        .into_iter()
        .map(|(x, paths)| (x, paths.into_inner()))
        .collect();

    if !structure.is_empty() {
        // let start_time = std::time::SystemTime::now();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create `root` directory
        let new_root = format!("dicat_{}", timestamp);
        let new_root_path = PathBuf::from(new_root);

        std::fs::create_dir(&new_root_path)
            .map_err(|_| CliError::CreatingDirectoryError(PathBuf::from(&new_root_path)))?;

        // For each person, create `root/person_id` directory
        for (person, _) in &structure {
            let mut persons_path = new_root_path.clone();
            persons_path.push(PathBuf::from(&person.id));
            std::fs::create_dir(&persons_path)
                .map_err(|_| CliError::CreatingDirectoryError(PathBuf::from(&persons_path)))?;
        }

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async move {
                // Copy corresponding .DICOM files into newely created directories
                copy_files_in_tasks(structure, TASKS_AMOUNT, &new_root_path).await;
            });

        // let end_time = std::time::SystemTime::now();
        // let duration = end_time
        //     .duration_since(start_time)
        //     .expect("Time went backwards");

        // println!("Restruct bench: {:?}", duration);
    }

    Ok(())
}

/// For a given [`path`], traverse the directory in parallel threads and scaffold
/// a `catalog-like` structure made of valid .DICOM files, based on the IDs of patients.
fn scaffold_catalog(
    path: PathBuf,
    patients_id: Option<Vec<OsString>>,
) -> CliResult<HashMap<Person, SortedPaths>> {
    if !path.is_dir() {
        return Err(CliError::NotADirectory(path));
    }

    let patients_id = patients_id.unwrap_or_default();
    let patients_id: HashSet<Cow<'_, str>> =
        patients_id.iter().map(|x| x.to_string_lossy()).collect();

    // <https://github.com/byron/jwalk>
    // Iterate over directory tree in parallel and accummulates (Person, PathBuf) pairs
    // for valid .DICOM files
    let v: Vec<(Person, PathBuf)> = jwalk::WalkDir::new(path)
        .into_iter()
        .par_bridge()
        .filter_map(|dir_entry| {
            let Ok(dir_entry) = dir_entry else {
                // TODO: Add warning logs here
                return None;
            };

            if dir_entry.file_type().is_file() {
                let path = dir_entry.path();

                // <https://docs.rs/dicom/latest/dicom/>
                let Ok(obj) = open_file(&path) else {
                    // TODO: Add Logs
                    return None;
                };

                let id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap(); // TODO: Remove unwrap
                if patients_id.is_empty() || patients_id.contains(id.as_ref()) {
                    let file_name = path.as_os_str().to_string_lossy();
                    let patient_name = obj.element(tags::PATIENT_NAME).unwrap().to_str().unwrap(); // TODO: Remove unwrap()

                    let person = Person {
                        name: patient_name.to_string().into(),
                        id: id.to_string().into(),
                    };

                    Some((person, PathBuf::from(file_name.as_ref())))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // TODO: Refactor 2 statements below

    // Merge results obtained from parallel threads
    let map: HashMap<Person, Vec<PathBuf>> =
        v.into_iter()
            .fold(HashMap::new(), |mut acc, (person, file)| {
                acc.entry(person).or_default().push(file);
                acc
            });
    
    // Sort paths in topological order for each patient
    let map_with_sorted_paths = map
        .into_iter()
        .par_bridge()
        .map(|(person, files)| {
            let paths = SortedPaths::new(files);
            (person, paths)
        })
        .collect();

    Ok(map_with_sorted_paths)
}
