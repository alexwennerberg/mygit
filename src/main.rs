use askama::Template;
use git2::{Commit, Diff, Reference, Repository, Signature, Tree};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::str;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use tide::{http, Request, Response};

mod errorpage;
mod mail;

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default = "defaults::port")]
    port: u16,
    #[serde(default = "defaults::repo_directory")]
    projectroot: String,
    #[serde(default = "String::new")]
    emoji_favicon: String,
    #[serde(default = "defaults::site_name")]
    site_name: String,
    #[serde(default = "defaults::export_ok")]
    export_ok: String,
    #[serde(default = "String::new")]
    clone_base: String,
    #[serde(default = "defaults::log_per_page")]
    log_per_page: usize,
}

/// Defaults for the configuration options
// FIXME: simplify if https://github.com/serde-rs/serde/issues/368 is resolved
mod defaults {
    pub fn port() -> u16 {
        80
    }

    pub fn repo_directory() -> String {
        "repos".to_string()
    }

    pub fn site_name() -> String {
        "mygit".to_string()
    }

    pub fn export_ok() -> String {
        "git-daemon-export-ok".to_string()
    }

    pub fn log_per_page() -> usize {
        100
    }
}

const HELP: &str = "\
Usage: mygit

FLAGS:
  -h, --help            Prints this help information and exits.
OPTIONS:
  -c, --config <FILE>   Use a specific configuration file.
                        default is ./mygit.toml

Mandatory or optional arguments to long options are also mandatory or optional
for any corresponding short options.

Report bugs at https://todo.sr.ht/~aw/mygit
";

static CONFIG: Lazy<Config> = Lazy::new(args);
// so we only have to load this once to reduce startup time for syntax highlighting
static SYNTAXES: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);

