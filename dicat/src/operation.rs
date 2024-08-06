use dicom::{dictionary_std::tags, object::open_file};
use jwalk::Parallelism;
use prettytable::{format, table};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::OsString,
    path::{Path, PathBuf},
};

use crate::prompt_parser::options::{CatalogOptions, RestructOptions};
use utils::{Person, SortedPaths};

fn traverse_dir_print_csv<A: AsRef<Path>>(
    path: A,
    person_ids: Option<Vec<OsString>>,
) -> anyhow::Result<()> {
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

pub fn catalog(options: CatalogOptions) -> anyhow::Result<()> {
    let CatalogOptions { path, as_csv, ids } = options;

    if as_csv {
        traverse_dir_print_csv(path, ids)?;
    } else {
        // Taken from github/examples...
        dbg!(1);
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

        let map = get_structure(path, ids)?;

        for (person, paths) in map {
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

pub fn restruct(options: RestructOptions) -> anyhow::Result<()> {
    // const MAX_SEMAPHORE_PERMITS: usize = 100; // Було б дуже файно то додати собі ще

    let RestructOptions { path, ids } = options;

    let structure = get_structure(path, ids)?;

    if !structure.is_empty() {
        dbg!(structure.len());
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(async move {
                // let mut semaphore = Semaphore::new(MAX_SEMAPHORE_PERMITS);

                let mut handles = vec![];
                let folder_name = PathBuf::from(format!(
                    "test_restructured{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                ));

                tokio::fs::create_dir(&folder_name).await.unwrap();

                for (person, sorted_paths) in structure {
                    // let permit = semaphore.acquire().await.unwrap();

                    let mut persons_dir = folder_name.clone();
                    handles.push(tokio::task::spawn(async move {
                        // let name = "some_random";

                        persons_dir.push(&person.id);
                        dbg!(&persons_dir);
                        tokio::fs::create_dir(&persons_dir).await.unwrap();

                        // println!("\n{}\n\n, {}", person.id, sorted_paths);

                        let sorted_paths = sorted_paths.into_inner();

                        for path in sorted_paths {
                            let filename = path.file_name().unwrap().to_string_lossy();

                            let mut new_path = PathBuf::from(&persons_dir);
                            new_path.push(filename.as_ref());

                            tokio::fs::copy(path, new_path).await.unwrap();
                        }

                        // drop(permit);
                    }));
                }

                futures::future::join_all(handles).await;
                println!("Restructured files into {}", folder_name.to_string_lossy());
            });
    }

    Ok(())
}

// Would it be smart to write to files here as well?
// TODO: Think of someting way more efficient here
fn get_structure(
    path: PathBuf,
    person_ids: Option<Vec<OsString>>,
) -> anyhow::Result<HashMap<Person, SortedPaths>> {
    let person_ids = person_ids.unwrap_or_default();
    let person_ids: HashSet<Cow<'_, str>> =
        person_ids.iter().map(|x| x.to_string_lossy()).collect();

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

                let patient_id = obj.element(tags::PATIENT_ID).unwrap().to_str().unwrap();
                if person_ids.is_empty() || person_ids.contains(patient_id.as_ref()) {
                    let file_name = path.as_os_str().to_str().unwrap();
                    let patient_name = obj.element(tags::PATIENT_NAME).unwrap().to_str().unwrap();

                    let person = Person {
                        name: patient_name.to_string().into(),
                        id: patient_id.to_string().into(),
                    };

                    Some((person, PathBuf::from(file_name)))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let map: HashMap<Person, Vec<PathBuf>> =
        v.into_iter()
            .fold(HashMap::new(), |mut acc, (person, file)| {
                acc.entry(person).or_default().push(file); // тут ми маємо пушити всі файли зразу
                acc
            });

    // TODO: Неясно, скільки саме тут потоків буде
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

// impl DirTree {
//     fn new() -> Self {
//         Self { nodes: vec![] }
//     }

//     fn with_capacity(capacity: usize) -> Self {
//         Self {
//             nodes: Vec::with_capacity(capacity),
//         }
//     }
// }

// impl std::fmt::Dis

// На маці воно б мало спрацювати також
// fn get_dir_tree(paths: SortedPaths) -> Result<DirTree, Box<dyn std::error::Error>> {
//     if paths.is_empty() {
//         eprintln!("Sorted paths shouldn't be empty");
//         Ok(DirTree::new())
//     } else {
//         let SortedPaths(paths) = paths;

//         // Keeps indexes of [`String`] nodes in the [`DirTree`] inner vector
//         let mut indexes: HashMap<PathBuf, usize> = HashMap::with_capacity(paths.len());
//         let mut dir_tree = DirTree::with_capacity(paths.len());

//         let mut cached_path = paths[0].clone();
//         let mut cached_components = split_path_to_components(&paths[0]);

//         if cached_components.is_empty() {
//             // TODO: Introduce custom errors? this_error crate
//             return Err("Path should be splittable".into());
//         }

//         let mut tree_node_ind = 0_usize;
//         let mut ancestor: Option<&str> = None;
//         let mut cached_paths = PathBuf::from(cached_components[0].as_ref());

//         for component in &cached_components {
//             cached_path.push(component.into());
//             // hash(dir/file1/file/2) -> number in Vec, which preserves the initial structure
//             indexes.insert(cached_path, tree_node_ind);
//             tree_node_ind += 1;

//             let adjacent_nodes = if let Some(ancestor) = ancestor {
//                 vec![ancestor.to_string()]
//             } else {
//                 vec![]
//             };

//             // component — `file1`
//             dir_tree.nodes.push((component.to_string(), adjacent_nodes));
//             ancestor = Some(component);
//         }

//         // At this point `indexes` map for `dir/file1/file2/1.dcm` would look like this: `[dir, file1, file2, 1.dcm]`
//         //                                                                                |     |     |       |
//         //                                                                                0,    1,    2,      3

//         for path in paths.iter().skip(1) {
//             let cached_path = PathBuf::new();
//             let path_slice: Cow<'_, str> = path.to_string_lossy();
//             let mut valid_cached_ind = 0;

//             for cc in &cached_components {
//                 cached_path.push(cc.into());
//                 let bytes_left_on_slice = path_slice.len() - valid_cached_ind;

//                 if bytes_left_on_slice >= cc.len() {
//                     let component_from_slice =
//                         &path_slice[valid_cached_ind..(valid_cached_ind + cc.len())];
//                     if component_from_slice == *cc {
//                         // Additional check
//                         if component_from_slice.chars().last().unwrap() == std::path::MAIN_SEPARATOR
//                         {
//                             // All good, we can continue using our cache
//                             valid_cached_ind += 1;
//                         } else {
//                             break;
//                         }
//                     } else {
//                         break;
//                     }
//                 } else {
//                     break;
//                 }
//             }

//             let another_slice = path_slice.as_ref();
//             let subslice = &another_slice[valid_cached_ind..];
//             let remaining_components = split_path_to_components(subslice);

//             // let ancestor = cached_components.last().unwrap();
//             let mut node_ind = *indexes.get(cached_path.as_ref()).unwrap();

//             // Окей, тут ми вже маємо почати вставляти нові вузли в наше дерево
//             // The first node will already be in the dir_tree

//             // cahced_path - dir/file1/file2/
//             for rc in &remaining_components {
//                 cached_path.push(rc.into());
//                 // встановили батьку, що ми його син
//                 dir_tree.nodes[node_ind].1.push(rc.clone().to_string());
//                 node_ind += 1;
//                 indexes.insert(cached_path.to_string_lossy().into(), node_ind);

//                 dir_tree.nodes.push((rc.clone().to_string(), vec![]));
//             }

//             // Оновити cached_components
//             let mut new_cached_components = vec![];
//             new_cached_components.extend_from_slice(&cached_components[..=valid_cached_ind]); // TODO: `clones` Нам би тут зробити якийсь move гарний
//             new_cached_components
//         }

//         Ok(dir_tree)
//     }
// }

pub mod utils {
    use std::{
        borrow::Cow,
        ffi::OsString,
        path::{Path, PathBuf},
    };

    /// Represents a person's data in a DICOM file.
    #[derive(Debug, Eq, PartialEq, Hash)]
    pub struct Person {
        pub name: OsString,
        pub id: OsString,
    }

    /// Relies on inner paths being sorted in the topological order as its invariant.
    /// ## Usage
    /// **Example**
    /// ```
    /// use std::path::PathBuf;
    /// use dicat::operation::utils::SortedPaths;
    ///
    /// let vec = vec![PathBuf::from("drive/db/a.txt"), PathBuf::from("drive/da/b.txt")];
    /// let sorted_paths = SortedPaths::from(vec);
    /// let inner = sorted_paths.into_inner();
    /// assert_eq!(vec![PathBuf::from("drive/da/b.txt"), PathBuf::from("drive/db/a.txt")], inner);
    /// ````    
    /// **Example**
    /// ```compile_fail
    /// use std::path::PathBuf;
    /// use dicat::operation::utils::SortedPaths;
    ///
    /// let vec = vec![PathBuf::from("drive/db/a.txt"), PathBuf::from("drive/da/b.txt")];
    /// // line below doesn't compile
    /// let sorted_paths = SortedPaths(vec![PathBuf::from("drive/db/a.txt"), PathBuf::from("drive/da/b.txt")]);
    /// ````   
    pub struct SortedPaths(Vec<PathBuf>);

    impl SortedPaths {
        /// Creates [`SortedPaths`] new type and sorts the inner collection of [`paths`] in the topological order.
        pub fn new<I: Into<Vec<PathBuf>>>(paths: I) -> Self {
            let mut paths = paths.into();
            paths.sort();
            Self(paths)
        }

        pub fn into_inner(self) -> Vec<PathBuf> {
            self.0
        }
    }

    impl std::fmt::Display for SortedPaths {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            let Self(paths) = self;

            if !paths.is_empty() {
                // First traversal for finding start and end of vertical lines

                // let mut components =
            }

            Ok(())
        }
    }

    // TODO: Це треба гарненько переписати собі
    // impl std::fmt::Display for SortedPaths {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         let Self(paths) = self;

    //         if !paths.is_empty() {
    //             let mut components = split_path_to_components(&paths[0]);
    //             let mut current_point_count = 0;

    //             let dir_name = &components[0];

    //             // write!(f, "|")?;
    //             write!(f, "{dir_name}")?;

    //             for comp in components.iter().skip(1) {
    //                 let tabs = ".".repeat(current_point_count);
    //                 writeln!(f)?;
    //                 write!(f, "{tabs}")?;
    //                 write!(f, "|____")?;
    //                 write!(f, "{comp}")?;

    //                 current_point_count += 1;
    //             }

    //             for path in paths.iter().skip(1) {
    //                 let next_path_components = split_path_to_components(path);

    //                 let common_prefix_len = next_path_components
    //                     .iter()
    //                     .zip(components.iter())
    //                     .take_while(|(a, b)| a == b)
    //                     .count();

    //                 current_point_count = common_prefix_len;
    //                 for comp in next_path_components.iter().skip(common_prefix_len) {
    //                     let tabs = ".".repeat(current_point_count);
    //                     writeln!(f)?;
    //                     write!(f, "{tabs}")?;
    //                     write!(f, "|____")?;
    //                     write!(f, "{comp}")?;

    //                     current_point_count += 1;
    //                 }

    //                 components = next_path_components;
    //             }
    //         }

    //         Ok(())
    //     }
    // }

    fn split_path_to_components<A>(path: &A) -> Vec<Cow<'_, str>>
    where
        A: AsRef<Path> + ?Sized,
    {
        let path = path.as_ref();
        path.components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect()
    }
}
