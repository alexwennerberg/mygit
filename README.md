# mygit -- the world's smallest Git host

A small, self-hosted git forge, with a web viewer for repositories and and a mailing list archive

## The problem

Many people want to self-host Git in order to get rid of their reliance on GitHub or other institutions. However, the options for doing this are problematic in a number of ways. There are ancient CGI programs written in C or Perl like gitolite, cgit and webgit, and there are modern programs like gitea or gitlab that are essentially GitHub clones, with a lot of unnecessary complexity for many people's use cases.

I really like [stagit](https://codemadness.org/stagit.html), but it's a bit too austere for my use case and very "suckless" philosophy: e.g. doesn't support markdown READMEs. I also really like [sourcehut](https://git.sr.ht/) but it is pretty complex to self-host a single-user instance. A lot of the design of mygit is drawn from both these sources.

The simplest way to accept patches is through [git-send-email](https://git-scm.com/docs/git-send-email), so I also want to setup a mailing list archive. The simplest way to do this is via IMAP and the [public-inbox](https://public-inbox.org/README.html) model -- the mailing list does not send out messages but simply receives them. Users can use RSS/imap/web view to view the patches. This is step above the opacity of a personal email, but much, much easier to self-host than a full mailing list.

Like a lot of git software, a lot of email software, like public-inbox and [hyperkitty](https://github.com/hypermail-project/hypermail) are ancient C/Perl programs with certain disadvantages. I think [sourcehut](https://lists.sr.ht)'s mailing list is a great example of a modern, easy-to-use mailing list software, but in addition to being challenging to self-host, also has some highly opinionated design decisions, like blocking all html emails, even multipart ones, and using a tilde in the mailing list address, which not all providers support.

This project is on sr.ht until I can get it self-hosted. Here is the ticket tracker: https://todo.sr.ht/~aw/mygit

## Design

I basically look at a combination of:

* https://git.zx2c4.com/cgit/
* https://codemadness.org/git/
* https://git.sr.ht/

And pick the design I like best out of the three. Sometimes I do things slightly differently too.
