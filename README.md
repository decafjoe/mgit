# mgit

mgit is a command-line application for managing multiple git
repositories.

**Big, honking note.** This is my first foray into Rust. So I still
have a lot to learn about documentation, testing, QA, etc. And there's
a high probability the code is garbage. But if you clone the repo and
run ``cargo build`` you should end up with a binary that does what it
says on the tin.

mgit was built to help bring sanity to working with tens of git
repositories. Every day I touch zero to ~20 repositories. Not all of
those touches are neat, linear, committable chunks that I'm ready to
push upstream. So at the end of the day there is an unknown number of
changed repositories and an unknown number of changes within those.

Enter `mgit status`:

![screenshot of mgit status output](img/status.png)

Ta-da! `mgit status` tells you which worktrees are dirty and which
tracking branches are ahead/behind/diverged from their upstreams.

mgit's other command, `mgit pull`, brings sanity to the process of
pulling changes from remotes. For each repo, it fetches from all
configured remotes and, if it's safe to do so, fast-forwards tracking
branches that are behind.


## Installation

Requires:

* Cargo
* libssh2 + headers
* other stuff?

```sh
git clone https://github.com/decafjoe/mgit.git
cd mgit
cargo build --release
# Copy target/release/mgit to somewhere on your $PATH
```


## Configuration

Coming soon.
