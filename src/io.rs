use crate::error::ShError;

pub trait Io {
    fn read_stdin(&self) -> Vec<u8>;
    fn out(&self, text: &str);
    fn err(&self, text: &str);
}

pub fn text_arg(io: &dyn Io, arg: &str) -> Result<String, ShError> {
    if arg != "-" {
        return Ok(arg.to_owned());
    }

    let bytes = io.read_stdin();
    let mut text = String::from_utf8(bytes)
        .map_err(|_| ShError("<stdin>: input is not valid UTF-8".to_owned()))?;

    if text.starts_with('\u{feff}') {
        text.drain(..'\u{feff}'.len_utf8());
    }

    if text.ends_with("\r\n") {
        text.truncate(text.len() - 2);
    } else if text.ends_with('\n') {
        text.pop();
    }

    Ok(text)
}
