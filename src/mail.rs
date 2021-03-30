use anyhow::Result;
use askama::Template;
use tide::Request;

use std::fs;

use mailparse::{parse_mail, ParsedMail};
/* Mail related routes */

// TODO create a thread object, a collection of references to emails

#[derive(Template)]
#[template(path = "list-threads.html")] // using the template in this path, relative
struct ListThreadsTemplate<'a> {
    emails: Vec<&'a ParsedMail<'a>>,
}
/* This function handles a lot */
pub async fn list_threads(req: Request<()>) -> tide::Result {
    let mut mail_files = fs::read_dir("./mail")?;
    let mut result = vec![];
    let test = mail_files.next().unwrap();
    let path = test?.path();
    let bytes = fs::read(path)?;
    // Need to figure out out to do this parsing properly.
    // parse_mail doesn't own its data
    let parsed = parse_mail(&bytes)?;
    result.push(&parsed);
    Ok(ListThreadsTemplate { emails: result }.into())
}

async fn show_thread(req: Request<()>) -> tide::Result {
    Ok("".into())
}

async fn raw_email(req: Request<()>) -> tide::Result {
    Ok("".into())
}
