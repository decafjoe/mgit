# mgit

mgit is a command-line application for managing multiple git
repositories.

**Big, honking note.** This is my first foray into Rust. So there's no
documentation, testing, QA, etc. And there's a high probability the
code is garbage. But if you clone the repo and run ``cargo build`` you
should end up with a binary that does what it says on the tin.

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

Configuration files are in the ini format. The `[repos]` section
specifies the repositories mgit will look at:

```ini
[repos]
mgit = /home/jjoyce/mgit
emacs-d = /home/jjoyce/.emacs.d
# ...
```

Keys are the "human-friendly" name for the repo. Values are the path.
And that's it! mgit figures out the rest on its own.

By default, mgit looks at `~/.mgit` for configuration. You can change
that by specifying the `-c` (`--config`) option. The option may be
specified multiple times:

```sh
mgit -c /path/to/a/file.conf -c /path/to/a/directory <COMMAND>
```

All paths will be read into the configuration.

mgit recursively walks directories looking for files with the
extension `.conf`, which are read into the configuration.

The recommended configuration layout is to create `~/.mgit` as a
directory and add configuration files underneath. The names of the
configuration files are guided by how you want to group your
repositories (see below).


### Groups

Under the covers, each configuration file actually creates or extends
a group. Groups may contain multiple repos, but a repo may belong to
only one group (corresponding to the file in which it was configured).

By default the group name is the stem of the configuration filename:

* `foo.conf`'s group name is `foo`
* `foo-bar.conf`'s group name is `foo-bar`
* `foo.bar.conf`'s group name is `foo.bar`
* …and so on

To override the name, specify the `name` key in the `[group]` section:

```ini
[group]
name = Not Foo Bar
```

The group also defines the "symbol" for the repos in the group — the
text that is output before the repository name. This defaults to `•`,
but can be changed by specifying `symbol`:

```ini
[group]
symbol = ⊃
```

This allows group information to be "exposed" in `mgit status`
output::

```sh
• repo1
• repo2
⊃ repo3
• repo4
⊃ repo5
⊃ repo6
```

If mgit encounters two (or more) files with the same group name, the
configurations are merged. So if `~/.mgit/foo.conf` defines two repos
and `~/.mgit/bar/foo.conf` defines three, the end result is one
`foo` group with five repos.

*Except* when both files have a repo with the same name. In that case,
mgit prints a warning and "last one wins." Configuration processing
order is not formally defined, so it is not recommended to rely on it.

Symbols are handled the same way; if two files of the same group
define different `symbol` values, the last one wins. Again, relying
on this behavior is not recommended.

So how to use groups?

You could group repos by language:

```
~/.mgit/
  cobol.conf
  python.conf
  rust.conf
```

Or by function:

```
~/.mgit/
  apps.conf
  docs.conf
  libs.conf
  infra.conf
```

Or by origin:

```
~/.mgit/
  personal.conf
  work.conf
```

Or you can just toss all your configuration into a single file if
groups aren't appropriate!

```
~/.mgit/
  repos.conf
```