fn args() -> Config {
    // TODO cli

    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let config_filename = pargs
        .opt_value_from_str(["-c", "--config"])
        .unwrap()
        .unwrap_or_else(|| "mygit.toml".to_string());

    let toml_text = fs::read_to_string(&config_filename).unwrap_or_else(|_| {
        tide::log::warn!(
            "configuration file {:?} not found, using defaults",
            config_filename
        );
        String::new()
    });
    match toml::from_str(&toml_text) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("could not parse configuration file: {}", e);
            std::process::exit(1);
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")] // using the template in this path, relative
struct IndexTemplate {
    repos: Vec<Repository>,
}

async fn index(_req: Request<()>) -> tide::Result {
    let repos = fs::read_dir(&CONFIG.projectroot)
        .map(|entries| {
            entries
                .filter_map(|entry| Some(entry.ok()?.path()))
                .filter_map(|entry| Repository::open(entry).ok())
                .filter(|repo| {
                    // check for the export file in the git directory
                    // (the .git subfolder for non-bare repos)
                    repo.path().join(&CONFIG.export_ok).exists()
                })
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

fn repo_from_request(repo_name: &str) -> Result<Repository, tide::Error> {
    let repo_name = percent_encoding::percent_decode_str(repo_name)
        .decode_utf8_lossy()
        .into_owned();

    let repo_path = Path::new(&CONFIG.projectroot).join(repo_name);

    // prevent path traversal
    if !repo_path.starts_with(&CONFIG.projectroot) {
        return Err(tide::Error::from_str(
            403,
            "You do not have access to this resource.",
        ));
    }

    Repository::open(repo_path)
        .ok()
        // outside users should not be able to tell the difference between
        // nonexistent and existing but forbidden repos, so not using 403
        .filter(|repo| repo.path().join(&CONFIG.export_ok).exists())
        .ok_or_else(|| tide::Error::from_str(404, "This repository does not exist."))
}

async fn repo_home(req: Request<()>) -> tide::Result {
    use pulldown_cmark::{escape::escape_html, html::push_html, Options, Parser};

    enum ReadmeFormat {
        Plaintext,
        Html,
        Markdown,
    }

    let repo = repo_from_request(&req.param("repo_name")?)?;

    let mut format = ReadmeFormat::Plaintext;
    let readme_text = repo
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
        .or_else(|_| repo.revparse_single("HEAD:README.htm"))
        .ok()
        .and_then(|readme| readme.into_blob().ok())
        .map(|blob| {
            let text = str::from_utf8(blob.content()).unwrap_or_default();

            // render the file contents to HTML
            match format {
                // render plaintext as preformatted text
                ReadmeFormat::Plaintext => {
                    let mut output = "<pre>".to_string();
                    escape_html(&mut output, text).unwrap();
                    output.push_str("</pre>");
                    output
                }
                // already is HTML
                ReadmeFormat::Html => text.to_string(),
                // render Markdown to HTML
                ReadmeFormat::Markdown => {
                    let mut output = String::new();
                    let parser = Parser::new_ext(text, Options::empty());
                    push_html(&mut output, parser);
                    output
                }
            }
        })
        .unwrap_or_default();

    Ok(RepoHomeTemplate { repo, readme_text }.into())
}

#[derive(Template)]
#[template(path = "log.html")] // using the template in this path, relative
struct RepoLogTemplate<'a> {
    repo: &'a Repository,
    commits: Vec<Commit<'a>>,
    branch: &'a str,
    // the spec the user should be linked to to see the next page of commits
    next_page: Option<String>,
}

async fn repo_log(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(&req.param("repo_name")?)?;
    if repo.is_empty().unwrap() {
        // redirect to start page of repo
        let mut url = req.url().clone();
        url.path_segments_mut().unwrap().pop();
        return Ok(tide::Redirect::temporary(url.to_string()).into());
    }

    let next_page_spec;
    let mut commits = if repo.is_shallow() {
        tide::log::warn!("repository {:?} is only a shallow clone", repo.path());
        next_page_spec = "".into();
        vec![repo.head()?.peel_to_commit().unwrap()]
    } else {
        let mut revwalk = repo.revwalk()?;
        let r = req.param("ref").unwrap_or("HEAD");
        revwalk.push(repo.revparse_single(r)?.peel_to_commit()?.id())?;

        if let Some(i) = r.rfind('~') {
            // there is a tilde, try to find a number too
            let n = r[i + 1..].parse::<usize>().ok().unwrap_or(1);
            next_page_spec = format!("{}~{}", &r[..i], n + CONFIG.log_per_page);
        } else {
            // there was no tilde
            next_page_spec = format!("{}~{}", r, CONFIG.log_per_page);
        }

        revwalk.set_sorting(git2::Sort::TIME).unwrap();
        revwalk
            .filter_map(|oid| repo.find_commit(oid.unwrap()).ok()) // TODO error handling
            .take(CONFIG.log_per_page + 1)
            .collect()
    };
    // check if there even is a next page
    let next_page = if commits.len() < CONFIG.log_per_page + 1 {
        None
    } else {
        // remove additional commit from next page check
        commits.pop();
        Some(next_page_spec)
    };

    let head_branch = repo.head()?;
    let branch = req
        .param("ref")
        .ok()
        .or_else(|| head_branch.shorthand())
        .unwrap();
    let tmpl = RepoLogTemplate {
        repo: &repo,
        commits,
        branch,
        next_page,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "refs.html")] // using the template in this path, relative
struct RepoRefTemplate<'a> {
    repo: &'a Repository,
    branches: Vec<Reference<'a>>,
    tags: Vec<(String, Signature<'static>)>,
}

async fn repo_refs(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(&req.param("repo_name")?)?;
    if repo.is_empty().unwrap() {
        // redirect to start page of repo
        let mut url = req.url().clone();
        url.path_segments_mut().unwrap().pop();
        return Ok(tide::Redirect::temporary(url.to_string()).into());
    }

    let branches = repo
        .references()?
        .filter_map(|x| x.ok())
        .filter(|x| x.is_branch())
        .collect();
    let mut tags = Vec::new();
    repo.tag_foreach(|oid, name_bytes| {
        let obj = repo.find_object(oid, None).unwrap();
        let signature = match obj.kind().unwrap() {
            git2::ObjectType::Tag => obj
                .as_tag()
                .unwrap()
                .tagger()
                .unwrap_or_else(|| obj.peel_to_commit().unwrap().committer().to_owned())
                .to_owned(),
            git2::ObjectType::Commit => {
                // lightweight tag
                obj.as_commit().unwrap().committer().to_owned()
            }
            _ => unreachable!("a tag was not a tag or lightweight tag"),
        };
        tags.push((
            // remove prefix "ref/tags/"
            String::from_utf8(name_bytes[10..].to_vec()).unwrap(),
            signature,
        ));
        true
    })
    .unwrap();
    // sort so that newest tags are at the top
    tags.sort_unstable_by(|(_, a), (_, b)| a.when().cmp(&b.when()).reverse());
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
    path: &'a Path,
    spec: &'a str,
}

async fn repo_tree(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(&req.param("repo_name")?)?;
    if repo.is_empty().unwrap() {
        // redirect to start page of repo
        let mut url = req.url().clone();
        url.path_segments_mut().unwrap().pop();
        return Ok(tide::Redirect::temporary(url.to_string()).into());
    }

    let head = repo.head()?;
    let spec = req.param("ref").ok().or_else(|| head.shorthand()).unwrap();
    let commit = repo.revparse_single(spec)?.peel_to_commit()?;
    let tree = commit.tree()?;
    let tmpl = RepoTreeTemplate {
        repo: &repo,
        tree,
        path: Path::new(""),
        spec,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "commit.html")] // using the template in this path, relative
struct RepoCommitTemplate<'a> {
    repo: &'a Repository,
    commit: Commit<'a>,
    diff: &'a Diff<'a>,
}

impl RepoCommitTemplate<'_> {
    fn parent_ids(&self) -> Vec<git2::Oid> {
        self.commit.parent_ids().collect()
    }

    fn diff(&self) -> String {
        let mut buf = String::new();
        self.diff
            .print(
                git2::DiffFormat::Patch,
                |_delta, _hunk, line| match str::from_utf8(line.content()) {
                    Ok(content) => {
                        match line.origin() {
                            'F' | 'H' => {}
                            c @ ' ' | c @ '+' | c @ '-' | c @ '=' | c @ '<' | c @ '>' => {
                                buf.push(c)
                            }
                            _ => unreachable!(),
                        }
                        buf.push_str(content);
                        true
                    }
                    Err(_) => {
                        buf.push_str("Cannot display diff for binary file.");
                        false
                    }
                },
            )
            .unwrap();

        // highlight the diff
        let syntax = SYNTAXES
            .find_syntax_by_name("Diff")
            .expect("diff syntax missing");
        let mut highlighter =
            ClassedHTMLGenerator::new_with_class_style(&syntax, &SYNTAXES, ClassStyle::Spaced);
        LinesWithEndings::from(&buf)
            .for_each(|line| highlighter.parse_html_for_line_which_includes_newline(line));
        highlighter.finalize()
    }
}

async fn repo_commit(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(req.param("repo_name")?)?;
    let commit = repo
        .revparse_single(req.param("commit")?)?
        .peel_to_commit()?;

    // This is identical to getting "commit^" and on merges this will be the
    // merged into branch before the merge.
    let parent_tree = commit.parent(0).ok().map(|parent| parent.tree().unwrap());

    let mut diff = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&commit.tree()?), None)?;
    let mut find_options = git2::DiffFindOptions::new();
    // try to find moved/renamed files
    find_options.all(true);
    diff.find_similar(Some(&mut find_options)).unwrap();

    let tmpl = RepoCommitTemplate {
        repo: &repo,
        commit,
        diff: &diff,
    };
    Ok(tmpl.into())
}

