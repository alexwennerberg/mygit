use anyhow::Result;
use askama::Template;
use git2::{Commit, Reference, Repository, Tree, TreeEntry};
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
    let markdown_input = std::str::from_utf8(readme.as_blob().unwrap().content())?;
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
        .filter_map(|oid| repo.find_commit(oid.unwrap()).ok().clone()) // TODO error handling
        .take(100) // Only get first 100 commits
        .collect();
    let tmpl = RepoLogTemplate {
        repo: &repo,
        config,
        commits,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "refs.html")] // using the template in this path, relative
struct RepoRefTemplate<'a> {
    repo: &'a Repository,
    config: &'a Config,
    branches: Vec<Reference<'a>>,
    tags: Vec<Reference<'a>>,
}
async fn repo_refs(req: Request<()>) -> tide::Result {
    let config = &Config::global();
    let repo = repo_from_request(&req.param("repo_name")?)?;
    let branches = repo
        .references()?
        .filter_map(|x| x.ok())
        .filter(|x| x.is_branch())
        .collect();
    let tags = repo
        .references()?
        .filter_map(|x| x.ok())
        .filter(|x| x.is_tag())
        .collect();
    let tmpl = RepoRefTemplate {
        repo: &repo,
        config,
        branches,
        tags,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "tree.html")] // using the template in this path, relative
struct RepoTreeTemplate<'a> {
    repo: &'a Repository,
    config: &'a Config,
    tree: Tree<'a>,
}
async fn repo_tree(req: Request<()>) -> tide::Result {
    // TODO handle subtrees
    let config = &Config::global();
    let repo = repo_from_request(&req.param("repo_name")?)?;
    // TODO accept reference or commit id
    let commit = match req.param("ref") {
        _ => repo.revparse_single("HEAD")?.peel_to_commit()?,
    };
    let tree = commit.tree()?;
    let tmpl = RepoTreeTemplate {
        repo: &repo,
        config,
        tree,
    };
    Ok(tmpl.into())
}

mod filters {
    pub fn iso_date(i: &i64) -> ::askama::Result<String> {
        // UTC date
        let datetime: chrono::DateTime<chrono::Utc> =
            chrono::DateTime::from_utc(chrono::NaiveDateTime::from_timestamp(*i, 0), chrono::Utc);
        Ok(datetime.format("%Y-%m-%d").to_string())
    }

    pub fn unix_perms(m: &i32) -> ::askama::Result<String> {
        let mut m = *m;
        // manually wrote this bc I couldn't find a library
        // acting like I'm writing C for fun
        // TODO -- symlinks?
        // https://unix.stackexchange.com/questions/450480/file-permission-with-six-bytes-in-git-what-does-it-mean
        if m == 0o040000 {
            // is directory
            return Ok("d---------".to_owned());
        }
        let mut output: [u8; 10] = [0; 10]; // ascii string
        let mut i = 9;
        for _ in 0..3 {
            // Go backwards here
            for c in &[0x78, 0x77, 0x72] {
                // xrw
                if m % 2 == 1 {
                    output[i] = *c;
                } else {
                    output[i] = 0x2d; // -
                }
                m >>= 1;
                i -= 1;
            }
        }
        output[i] = 0x2d; // -
        return Ok(std::str::from_utf8(&output).unwrap().to_owned());
    }
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
    app.at("/:repo_name/refs").get(repo_refs);
    app.at("/:repo_name/log").get(repo_log);
    app.at("/:repo_name/log/:ref").get(repo_log); // ref optional
    app.at("/:repo_name/tree").get(repo_tree);
    app.at("/:repo_name/tree/:ref").get(repo_tree);
    // app.at("/:repo_name/tree/:ref/item/:file").get(repo_log); ref = master/main when not present
    // Bonus: raw files, patchsets
    app.listen("127.0.0.1:8081").await?;
    Ok(())
}
