use tide::Request;
use tide::prelude::*;
use std::time::Instant;
use once_cell::sync::OnceCell;
use std::fs;
use serde::{Serialize, Deserialize};
use pico_args;

#[derive(Deserialize, Debug)]
pub struct Config {
    port: i32 // should be u8
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn global() -> &'static Config {
        CONFIG.get().expect("Config is not initialized")
    }

}

async fn index(req: Request<()>) -> tide::Result<String> {
    let res = "Hello world!".to_owned();
    Ok(res)
}

const HELP: &str = "\
mygit

FLAGS:
  -h, --help            Prints help information
OPTIONS:
  -c                    Path to config file
";

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    // TODO cli
 
    let toml_text = fs::read_to_string("mygit.toml")?;
    let config: Config = toml::from_str(&toml_text)?;
    CONFIG.set(config).unwrap();

    tide::log::start();
    let mut app = tide::new();
    app.at("/").get(index);
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