#[derive(Template)]
#[template(path = "file.html")] // using the template in this path, relative
struct RepoFileTemplate<'a> {
    repo: &'a Repository,
    path: &'a Path,
    file_text: &'a str,
    spec: &'a str,
}

async fn repo_file(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(req.param("repo_name")?)?;

    let spec = req.param("ref").unwrap();
    let commit = repo.revparse_single(spec)?.peel_to_commit()?;
    let tree = commit.tree()?;

    let path = Path::new(req.param("object_name")?);
    let tree_entry = tree.get_path(path).unwrap();

    // TODO make sure I am escaping html properly here
    // TODO allow disabling of syntax highlighting
    // TODO -- dont pull in memory, use iterators if possible
    let extension = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or_default();
    let syntax = SYNTAXES
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text());
    let tmpl = match tree_entry.to_object(&repo)?.into_tree() {
        // this is a subtree
        Ok(tree) => RepoTreeTemplate {
            repo: &repo,
            tree,
            path,
            spec: &spec,
        }
        .into(),
        // this is not a subtree, so it should be a blob i.e. file
        Err(tree_obj) => {
            let blob = tree_obj.as_blob().unwrap();
            let output = if blob.is_binary() {
                // this is not a text file, but try to serve the file if the MIME type
                // can give a hint at how
                let mime = http::Mime::from_extension(extension).unwrap_or_else(|| {
                    if blob.is_binary() {
                        http::mime::BYTE_STREAM
                    } else {
                        http::mime::PLAIN
                    }
                });
                println!("{}", mime);
                match mime.basetype() {
                    "text" => unreachable!("git detected this file as binary"),
                    "image" => format!(
                        "<img src=\"/{}/tree/{}/raw/{}\" />",
                        req.param("repo_name").unwrap(),
                        spec,
                        path.display()
                    ),
                    tag@"audio"|tag@"video" => format!(
                        "<{} src=\"/{}/tree/{}/raw/{}\" controls>Your browser does not have support for playing this {0} file.</{0}>",
                        tag,
                        req.param("repo_name").unwrap(),
                        spec,
                        path.display()
                    ),
                    _ => "Cannot display binary file.".to_string()
                }
            } else {
                // get file contents from git object
                let file_string = str::from_utf8(tree_obj.as_blob().unwrap().content())?;
                // create a highlighter that uses CSS classes so we can use prefers-color-scheme
                let mut highlighter = ClassedHTMLGenerator::new_with_class_style(
                    &syntax,
                    &SYNTAXES,
                    ClassStyle::Spaced,
                );
                LinesWithEndings::from(file_string)
                    .for_each(|line| highlighter.parse_html_for_line_which_includes_newline(line));

                // use oid so it is a permalink
                let prefix = format!(
                    "/{}/tree/{}/item/{}",
                    req.param("repo_name").unwrap(),
                    commit.id(),
                    path.display()
                );

                let mut output = String::from("<pre>\n");
                for (n, line) in highlighter.finalize().lines().enumerate() {
                    output.push_str(&format!(
                        "<a href='{1}#L{0}' id='L{0}' class='line'>{0}</a>{2}\n",
                        n + 1,
                        prefix,
                        line,
                    ));
                }
                output.push_str("</pre>\n");
                output
            };
            RepoFileTemplate {
                repo: &repo,
                path,
                file_text: &output,
                spec: &spec,
            }
            .into()
        }
    };
    Ok(tmpl)
}

