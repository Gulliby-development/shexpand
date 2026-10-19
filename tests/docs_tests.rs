use shexpand::io::Io;
use std::cell::RefCell;

const README: &str = include_str!("../README.md");

struct FakeIo {
    stdout: RefCell<String>,
    stderr: RefCell<String>,
}

impl FakeIo {
    fn new() -> Self {
        Self {
            stdout: RefCell::new(String::new()),
            stderr: RefCell::new(String::new()),
        }
    }
}

impl Io for FakeIo {
    fn read_stdin(&self) -> Vec<u8> {
        Vec::new()
    }

    fn out(&self, text: &str) {
        self.stdout.borrow_mut().push_str(text);
    }

    fn err(&self, text: &str) {
        self.stderr.borrow_mut().push_str(text);
    }
}

fn fenced_blocks<'a>(document: &'a str, language: &str) -> Vec<&'a str> {
    let opening = format!("```{language}\n");
    let closing = "```\n";
    let mut blocks = Vec::new();
    let mut search_from = 0;

    while let Some(opening_offset) = document[search_from..].find(&opening) {
        let body_start = search_from + opening_offset + opening.len();
        let Some(closing_offset) = document[body_start..].find(closing) else {
            break;
        };
        let body_end = body_start + closing_offset;
        blocks.push(&document[body_start..body_end]);
        search_from = body_end + closing.len();
    }

    blocks
}

fn run_cli(argv: &[&str]) -> (i32, String, String) {
    let io = FakeIo::new();
    let owned_argv: Vec<String> = argv.iter().map(|argument| (*argument).to_owned()).collect();
    let status = shexpand::cli::run(&owned_argv, &io);

    (
        status,
        io.stdout.borrow().clone(),
        io.stderr.borrow().clone(),
    )
}

#[test]
fn readme_usage_and_examples_match_the_public_cli() {
    let text_blocks = fenced_blocks(README, "text");
    assert_eq!(text_blocks.len(), 4);

    let expected_usage = "shexpand split LINE\nshexpand braces LINE\nshexpand fields [--ifs TEXT] VALUE\nshexpand glob [--file PATH]... PATTERN...\nshexpand expand [--var NAME=VALUE]... [--ifs TEXT] [--file PATH]... LINE [ARG...]\nshexpand quote [--style single|double] [--always] TEXT...\nshexpand lint LINE\n";
    assert_eq!(text_blocks[0], expected_usage);
    let (status, stdout, stderr) = run_cli(&["--help"]);
    assert_eq!(status, 0);
    assert_eq!(stdout, text_blocks[0]);
    assert_eq!(stderr, "");

    let expected_split = "word  parts\n1     u[cp]\n2     q[my file]\n3     p[${DEST:-/tmp}] u[/x]\n4     p[$@]\nwords       4\n";
    assert_eq!(text_blocks[1], expected_split);
    let (status, stdout, stderr) = run_cli(&["split", "cp \"my file\" ${DEST:-/tmp}/x $@ # note"]);
    assert_eq!(status, 0);
    assert_eq!(stdout, text_blocks[1]);
    assert_eq!(stderr, "");

    let expected_expand = "word  text\n1     [echo]\n2     [/home/u/x]\n3     [a]\n4     [b]\n5     [a b]\n6     []\n7     [y1]\n8     [y2]\n9     [q]\n10    [r]\nwords       10\n";
    assert_eq!(text_blocks[2], expected_expand);
    let (status, stdout, stderr) = run_cli(&[
        "expand",
        "--var",
        "X=a b",
        "--var",
        "E=",
        "--var",
        "HOME=/home/u",
        "echo ~/x $X \"$X\" $E \"$E\" y{1,2} ${U:-q r}",
    ]);
    assert_eq!(status, 0);
    assert_eq!(stdout, text_blocks[2]);
    assert_eq!(stderr, "");

    let expected_lint = "column 8: W1 unquoted parameter DIR is subject to splitting and pattern matching\ncolumn 16: W3 \"$*\" joins the parameters into one word; \"$@\" keeps them apart\ncolumn 20: W1 unquoted parameter X is subject to splitting and pattern matching\ncolumn 25: W1 unquoted parameter Y is subject to splitting and pattern matching\ncolumn 29: W2 unquoted $@ splits and matches its parameters again; write \"$@\"\nwarnings    5\n";
    assert_eq!(text_blocks[3], expected_lint);
    let (status, stdout, stderr) = run_cli(&["lint", "rm -rf $DIR/* \"$*\" ${X:-$Y} $@"]);
    assert_eq!(status, 1);
    assert_eq!(stdout, text_blocks[3]);
    assert_eq!(stderr, "");
}

#[test]
fn readme_example_invocations_are_the_complete_requested_commands() {
    assert_eq!(
        fenced_blocks(README, "sh"),
        vec![
            concat!(
                r#"shexpand split 'cp "my file" ${DEST:-/tmp}/x $@ # note'"#,
                "\n"
            ),
            concat!(
                r#"shexpand expand --var X='a b' --var E= --var HOME=/home/u 'echo ~/x $X "$X" $E "$E" y{1,2} ${U:-q r}'"#,
                "\n"
            ),
            concat!(r#"shexpand lint 'rm -rf $DIR/* "$*" ${X:-$Y} $@'"#, "\n"),
        ]
    );
}
