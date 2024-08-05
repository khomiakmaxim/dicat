use clap::{command, Parser};
use options::{CatalogOptions, RestructOptions};

#[derive(Parser)]
#[command(version, about)]
/// 'dicat' is a command line utility for cataloging files of DICOM standard
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser)]
#[command(version, about)]
pub enum Command {
    /// Catalog DICOM files in the directory and print the result to the stdout
    Catalog(CatalogOptions),
    /// Create a new directory with restructured structure as in a catalog
    Restruct(RestructOptions),
}

pub(crate) mod options {
    use std::path::PathBuf;

    #[derive(clap::Args)]
    pub struct RestructOptions {
        /// Path to the directory, which will be restructured
        pub path: PathBuf,
        /// Person IDs(separated by `,`), which DICOM files will be restructured in a new directory
        #[arg(long, value_delimiter = ',')]
        pub ids: Option<Vec<String>>,
    }

    #[derive(clap::Args)]
    pub struct CatalogOptions {
        #[arg(short, long)]
        /// Path to the directory, which fiels will be viewed in a catalog format(default format can be overwritten by specifying `--keep-structure` flag)
        pub path: PathBuf,
        #[arg(short, long)]
        /// Keep the original directory hierarchy unchanged
        pub keep_structure: bool,
        /// Person IDs(separated by `,`), which DICOM files will be viewed in a catalog format
        #[arg(long, value_delimiter = ',')]
        pub ids: Option<Vec<String>>, // TODO: Think of using OsString here
    }
}