async fn repo_file_raw(req: Request<()>) -> tide::Result {
    println!("hey");
    let repo = repo_from_request(req.param("repo_name")?)?;

    let spec = req.param("ref").unwrap();
    let tree = repo.revparse_single(spec)?.peel_to_commit()?.tree()?;

    let path = Path::new(req.param("object_name")?);
    let blob = tree
        .get_path(path)
        .and_then(|tree_entry| tree_entry.to_object(&repo)?.peel_to_blob());
    match blob {
        Ok(blob) => {
            let extension = path
                .extension()
                .and_then(std::ffi::OsStr::to_str)
                .unwrap_or_default();
            let mime = http::Mime::from_extension(extension).unwrap_or_else(|| {
                if blob.is_binary() {
                    http::mime::BYTE_STREAM
                } else {
                    http::mime::PLAIN
                }
            });

            // have to put the blob's content into a Vec here because the repo will be dropped
            Ok(Response::builder(200)
                .body(blob.content().to_vec())
                .content_type(mime)
                .build())
        }
        Err(e) => Err(tide::Error::from_str(
            404,
            format!(
                "There is no such file in this revision of the repository: {}",
                e
            ),
        )),
    }
}

async fn git_data(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(req.param("repo_name")?)?;
    let path = req
        .url()
        .path()
        .strip_prefix(&format!("/{}/", req.param("repo_name").unwrap()))
        .unwrap_or_default();
    let path = repo.path().join(path);

    if !path.starts_with(repo.path()) {
        // that path got us outside of the repository structure somehow
        tide::log::warn!("Attempt to acces file outside of repo dir: {:?}", path);
        Err(tide::Error::from_str(
            403,
            "You do not have access to this file.",
        ))
    } else if !path.is_file() {
        // Either the requested resource does not exist or it is not
        // a file, i.e. a directory.
        Err(tide::Error::from_str(404, "This page does not exist."))
    } else {
        // ok - inside the repo directory
        let mut resp = tide::Response::new(200);
        let mut body = tide::Body::from_file(path).await?;
        body.set_mime("text/plain; charset=utf-8");
        resp.set_body(body);
        Ok(resp)
    }
}

