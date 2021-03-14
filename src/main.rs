use anyhow::Result;
use askama::Template;
use git2::{Commit, Repository};
use once_cell::sync::OnceCell;
use pico_args;
use pulldown_cmark::{html, Options, Parser};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str;
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
    repo: &'a Repository,
    readme_text: &'a str,
    config: &'a Config,
}

fn repo_from_request(repo_name: &str) -> Result<Repository> {
    let repo_path = Path::new(&Config::global().repo_directory).join(repo_name);
    // TODO CLEAN PATH! VERY IMPORTANT! DONT FORGET!
    let r = Repository::open(repo_path)?;
    Ok(r)
}

async fn repo_home(req: Request<()>) -> tide::Result {
    let config = &Config::global();
    let repo = repo_from_request(&req.param("repo_name")?)?;
    let readme = &repo.revparse_single("HEAD:README.md")?; // TODO allow more incl plaintext
    let markdown_input = str::from_utf8(readme.as_blob().unwrap().content())?;
    let mut options = Options::empty();
    let parser = Parser::new_ext(markdown_input, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    let tmpl = RepoHomeTemplate {
        repo: &repo,
        readme_text: &html_output,
        config,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "log.html")] // using the template in this path, relative
struct RepoLogTemplate<'a> {
    repo: &'a Repository,
    config: &'a Config,
    commits: Vec<Commit<'a>>,
}

async fn repo_log(req: Request<()>) -> tide::Result {
    let config = &Config::global();
    let repo = repo_from_request(&req.param("repo_name")?)?;
    let mut revwalk = repo.revwalk()?;
    match req.param("ref") {
        Ok(r) => revwalk.push_ref(&format!("refs/heads/{}", r))?,
        _ => revwalk.push_head()?,
    };
    let commits = revwalk
        .map(|oid| repo.find_commit(oid.unwrap()).unwrap().clone()) // TODO error handling
        .collect();
    let tmpl = RepoLogTemplate {
        repo: &repo,
        config,
        commits,
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
    app.at("/:repo_name/log").get(repo_log);
    app.at("/:repo_name/log/:ref").get(repo_log); // ref optional
                                                  // app.at("/:repo_name/tree/:ref").get(repo_log); ref = master/main when not present
                                                  // app.at("/:repo_name/tree/:ref/item/:file").get(repo_log); ref = master/main when not present
                                                  // app.at("/:repo_name/refs").get(repo_log); ref = master/main when not present
                                                  // Bonus: raw files, patchsets
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
