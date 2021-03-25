# mygit
A small self-hosted git server; a Rust, non-cgi alternative to git-web and cgit

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

I really like [stagit](https://codemadness.org/stagit.html), but it's a bit too austere for my use case and very "suckless" philosophy: e.g. doesn't support markdown READMEs. I also really like [sourcehut](https://git.sr.ht/) but it is pretty complex to self-host a single-user instance. 

The simplest way to accept patches when hosting a Git repo in this manner is through [git-send-email](https://git-scm.com/docs/git-send-email). You can accept patches either to your personal email or use a mailing list. This is a somewhat archaic way of doing things, and definitely has some disadvantages, but it is the simplest way to accept patches when self-hosting Git. A single-user [Gitea](https://gitea.io/en-us/) instance, for example, requires that users register and you manage their user accounts. With git-send-email, users can contribute to your project without having to create another account. This may not be the easiest or most accessible way to handle your project (unquestionably, that would be just using GitHub) but IMO it's the best way to do things if you want a simple self-hosted solution. Unfortunately, not many people are familiar with git-send-email anymore (and self-hosting dramatically hurts your project discovery), so probably it only makes sense to do this if you're an ideological purist of some sort, or just want to try self-hosting some small, low-stakes projects for fun.

I am working on a sibling project to this that handles mailing list archives for exactly this purpose:
https://git.sr.ht/~aw/rusty-inbox

## Contributing
* [ticket tracker](https://todo.sr.ht/~aw/mygit)
* [patches](https://lists.sr.ht/~aw/patches)
