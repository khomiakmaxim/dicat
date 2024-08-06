use prompt_parser::{Args, Command};

pub mod operation;
pub mod prompt_parser;

pub struct App;

impl App {
    pub fn start(args: Args) -> anyhow::Result<()> {
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
