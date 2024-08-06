use dicom::{dictionary_std::tags, object::open_file};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use prettytable::{format, table};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::OsString,
    fmt::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    errors::{CliError, CliResult},
    prompt_parser::options::{CatalogOptions, RestructOptions},
    utils::{Person, SortedPaths},
};

/// Catalogs DICOM files in the directory and prints the result to the stdout.
pub fn catalog(options: CatalogOptions) -> CliResult<()> {
    let CatalogOptions { path, as_csv, ids } = options;

    if as_csv {
        // TODO: Add  Logging of the inner unrepresentable errors
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

        // Get the structure, which can be printed
        let catalog = scaffold_catalog(path, ids)?;

        // Print the structure using `prettytable::table!`
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

    Ok(())
}

/// Creates a new directory with restructured structure for each patient, which contains patient's files directly.
pub fn restruct(options: RestructOptions) -> CliResult<()> {
    // Amount of tasks spawned for asynchronous copying. Has been picked experimentally at this moment.
    // On 'SK hynix PC601 HFS512GD9TNG-L2A0A' SSD less than 4 tasks occupy < 100% of possible throughput
    const TASKS_AMOUNT: usize = 4;
    let RestructOptions { path, ids } = options;
    let catalog = scaffold_catalog(path, ids)?;
    let catalog: HashMap<Person, Vec<PathBuf>> = catalog
        .into_iter()
        .map(|(x, paths)| (x, paths.into_inner()))
        .collect();

    if !catalog.is_empty() {
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
        for person in catalog.keys() {
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
                // asynchrnously in `TASK_AMOUNT` tasks
                copy_files_in_tasks(catalog, TASKS_AMOUNT, &new_root_path).await;
            });
    }

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
    for (i, chunk) in chunk_sizes.iter_mut().enumerate().take(num_tasks) {
        *chunk = files_per_task + if i < remainder { 1 } else { 0 };
    }

    let mut pairs_iter = file_map
        .into_iter()
        .flat_map(|(to, from)| from.into_iter().zip(std::iter::repeat(to)));

    // Spawn new `tokio` task per chunk of paths and copy them to newely created locations
    for chunk_size in chunk_sizes {
        let root_path_owned = PathBuf::from(root_path);
        let files_to_copy: Vec<(PathBuf, Person)> = pairs_iter.by_ref().take(chunk_size).collect();

        let handle = tokio::spawn(async move {
            let files_in_chunk = files_to_copy.len();

            for (path_buf, person) in files_to_copy {
                let mut persons_path = root_path_owned.clone();
                persons_path.push(&person.id);

                let filename = path_buf.file_name().unwrap().to_string_lossy(); // TODO: Remove unwrap()
                let filename_pathbuf = PathBuf::from(filename.as_ref());
                persons_path.push(&filename_pathbuf);

                tokio::fs::copy(&path_buf, &persons_path).await.unwrap(); // TODO: Remove unwrap()
            }

            files_in_chunk
        });
        task_handles.push(handle);
    }

    println!("Restructuring...");
    let mut files_copied = 0;

    // Progress bar for better user experience
    let pb = ProgressBar::new(files_amount as u64);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{wide_bar:.blue}] ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap()
            })
            .progress_chars("#>-"),
    );

    for handle in task_handles {
        let files_added = handle.await.unwrap();

        while files_copied < files_amount {
            files_copied += files_added;
            // Sleeping for better user experience while rendering the progress bar for small
            // directories. It is beningn, since won't block any thread
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    println!("Restructured into '{}'", root_path.to_string_lossy());
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

                let id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap(); // TODO: Remove unwrap()
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

fn traverse_sequentially_and_print_csv<A: AsRef<Path>>(
    path: A,
    person_ids: Option<Vec<OsString>>,
) -> CliResult<()> {
    let path = path.as_ref();
    let walkdir = walkdir::WalkDir::new(path);

    let person_ids = person_ids.unwrap_or_default();
    let person_ids: HashSet<Cow<'_, str>> =
        person_ids.iter().map(|x| x.to_string_lossy()).collect();

    let mut print_headers = true;
    for entry in walkdir {
        let Ok(entry) = entry else {
            return Err(CliError::GeneralError);
        };

        if entry.file_type().is_file() {
            let path = entry.path();

            let Ok(obj) = open_file(path) else {
                // TODO: Log warn
                continue;
            };

            let patient_id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap(); // TODO: Remove unwrap()
            if person_ids.is_empty() || person_ids.contains(patient_id.as_ref()) {
                if print_headers {
                    println!("Name,ID,Path");
                    print_headers = false;
                }
                let patient_name = obj.element(tags::PATIENT_NAME).unwrap().to_str().unwrap(); // TODO: Remove unwrap()
                println!("{},{},{}", patient_name, patient_id, path.to_string_lossy());
            }
        }
    }

    if print_headers {
        Err(CliError::FilesDoNotExist(path.into()))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaffold_catalog() -> CliResult<()> {
        let pb = PathBuf::from("test_small_dir");
        let scaffolded_catalog = scaffold_catalog(pb, None)?;

        let mut expected: HashMap<Person, SortedPaths> = HashMap::new();

        let p1 = Person {
            name: "".into(),
            id: "98.12.21".into(),
        };

        let p2 = Person {
            name: "CMB-GEC-MSB-06857".into(),
            id: "CMB-GEC-MSB-06857".into(),
        };

        expected.insert(
            p1,
            SortedPaths::new(vec![
                PathBuf::from("test_small_dir/56364404.dcm"),
                PathBuf::from("test_small_dir/56364403.dcm"),
            ]),
        );
        expected.insert(
            p2,
            SortedPaths::new(vec![
                PathBuf::from("test_small_dir/1-013.dcm"),
                PathBuf::from("test_small_dir/1-012.dcm"),
                PathBuf::from("test_small_dir/1-011.dcm"),
                PathBuf::from("test_small_dir/1-010.dcm"),
            ]),
        );

        assert_eq!(scaffolded_catalog, expected);
        Ok(())
    }

    #[test]
    fn test_scaffold_catalog_with_ids() -> CliResult<()> {
        let pb = PathBuf::from("test_small_dir");
        let scaffolded_catalog = scaffold_catalog(pb, Some(vec!["98.12.21".into()]))?;

        let mut expected: HashMap<Person, SortedPaths> = HashMap::new();

        let p1 = Person {
            name: "".into(),
            id: "98.12.21".into(),
        };

        expected.insert(
            p1,
            SortedPaths::new(vec![
                PathBuf::from("test_small_dir/56364404.dcm"),
                PathBuf::from("test_small_dir/56364403.dcm"),
            ]),
        );

        assert_eq!(scaffolded_catalog, expected);
        Ok(())
    }
}
