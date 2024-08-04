use dicom::{dictionary_std::tags, object::open_file};
use jwalk::Parallelism;
use prettytable::{format, table};
use rayon::iter::{ParallelBridge, ParallelIterator};
use std::{
    borrow::Cow,
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    prompt_parser::options::{CatalogOptions, RestructOptions},
    Person,
};

// Relies on underneath paths being sorted as it's type invariant
pub(crate) struct SortedPaths(Vec<PathBuf>); // зробити його non-empty?

impl SortedPaths {
    // TODO: Це ок?
    fn from<I: Into<Vec<PathBuf>>>(paths: I) -> Self {
        let mut paths = paths.into();
        paths.sort();
        Self(paths)
    }
}

impl std::fmt::Display for SortedPaths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(paths) = self;

        if paths.is_empty() {
            write!(f, "")
        } else {
            let mut components = split_path_to_components(&paths[0]);
            let mut tab_count = 0;

            for comp in &components {
                let tabs = ".".repeat(tab_count);
                write!(f, "\n")?;
                write!(f, "{tabs}")?;
                write!(f, "|____")?;
                write!(f, "{comp}")?;

                tab_count += 1;
            }

            for path in paths.iter().skip(1) {
                //dbg!(&components);
                let new_path_components = split_path_to_components(path);
                //dbg!(&new_path_components);

                let mut same_components_count = 0;
                while same_components_count < new_path_components.len()
                    && same_components_count < components.len()
                {
                    if new_path_components[same_components_count]
                        != components[same_components_count]
                    {
                        break;
                    }
                    same_components_count += 1;
                }
                //dbg!(same_components_count);

                tab_count = same_components_count;
                for i in same_components_count..new_path_components.len() {
                    let comp = new_path_components[i].as_ref();

                    let tabs = ".".repeat(tab_count);
                    write!(f, "\n")?;
                    write!(f, "{tabs}")?;
                    write!(f, "|___")?;
                    write!(f, "{comp}")?;

                    tab_count += 1;
                }

                components = new_path_components;
                // //dbg!(&components);
            }

            write!(f, "")
        }
    }
}

pub fn catalog(options: CatalogOptions) -> Result<(), Box<dyn std::error::Error>> {
    let CatalogOptions {
        path,
        as_csv,
        keep_structure,
        person_id,
    } = options;

    if keep_structure {
        if as_csv {
        } else {
        }
    } else {
        let pairs: Vec<(Person, PathBuf)> = vec![];
        // TODO: Use sequential walkdir here

        let map = get_structure(path)?;
        if as_csv {
            unimplemented!();
        } else {
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
                // let tree = paths);

                let mut table = table!([FG -> id], [FG -> name], [paths]);
                table.set_format(table_format);
                table.printstd();
            }
        }
    }

    Ok(())
}

pub fn restruct(options: RestructOptions) -> Result<(), Box<dyn std::error::Error>> {
    let RestructOptions { path, person_id } = options;

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
                acc.entry(person).or_default().push(file); // тут ми маємо пушити всі файли зразу
                acc
            });

    // TODO: Неясно, скільки саме тут потоків буде
    let map_with_sorted_paths = map
        .into_iter()
        .par_bridge()
        .map(|(person, files)| {
            let paths = SortedPaths::from(files);
            (person, paths)
        })
        .collect();

    Ok(map_with_sorted_paths)
}

// TODO: Write documentation
// TODO: Think of different OSs here
fn split_path_to_components<'a, A>(path: &'a A) -> Vec<Cow<'a, str>>
where
    A: AsRef<Path> + ?Sized,
{
    let path = path.as_ref();
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect()
}

#[derive(Default)]
struct DirTree {
    nodes: Vec<(String, Vec<String>)>, // TODO: Може тут якось використовувати це OsString натомість???
}

impl std::fmt::Display for DirTree {
    // DFS/BFS???
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        write!(f, "{directory}")
    }
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
