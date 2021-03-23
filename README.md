# mygit

A small self-hosted git server; a Rust alternative to git-web and cgit

## Background

Many people want to self-host Git in order to get rid of their reliance on GitHub or other institutions. However, the options for doing this are problematic in a number of ways. There are ancient CGI programs written in C or Perl like gitolite, cgit and webgit, and there are modern programs like gitea or gitlab that are essentially GitHub clones, with a lot of unnecessary complexity for many people's use cases.

I really like [stagit](https://codemadness.org/stagit.html), but it's a bit too austere for my use case and very "suckless" philosophy: e.g. doesn't support markdown READMEs. I also really like [sourcehut](https://git.sr.ht/) but it is pretty complex to self-host a single-user instance. 

The standard old school way of self-hosting git repos is via cgit or webgit. These are fine, but they are also ancient c/perl programs (ie, not really actively developed) that rely on CGI. Their UI is also a bit busy for me. I thought it would be nice to have an austere, simple alternative built in Rust.

The simplest way to accept patches when hosting a Git repo in this manner is through [git-send-email](https://git-scm.com/docs/git-send-email). You can accept patches either to your personal email or use a mailing list. This is a somewhat archaic way of doing things, and definitely has some disadvantages, but it is the simplest way to accept patches when self-hosting Git. A single-user [Gitea](https://gitea.io/en-us/) instance, for example, requires that users register and you manage their user accounts. With git-send-email, users can contribute to your project without having to create another account. This may not be the easiest or most accessible way to handle your project (unquestionably, that would be just using GitHub) but IMO it's the best way to do things if you want a simple self-hosted solution. Unfortunately, not many people are familiar with git-send-email anymore (and self-hosting dramatically hurts your project discovery), so probably it only makes sense to do this if you're an ideological purist of some sort, or just want to try self-hosting some small, low-stakes projects for fun.

I am working on a sibling project to this that handles mailing list archives for exactly this purpose:
https://git.sr.ht/~aw/rusty-inbox

This project is on sr.ht until I can get it self-hosted. 
* [ticket tracker](https://todo.sr.ht/~aw/mygit)
* [patches](https://lists.sr.ht/~aw/patches)

## Design

I basically look at a combination of:

* https://git.zx2c4.com/cgit/
* https://codemadness.org/git/
* https://git.sr.ht/

And pick the design I like best out of the three. Sometimes I do things slightly differently too.
