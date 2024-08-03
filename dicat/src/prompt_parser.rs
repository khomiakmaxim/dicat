use clap::{command, Parser};
use options::{CatalogOptions, RestructOptions};

#[derive(Parser, Debug)]
#[command(version, about)]
/// dicat is a command line utility for cataloging files of DICOM standard
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Parser, Debug, Clone)]
#[command(version, about)]
pub enum Command {
    /// Catalogs DICOM files in the folder and prints the result to the stdout
    Catalog(CatalogOptions),
    // TODO: Write more text here
    /// Builds a new hierarchy with restructured structure as in the catalog
    Resturct(RestructOptions),
}

pub(crate) mod options {
    use std::path::PathBuf;

    #[derive(clap::Args, Debug, Clone)]
    pub struct RestructOptions {
        // NB: In the future, it would be reasonable to define and use struct with common options via #[flatten]
        pub path: PathBuf,
        #[arg(short, long)]
        pub zip: bool, // TODO: Подумати за це , бо воно якось тупо виглядає
        #[arg(conflicts_with = "zip")]
        pub zip_only: bool,
    }

    #[derive(clap::Args, Debug, Clone)]
    pub struct CatalogOptions {
        #[arg(short, long)]
        /// Path to the directory
        pub path: PathBuf,
        /// When used, changes the output to be in the .csv format
        #[arg(long)] // TODO: Це те саме, що без нічого?
        pub as_csv: bool,
        #[arg(long)]
        /// Keeps the original directory hierarchy unchanged
        pub keep_structure: bool,
    }
}