/// Serve a file from ./templates/static/
async fn static_resource(req: Request<()>) -> tide::Result {
    use http::conditional::{IfModifiedSince, LastModified};

    // only use a File handle here because we might not need to load the file
    let file_mime_option = match req.url().path() {
        "/style.css" => Some((
            File::open("templates/static/style.css").unwrap(),
            http::mime::CSS,
        )),
        "/robots.txt" => Some((
            File::open("templates/static/robots.txt").unwrap(),
            http::mime::PLAIN,
        )),
        _ => None,
    };

    match file_mime_option {
        Some((mut file, mime)) => {
            let metadata = file.metadata().unwrap();
            let last_modified = metadata.modified().unwrap();

            let header = IfModifiedSince::from_headers(&req).unwrap();

            // check cache validating headers
            if matches!(header, Some(date) if IfModifiedSince::new(last_modified) <= date) {
                // the file has not changed
                let mut response = Response::new(304);
                response.set_content_type(mime);
                LastModified::new(last_modified).apply(&mut response);

                /*
                A server MAY send a Content-Length header field in a 304
                response to a conditional GET request; a server MUST NOT send
                Content-Length in such a response unless its field-value equals
                the decimal number of octets that would have been sent in the
                payload body of a 200 response to the same request.
                - RFC 7230 § 3.3.2
                */
                response.insert_header("Content-Length", metadata.len().to_string());

                return Ok(response);
            }

            let mut response = Response::new(200);

            match req.method() {
                http::Method::Head => {
                    /*
                    A server MAY send a Content-Length header field in a
                    response to a HEAD request; a server MUST NOT send
                    Content-Length in such a response unless its field-value
                    equals the decimal number of octets that would have been
                    sent in the payload body of a response if the same request
                    had used the GET method.
                    - RFC 7230 § 3.3.2
                    */
                    response.insert_header(
                        "Content-Length",
                        file.metadata().unwrap().len().to_string(),
                    );
                }
                http::Method::Get => {
                    // load the file from disk
                    let mut content = String::new();
                    file.read_to_string(&mut content).unwrap();
                    response.set_body(content);
                }
                _ => return Err(tide::Error::from_str(405, "")),
            }

            response.set_content_type(mime);
            LastModified::new(last_modified).apply(&mut response);
            Ok(response)
        }
        None if req.method() == http::Method::Get => {
            Err(tide::Error::from_str(404, "This page does not exist."))
        }
        // issue a 405 error since this is used as the catchall
        None => Err(tide::Error::from_str(405, "")),
    }
}

#[derive(Template)]
#[template(path = "log.xml")]
struct RepoLogFeedTemplate<'a> {
    repo: &'a Repository,
    commits: Vec<Commit<'a>>,
    branch: &'a str,
    base_url: &'a str,
}

