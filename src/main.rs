use tide::Request;
use askama::Template;
use tide::prelude::*;
use std::path::Path;
use std::time::Instant;
use once_cell::sync::OnceCell;
use std::fs;
use serde::{Serialize, Deserialize};
use pico_args;
use git2::Repository;

#[derive(Deserialize, Debug)]
pub struct Config {
    port: i32, // should be u8
    repo_directory: String
}

static CONFIG: OnceCell<Config> = OnceCell::new();

impl Config {
    pub fn global() -> &'static Config {
        CONFIG.get().expect("Config is not initialized")
    }

}

#[derive(Template)] 
#[template(path = "index.html")] // using the template in this path, relative
struct IndexTemplate { 
    repos: Vec<Repository>
}

async fn index(req: Request<()>) -> tide::Result {
    let repos = &Config::global().repo_directory;
    let paths = fs::read_dir(repos)?;
    let mut index_template = IndexTemplate{
        repos:  vec![]
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
    readme_text: &'a str
}

async fn repo_home(req: Request<()>) -> tide::Result {
    let repo_path = Path::new(&Config::global().repo_directory).join(req.param("repo_name")?);
    // TODO CLEAN PATH! VERY IMPORTANT! DONT FORGET!
    let repo = Repository::open(repo_path)?;
    let tmpl = RepoHomeTemplate {
        repo: repo,
        readme_text: "Hello world"
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
    app.at("/:repo_name").get(repo_home);
    app.at("/:repo_name/log").get(repo_log);
    // app.at("/:repo_name/:commit/tree").get(repo_log);
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
