# mygit
Small self-hosted git, written in Rust

More lightweight than something like [gitea](https://gitea.io/en-us/), more modern than something like [cgit](https://git.zx2c4.com/cgit/) or [gitweb](https://git-scm.com/book/en/v2/Git-on-the-Server-GitWeb)

Live demo at https://git.alexwennerberg.com

## Deploying
Build your binary with `cargo build --release`. Then probably move it somewhere sensible so it's in your $PATH or use `cargo install --release`. Packages and prebuilt binaries are TBD.

Probably you want to use your linux distro's init system to keep this server running.

## Setting up your repos

Acquire a Linux server that you have ssh access to, and decide on the best place to place your repos.

To initialize a repo, you'll need to run a few commands. I'm using a self-hosted instance of the mygit repo as an example. Find a directory where you want to host your repositories. This is using the default settings found in mygit.toml
```
git init --bare mygit
cd mygit
touch git-daemon-export-ok
# update "dumb http" server on updates
mv hooks/post-update.sample hooks/post-update
```
Update the `description` file with a description of the repository

Make sure the HEAD in your remote repo points to your default branch (e.g. master vs main)

Pushing your changes is not handled via mygit -- this will be done over ssh. For example:

```
git remote add origin ssh://git@git.alexwennerberg.com:/www/git/mygit
git push -u origin main
```

You'll want to modify these commands to push to your mygit server and somewhere else (e.g. a GitHub mirror). This is left as an exercise to the reader.

## Background
Many people want to self-host Git in order to get rid of their reliance on GitHub or other institutions. However, the options for doing this are problematic in a number of ways. There are ancient CGI programs written in C or Perl like gitolite, cgit and webgit, and there are modern programs like gitea or gitlab that are essentially GitHub clones, with a lot of unnecessary complexity for many people's use cases.

The simplest way to accept patches when self-hosting Git is through [git-send-email](https://git-scm.com/docs/git-send-email). You can accept patches either to your personal email or use a mailing list. 

I am working on a sibling project to this that handles mailing list archives for exactly this purpose:
https://github.com/alexwennerberg/crabmail

## Contributing
* [ticket tracker](https://todo.sr.ht/~aw/mygit)
* [patches](https://lists.sr.ht/~aw/patches)
