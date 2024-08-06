use std::{
    borrow::Cow,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Represents a person's data in a DICOM file.
#[derive(Debug, Eq, Clone, PartialEq, Hash)]
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

// TODO: Currently this prints the directory sub-tree like this:
// --------------------
//  root
//  |__sub_dir1
//  .|__sub_sub_dir1
//  ..|__sub_sub_sub_dir1
//  |__sub_dir2
// --------------------
//  It would be easier to read if the  sub-directories were
// horizontally connected across the parent's directory like this:
// --------------------
//  root
//  |__sub_dir1
//  ||__sub_sub_dir1
//  |.|__sub_sub_sub_dir1
//  |__sub_dir2
// --------------------

impl std::fmt::Display for SortedPaths {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(paths) = self;

        if !paths.is_empty() {
            let mut components = split_path_to_components(&paths[0]);
            let mut indentation_amount = 1;

            let dir_name = &components[0];

            // Print the root
            write!(f, "{dir_name}")?;

            // Print first's path components, except for the root
            for comp in components.iter().skip(1) {
                write_path_component(f, comp, indentation_amount)?;
                indentation_amount += 1;
            }

            // Print the next paths, relying on the calculated indentation
            for path in paths.iter().skip(1) {
                let next_path_components = split_path_to_components(path);
                let common_prefix_len = next_path_components
                    .iter()
                    .zip(components.iter())
                    .take_while(|(a, b)| a == b)
                    .count();

                indentation_amount = common_prefix_len;
                for comp in next_path_components.iter().skip(common_prefix_len) {
                    write_path_component(f, comp, indentation_amount)?;
                    indentation_amount += 1;
                }

                components = next_path_components;
            }
        }

        Ok(())
    }
}

fn write_path_component(
    f: &mut std::fmt::Formatter<'_>,
    component: &Cow<'_, str>,
    indentation_amount: usize,
) -> std::fmt::Result {
    let dots = ".".repeat(indentation_amount);
    writeln!(f)?;
    write!(f, "{dots}")?;
    write!(f, "|__")?;
    write!(f, "{component}")?;
    Ok(())
}

fn split_path_to_components<A>(path: &A) -> Vec<Cow<'_, str>>
where
    A: AsRef<Path> + ?Sized,
{
    let path = path.as_ref();
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect()
}

pub mod errors {
    use std::path::PathBuf;

    pub type CliResult<T> = Result<T, CliError>;

    #[derive(thiserror::Error, Debug)]
    pub enum CliError {
        #[error("Directory {0} doesn't exist")]
        DirectoryDoesNotExist(PathBuf),
        #[error("Directory {0} doesn't contain valid .DICOM files")]
        FilesDoNotExist(PathBuf),
        #[error("Directory {0} doesn't contain valid .DICOM files for person's ID {1}")]
        FilesDoNotExistForPerson(PathBuf, String),
        #[error("{0} isn't a directory")]
        NotADirectory(PathBuf),
        #[error("Something went wrong")]
        GeneralError,
        #[error("Couldn't create {0} directory")]
        CreatingDirectoryError(PathBuf),
    }
}
