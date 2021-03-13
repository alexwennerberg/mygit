use askama::Template;
use git2::Repository;
use once_cell::sync::OnceCell;
use pico_args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;
use tide::prelude::*;
use tide::Request;

#[derive(Deserialize, Debug)]
pub struct Config {
    port: i32, // should be u8
    repo_directory: String,
    emoji_favicon: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn global() -> &'static Config {
        CONFIG.get().expect("Config is not initialized")
    }
}

#[derive(Template)]
#[template(path = "index.html")] // using the template in this path, relative
struct IndexTemplate<'a> {
    repos: Vec<Repository>,
    config: &'a Config,
}

async fn index(req: Request<()>) -> tide::Result {
    let config = &Config::global();
    let repos = &config.repo_directory;
    let paths = fs::read_dir(repos)?;
    let mut index_template = IndexTemplate {
        repos: vec![],
        config: config,
    }; // TODO replace with map/collect
    for path in paths {
        let repo = Repository::open(path?.path())?;
        index_template.repos.push(repo);
    }

    Ok(index_template.into())
}

#[derive(Template)]
#[template(path = "repo.html")] // using the template in this path, relative
struct RepoHomeTemplate<'a> {
    repo: Repository,
    readme_text: &'a str,
    config: &'a Config,
}

async fn repo_home(req: Request<()>) -> tide::Result {
    let config = &Config::global();
    let repo_path = Path::new(&config.repo_directory).join(req.param("repo_name")?);
    // TODO CLEAN PATH! VERY IMPORTANT! DONT FORGET!
    let repo = Repository::open(repo_path)?;
    let tmpl = RepoHomeTemplate {
        repo,
        readme_text: "Hello world",
        config,
    };
    Ok(tmpl.into())
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
    app.at("/robots.txt")
        .serve_file("templates/static/robots.txt")?; // TODO configurable
    app.at("/style.css")
        .serve_file("templates/static/style.css")?; // TODO configurable
    app.at("/:repo_name").get(repo_home);
    // ALSO do git pull at this url somehow ^
    // app.at("/:repo_name/commit/:hash").get(repo_log);
    // app.at("/:repo_name/log/:ref").get(repo_log); ref optional, default master/main
    // app.at("/:repo_name/tree/:ref").get(repo_log); ref = master/main when not present
    // app.at("/:repo_name/tree/:ref/item/:file").get(repo_log); ref = master/main when not present
    // app.at("/:repo_name/refs").get(repo_log); ref = master/main when not present
    // Bonus: raw files, patchsets
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
