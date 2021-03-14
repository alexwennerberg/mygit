use anyhow::Result;
use askama::Template;
use git2::{Commit, Diff, DiffDelta, DiffFormat, Oid, Reference, Repository, Tree, TreeEntry};
use once_cell::sync::Lazy;
use pico_args;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::str;
use tide::prelude::*;
use tide::Request;

#[derive(Deserialize, Debug)]
pub struct Config {
    port: u16,
    repo_directory: String,
    emoji_favicon: String,
}

const HELP: &str = "\
mygit

FLAGS:
  -h, --help            Prints help information
OPTIONS:
  -c                    Path to config file
";

static CONFIG: Lazy<Config> = Lazy::new(args);

fn args() -> Config {
    // TODO cli

    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let toml_text =
        fs::read_to_string("mygit.toml").expect("expected configuration file mygit.toml");
    match toml::from_str(&toml_text) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("could not read configuration file: {}", e);
            std::process::exit(1);
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")] // using the template in this path, relative
struct IndexTemplate {
    repos: Vec<Repository>,
}

async fn index(req: Request<()>) -> tide::Result {
    let repos = fs::read_dir(&CONFIG.repo_directory)
        .map(|entries| {
            entries
                .filter_map(|entry| Some(entry.ok()?.path()))
                .filter(|entry| {
                    // check for the export file
                    let mut path = entry.clone();
                    path.push("git-daemon-export-ok");
                    path.exists()
                })
                .filter_map(|entry| Repository::open(entry).ok())
                .collect::<Vec<_>>()
        })
        .map_err(|e| tide::log::warn!("can not read repositories: {}", e))
        .unwrap_or_default();
    let index_template = IndexTemplate { repos };

    Ok(index_template.into())
}

#[derive(Template)]
#[template(path = "repo.html")] // using the template in this path, relative
struct RepoHomeTemplate {
    repo: Repository,
    readme_text: String,
}

fn repo_from_request(repo_name: &str) -> Result<Repository> {
    let repo_path = Path::new(&CONFIG.repo_directory).join(repo_name);
    // TODO CLEAN PATH! VERY IMPORTANT! DONT FORGET!
    let r = Repository::open(repo_path)?;
    Ok(r)
}

async fn repo_home(req: Request<()>) -> tide::Result {
    use pulldown_cmark::{escape::escape_html, html::push_html, Options, Parser};

    enum ReadmeFormat {
        Plaintext,
        Html,
        Markdown,
    }

    let repo = repo_from_request(&req.param("repo_name")?)?;

    let readme_text = {
        let mut format = ReadmeFormat::Plaintext;
        let readme = repo
            .revparse_single("HEAD:README")
            .or_else(|_| repo.revparse_single("HEAD:README.txt"))
            .or_else(|_| {
                format = ReadmeFormat::Markdown;
                repo.revparse_single("HEAD:README.md")
            })
            .or_else(|_| repo.revparse_single("HEAD:README.mdown"))
            .or_else(|_| repo.revparse_single("HEAD:README.markdown"))
            .or_else(|_| {
                format = ReadmeFormat::Html;
                repo.revparse_single("HEAD:README.html")
            })
            .or_else(|_| repo.revparse_single("HEAD:README.htm"))?;
        let readme_text = str::from_utf8(readme.as_blob().unwrap().content())?;

        // render the file contents to HTML
        match format {
            // render plaintext as preformatted text
            ReadmeFormat::Plaintext => {
                let mut output = "<pre>".to_string();
                escape_html(&mut output, readme_text)?;
                output.push_str("</pre>");
                output
            }
            // already is HTML
            ReadmeFormat::Html => readme_text.to_string(),
            // render Markdown to HTML
            ReadmeFormat::Markdown => {
                let mut output = String::new();
                let parser = Parser::new_ext(readme_text, Options::empty());
                push_html(&mut output, parser);
                output
            }
        }
    };

    Ok(RepoHomeTemplate { repo, readme_text }.into())
}

#[derive(Template)]
#[template(path = "log.html")] // using the template in this path, relative
struct RepoLogTemplate<'a> {
    repo: &'a Repository,
    commits: Vec<Commit<'a>>,
}

async fn repo_log(req: Request<()>) -> tide::Result {
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
        commits,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "refs.html")] // using the template in this path, relative
struct RepoRefTemplate<'a> {
    repo: &'a Repository,
    branches: Vec<Reference<'a>>,
    tags: Vec<Reference<'a>>,
}
async fn repo_refs(req: Request<()>) -> tide::Result {
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
        branches,
        tags,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "tree.html")] // using the template in this path, relative
struct RepoTreeTemplate<'a> {
    repo: &'a Repository,
    tree: Tree<'a>,
}
async fn repo_tree(req: Request<()>) -> tide::Result {
    // TODO handle subtrees
    let repo = repo_from_request(&req.param("repo_name")?)?;
    // TODO accept reference or commit id
    let spec = req.param("ref").unwrap_or("HEAD");
    let commit = repo.revparse_single(spec)?.peel_to_commit()?;
    let tree = commit.tree()?;
    let tmpl = RepoTreeTemplate { repo: &repo, tree };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "commit.html")] // using the template in this path, relative
struct RepoCommitTemplate<'a> {
    repo: &'a Repository,
    commit: Commit<'a>,
    parent: Commit<'a>,
    diff: &'a Diff<'a>,
    deltas: Vec<DiffDelta<'a>>,
}

async fn repo_commit(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(req.param("repo_name")?)?;
    let commit = repo
        .revparse_single(req.param("commit")?)?
        .peel_to_commit()?;

    let parent = repo
        .revparse_single(&format!("{}^", commit.id()))?
        .peel_to_commit()?;
    // TODO root commit
    // how to deal w multiple parents?
    let diff = repo.diff_tree_to_tree(Some(&commit.tree()?), Some(&parent.tree()?), None)?;
    let deltas = diff.deltas().collect();

    // TODO accept reference or commit id
    let tmpl = RepoCommitTemplate {
        repo: &repo,
        commit,
        parent,
        diff: &diff,
        deltas,
    };
    Ok(tmpl.into())
}

mod filters {
    use super::*;

    pub fn format_datetime(time: &git2::Time, format: &str) -> ::askama::Result<String> {
        use chrono::{FixedOffset, TimeZone};
        let offset = FixedOffset::west(time.offset_minutes() * 60);
        let datetime = offset.timestamp(time.seconds(), 0);
        Ok(datetime.format(format).to_string())
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

    pub fn repo_name(repo: &Repository) -> askama::Result<&str> {
        repo.workdir()
            // use the path for bare repositories
            .unwrap_or_else(|| repo.path())
            .file_name()
            .unwrap()
            .to_str()
            .ok_or(askama::Error::Fmt(std::fmt::Error))
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    tide::log::start();
    let mut app = tide::new();
    app.at("/").get(index);
    app.at("/robots.txt")
        .serve_file("templates/static/robots.txt")?; // TODO configurable
    app.at("/style.css")
        .serve_file("templates/static/style.css")?; // TODO configurable
    app.at("/:repo_name").get(repo_home);
    // ALSO do git pull at this url somehow ^
    app.at("/:repo_name/commit/:commit").get(repo_commit);
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
