<p align="center"><img src="docs/logo.svg" width="96" alt="" /></p>

# shexpand

[![CI](https://github.com/Gulliby-development/shexpand/actions/workflows/ci.yml/badge.svg)](https://github.com/Gulliby-development/shexpand/actions/workflows/ci.yml) [![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

shexpand is an offline command-line tool for people who write shell scripts,
Makefile recipes, CI steps, and Dockerfile lines and want to inspect how a
command line is interpreted by the supported shell subset. It shows where
parameters split, which braces expand, and what quoting protects. Values and
virtual file paths are supplied as arguments; the program does not read the
host environment or real filesystem and makes no network requests.

## Why

Before a script runs `rm $DIR/*` or passes `"$@"` on, you want to know exactly
which words the shell will make of it. shexpand answers that without running
anything: it cuts the line into words, expands braces, variables and patterns
over values and file names you give it, and warns about unquoted expansions.

## Install

A Rust toolchain is required. Build the release executable with
`cargo build --release`. To run a command from the project directory, use
`cargo run -- COMMAND`, replacing `COMMAND` with one of the commands below.

The application is designed for Windows, macOS, and Linux. The complete
invocations in this README use POSIX-style shell quoting; Windows command
shells use different quoting rules, so enter equivalent arguments using the
quoting syntax of your shell.

## Usage

The help command lists the supported command forms:

```text
shexpand split LINE
shexpand braces LINE
shexpand fields [--ifs TEXT] VALUE
shexpand glob [--file PATH]... PATTERN...
shexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]
shexpand quote [--style single|double] [--always] TEXT...
shexpand lint LINE
```

The following invocations use POSIX shell quoting. The quoted shell arguments
are passed as LINE or TEXT values to shexpand.

```sh
shexpand split 'cp "my file" ${DEST:-/tmp}/x $@ # note'
```

```text
word  parts
1     u[cp]
2     q[my file]
3     p[${DEST:-/tmp}] u[/x]
4     p[$@]
words       4
```

```sh
shexpand expand --var X='a b' --var E= --var HOME=/home/u 'echo ~/x $X "$X" $E "$E" y{1,2} ${U:-q r}'
```

```text
word  text
1     [echo]
2     [/home/u/x]
3     [a]
4     [b]
5     [a b]
6     []
7     [y1]
8     [y2]
9     [q]
10    [r]
words       10
```

```sh
shexpand lint 'rm -rf $DIR/* "$*" ${X:-$Y} $@'
```

```text
column 8: W1 unquoted parameter DIR is subject to splitting and pattern matching
column 16: W3 "$*" joins the parameters into one word; "$@" keeps them apart
column 20: W1 unquoted parameter X is subject to splitting and pattern matching
column 25: W1 unquoted parameter Y is subject to splitting and pattern matching
column 29: W2 unquoted $@ splits and matches its parameters again; write "$@"
warnings    5
```

## Syntax

shexpand implements a documented shell subset, not a complete POSIX shell or
bash. It recognizes single and double quotes, backslash escapes, and comments
beginning with `#` at the start of a word. It supports quoted arguments such as `'a b'`, brace alternatives such as
`{a,b}` and ranges such as `{1..3}`, leading-home tilde expansion such as
`~/x`, and parameter forms including `${X:-fallback}`, `"$X"`, `"$@"`, and
`"$*"`. Field splitting uses the supplied `--ifs` value, and
pathname matching uses virtual paths supplied with `--file`; patterns such
as `*.txt` are matched against those paths rather than the host filesystem.
The `quote` command creates shell text, while `lint` identifies selected
risky unquoted parameter expansions.

Command substitution such as `$(ls)` and backquotes is unsupported. Shell
operators such as `|` are unsupported, as are parameters such as `$?`.
The alternate quoting forms `$'...'` and `$"..."` are also unsupported.
Literal quote payloads are preserved; no universal ASCII-only guarantee is
made for arbitrary payload text.

## Commands

- `split` displays each input word and its plain, quoted, and parameter
  parts.
- `braces` lists the results of brace alternatives and ranges in each word.
- `fields` shows how a value is divided into fields using `--ifs`.
- `glob` matches patterns against the virtual paths supplied by `--file`.
- `expand` applies brace, tilde, parameter, field-splitting, and pathname
  expansion using supplied variables, positional arguments, and virtual paths.
- `quote` produces single- or double-quoted shell text for each argument.
- `lint` reports selected risky parameter expansions and their source columns.

## Exit codes

- `0`: command succeeded.
- `1`: `lint` found one or more warnings.
- `2`: an error occurred.

## Development

Run the project checks from the repository directory:

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo test`

## License

MIT, see [LICENSE](LICENSE).

---

Built with BAKED
