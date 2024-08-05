use prompt_parser::{Args, Command};

pub mod operation;
pub mod prompt_parser;

pub struct App;

impl App {
    pub fn start(args: Args) -> Result<(), Box<dyn std::error::Error>> {
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