async fn repo_log_feed(req: Request<()>) -> tide::Result {
    let repo = repo_from_request(&req.param("repo_name")?)?;
    if repo.is_empty().unwrap() {
        // show a server error
        return Err(tide::Error::from_str(
            503,
            "Cannot show feed because there are no commits.",
        ));
    }

    let commits = if repo.is_shallow() {
        tide::log::warn!("repository {:?} is only a shallow clone", repo.path());
        vec![repo.head()?.peel_to_commit().unwrap()]
    } else {
        let mut revwalk = repo.revwalk()?;
        let r = req.param("ref").unwrap_or("HEAD");
        revwalk.push(repo.revparse_single(r)?.peel_to_commit()?.id())?;

        revwalk.set_sorting(git2::Sort::TIME).unwrap();
        revwalk
            .filter_map(|oid| repo.find_commit(oid.unwrap()).ok()) // TODO error handling
            .take(CONFIG.log_per_page)
            .collect()
    };

    let head_branch = repo.head()?;
    let branch = req
        .param("ref")
        .ok()
        .or_else(|| head_branch.shorthand())
        .unwrap();

    let mut url = req.url().clone();
    {
        let mut segments = url.path_segments_mut().unwrap();
        segments.pop(); // pop "feed.xml"
        segments.pop(); // pop "log/" or ref
        if req.param("ref").is_ok() {
            segments.pop(); // the last pop was a ref, now pop "log/"
        }
    }

    let tmpl = RepoLogFeedTemplate {
        repo: &repo,
        commits,
        branch,
        base_url: url.as_str(),
    };
    let mut response: tide::Response = tmpl.into();
    response.set_content_type("application/rss+xml");
    Ok(response)
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

    pub fn description(repo: &Repository) -> askama::Result<String> {
        Ok(fs::read_to_string(repo.path().join("description"))
            .unwrap_or_default()
            // only use first line
            .lines()
            .next()
            .unwrap_or_default()
            .to_string())
    }

    pub fn last_modified(repo: &Repository) -> askama::Result<git2::Time> {
        Ok(repo
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .committer()
            .when())
    }

    pub fn repo_owner(repo: &Repository) -> askama::Result<String> {
        Ok(repo
            .config()
            .unwrap()
            .get_string("gitweb.owner")
            .unwrap_or_default())
    }

    pub fn signature_email_link(signature: &Signature) -> askama::Result<String> {
        Ok(if let Some(email) = signature.email() {
            format!(
                "<a href=\"mailto:{}\">{}</a>",
                email,
                signature.name().unwrap_or("&#65533;")
            )
        } else {
            signature.to_string()
        })
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    tide::log::start();
    let mut app = tide::new();
    app.with(errorpage::ErrorToErrorpage);
    app.at("/").get(index);

    app.at("/style.css").get(static_resource);
    app.at("/robots.txt").get(static_resource);

    // Raw files, patch files
    app.at("/mail").get(mail::list_threads);

    app.at("/:repo_name").get(repo_home);
    app.at("/:repo_name/").get(repo_home);

    // git clone stuff
    app.at("/:repo_name/info/refs").get(git_data);
    app.at("/:repo_name/HEAD").get(git_data);
    app.at("/:repo_name/objects/*obj").get(git_data);

    // web pages
    app.at("/:repo_name/commit/:commit").get(repo_commit);
    app.at("/:repo_name/refs").get(repo_refs);
    app.at("/:repo_name/log").get(repo_log);
    app.at("/:repo_name/log/").get(repo_log);
    app.at("/:repo_name/log/:ref").get(repo_log); // ref is optional
    app.at("/:repo_name/log/feed.xml").get(repo_log_feed);
    app.at("/:repo_name/log/:ref/feed.xml").get(repo_log_feed); // ref is optional
    app.at("/:repo_name/tree").get(repo_tree);
    app.at("/:repo_name/tree/").get(repo_tree);
    app.at("/:repo_name/tree/:ref").get(repo_tree); // ref is optional
    app.at("/:repo_name/tree/:ref/").get(repo_tree); // ref is optional
    app.at("/:repo_name/tree/:ref/item/*object_name")
        .get(repo_file);
    app.at("/:repo_name/tree/:ref/raw/*object_name")
        .get(repo_file_raw);

    app.at("*").all(static_resource);
    app.listen(format!("[::]:{}", CONFIG.port)).await?;
    Ok(())
}
