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
        #[arg(long)]
        pub person_id: Option<String>, // TODO, якщо все встигатиму, то можна тут пару тіпів приймати буде через якийсь розділювач, навіть
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
        #[arg(long)]
        pub person_id: Option<String>, // TODO винести в спільні | в Павла є на прикладі, як таке можна робити
    }
}
