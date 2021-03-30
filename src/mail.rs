use anyhow::Result;
use askama::Template;
use tide::Request;

use std::fs;
use std::borrow::Cow;

use mailparse::{parse_mail, ParsedMail, MailHeaderMap};
/* Mail related routes */

// TODO create a thread object, a collection of references to emails

#[derive(Template)]
#[template(path = "list-threads.html")] // using the template in this path, relative
struct ListThreadsTemplate {
    emails: Vec<Email>,
}

struct Email {
    subject: String
}

impl Email {
    fn from_parsed(mail: &ParsedMail) -> Result<Self> {
        Ok(Email {
            subject: mail.headers.get_first_value("Subject").unwrap()
        })
    }
}
/* This function handles a lot */
pub async fn list_threads(req: Request<()>) -> tide::Result {
    let mut mail_files = fs::read_dir("./mail")?;
    let mut result = vec![];
    for mail_file in mail_files {
        let path = mail_file?.path();
        let bytes = fs::read(path)?;
        let parsed = parse_mail(&bytes)?;
        // we need to create a struct with relevant owned data
        let mail = Email::from_parsed(&parsed)?;
        result.push(mail);
    }
    Ok(ListThreadsTemplate { emails: result }.into())
}

async fn show_thread(req: Request<()>) -> tide::Result {
    Ok("".into())
}

async fn raw_email(req: Request<()>) -> tide::Result {
    Ok("".into())
}
