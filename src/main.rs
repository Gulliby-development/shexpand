use std::cell::{Cell, RefCell};
use std::io::{self, Read, Write};

use shexpand::cli;
use shexpand::io::Io;

struct RealIo {
    stdin: RefCell<io::Stdin>,
    stdout: RefCell<Vec<u8>>,
    stderr: RefCell<Vec<u8>>,
    failed: Cell<bool>,
}

impl RealIo {
    fn new() -> Self {
        Self {
            stdin: RefCell::new(io::stdin()),
            stdout: RefCell::new(Vec::new()),
            stderr: RefCell::new(Vec::new()),
            failed: Cell::new(false),
        }
    }

    fn mark_failed(&self) {
        self.failed.set(true);
    }

    fn finish(&self) -> bool {
        let (stdout, stderr) = match (self.stdout.try_borrow(), self.stderr.try_borrow()) {
            (Ok(stdout), Ok(stderr)) if !self.failed.get() => (stdout.clone(), stderr.clone()),
            _ => {
                self.mark_failed();
                (Vec::new(), Vec::new())
            }
        };

        let stdout_ok = write_and_flush(&mut io::stdout().lock(), &stdout);
        let stderr_ok = write_and_flush(&mut io::stderr().lock(), &stderr);
        if !stdout_ok || !stderr_ok {
            self.mark_failed();
        }
        stdout_ok && stderr_ok && !self.failed.get()
    }
}

impl Io for RealIo {
    fn read_stdin(&self) -> Vec<u8> {
        let Ok(stdin) = self.stdin.try_borrow_mut() else {
            self.mark_failed();
            return Vec::new();
        };
        let mut bytes = Vec::new();
        if stdin.lock().read_to_end(&mut bytes).is_err() {
            self.mark_failed();
            Vec::new()
        } else {
            bytes
        }
    }

    fn out(&self, text: &str) {
        if self.failed.get() {
            return;
        }
        if let Ok(mut stdout) = self.stdout.try_borrow_mut() {
            stdout.extend_from_slice(text.as_bytes());
        } else {
            self.mark_failed();
        }
    }

    fn err(&self, text: &str) {
        if self.failed.get() {
            return;
        }
        if let Ok(mut stderr) = self.stderr.try_borrow_mut() {
            stderr.extend_from_slice(text.as_bytes());
        } else {
            self.mark_failed();
        }
    }
}

fn write_and_flush<W: Write>(writer: &mut W, bytes: &[u8]) -> bool {
    let write_ok = writer.write_all(bytes).is_ok();
    let flush_ok = writer.flush().is_ok();
    write_ok && flush_ok
}

fn main() {
    let io = RealIo::new();
    let mut argv = Vec::new();
    let mut invalid_argument = false;
    for arg in std::env::args_os().skip(1) {
        match arg.into_string() {
            Ok(arg) => argv.push(arg),
            Err(_) => {
                invalid_argument = true;
                break;
            }
        }
    }

    let status = if invalid_argument {
        io.err("shexpand: argument is not valid Unicode\n");
        2
    } else {
        cli::run(&argv, &io)
    };
    let streams_ok = io.finish();
    std::process::exit(if streams_ok { status } else { 2 });
}
