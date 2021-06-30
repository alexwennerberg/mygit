use super::*;

pub fn format_datetime(time: &git2::Time, format: &str) -> ::askama::Result<String> {
    use chrono::{FixedOffset, TimeZone};
    let offset = FixedOffset::west(time.offset_minutes() * 60);
    let datetime = offset.timestamp(time.seconds(), 0);
    Ok(datetime.format(format).to_string())
}

pub fn unix_perms(m: &i32) -> ::askama::Result<String> {
    // https://unix.stackexchange.com/questions/450480/file-permission-with-six-bytes-in-git-what-does-it-mean
    // Git doesn’t store arbitrary modes, only a subset of the values are
    // allowed. Since the number of possible values is quite small, it is
    // easiest to exhaustively match them.
    Ok(match m {
        0o040000 => "drwxr-xr-x", // directory
        0o100755 => "-rwxr-xr-x", // regular file, executable
        0o100644 => "-rw-r--r--", // regular file, default umask
        0o120000 => "lrwxrwxrwx", // symlink
        0o160000 => "m---------", // submodule
        _ => unreachable!("unknown file mode"),
    }
    .into())
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
