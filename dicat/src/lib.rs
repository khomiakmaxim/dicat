use prompt_parser::{Args, Command};
use utils::errors::CliResult;

pub mod operation;
pub mod prompt_parser;
mod utils;

pub use utils::errors;

pub struct App;

impl App {
    pub fn start(args: Args) -> CliResult<()> {
        let Args { command } = args;
        match command {
            Command::Catalog(catalog_options) => {
                operation::catalog(catalog_options)?;
            }
            Command::Restruct(restruct_options) => {
                operation::restruct(restruct_options)?;
            }
        }

        Ok(())
    }
}
