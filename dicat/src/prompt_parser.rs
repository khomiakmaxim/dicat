use clap::{command, Parser};
use options::{CatalogOptions, RestructOptions};

#[derive(Parser)]
#[command(version, about)]
/// Command line utility for cataloging files of DICOM standard
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
#[command(version, about)]
pub enum Command {
    /// Catalog DICOM files in the directory and print the result to the stdout
    Catalog(CatalogOptions),
    /// Create a new directory with a restructured hierarchy based on person IDs, as in a catalog output
    Restruct(RestructOptions),
}

pub(crate) mod options {
    use std::{ffi::OsString, path::PathBuf};

    #[derive(clap::Args)]
    pub struct RestructOptions {
        /// Path to the directory, which will be restructured
        #[arg(short, long)]
        pub path: PathBuf,
        /// Person IDs(separated by `,`), which DICOM files will be restructured in a new directory
        #[arg(long, value_delimiter = ',')]
        pub ids: Option<Vec<OsString>>,
    }

    #[derive(clap::Args)]
    pub struct CatalogOptions {
        #[arg(short, long)]
        /// Path to the directory, which fiels will be viewed in a catalog format(default format can be overwritten by specifying `--keep-structure` flag)
        pub path: PathBuf,
        #[arg(short, long)]
        /// Print names, IDs, and paths of DICOM files in a directory in .CSV format, preserving the original directory hierarchy
        pub as_csv: bool,
        /// Person IDs(separated by `,`), which DICOM files will be viewed in a catalog format
        #[arg(long, value_delimiter = ',')]
        pub ids: Option<Vec<OsString>>,
    }
}
