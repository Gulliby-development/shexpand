use crate::atom::{Atom, Kind, Param, Word};
use crate::error::ShError;
use crate::text::show;

pub fn lex(text: &str) -> Result<Vec<Word>, ShError> {
    Parser::new(text).lex()
}

struct ParamSpec {
    kind: Kind,
    name: String,
    colon: bool,
    longest: bool,
    word: Vec<Atom>,
    quoted: bool,
}

impl ParamSpec {
    fn simple(kind: Kind, name: String, quoted: bool) -> Self {
        Self {
            kind,
            name,
            colon: false,
            longest: false,
            word: Vec::new(),
            quoted,
        }
    }

    fn operated(
        kind: Kind,
        name: String,
        colon: bool,
        longest: bool,
        word: Vec<Atom>,
        quoted: bool,
    ) -> Self {
        Self {
            kind,
            name,
            colon,
            longest,
            word,
            quoted,
        }
    }
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
            pos: 0,
        }
    }

    fn lex(mut self) -> Result<Vec<Word>, ShError> {
        let mut words = Vec::new();
        let mut current: Option<Vec<Atom>> = None;

        while let Some(ch) = self.peek() {
            if is_blank(ch) {
                self.pos += 1;
                if let Some(atoms) = current.take() {
                    words.push(Word { atoms });
                }
                continue;
            }

            if ch == '#' && current.is_none() {
                while self.peek().is_some_and(|next| next != '\n') {
                    self.pos += 1;
                }
                continue;
            }

            match ch {
                '\\' => {
                    let slash_col = self.column();
                    self.pos += 1;
                    match self.peek() {
                        Some('\n') => self.pos += 1,
                        Some(next) => {
                            self.pos += 1;
                            current.get_or_insert_with(Vec::new).push(Atom::Ch {
                                ch: next,
                                quoted: true,
                            });
                        }
                        None => {
                            return Err(ShError(format!(
                                "trailing backslash at column {slash_col}"
                            )));
                        }
                    }
                }
                '\'' => {
                    let atoms = self.parse_single_quote()?;
                    current.get_or_insert_with(Vec::new).extend(atoms);
                }
                '"' => {
                    let atoms = self.parse_double_quote(0)?;
                    if !atoms.is_empty() {
                        current.get_or_insert_with(Vec::new).extend(atoms);
                    }
                }
                '`' => return Err(self.command_substitution_error()),
                '|' | '&' | ';' | '<' | '>' | '(' | ')' => {
                    return Err(self.operator_error(ch));
                }
                '$' => {
                    let atom = self.parse_dollar(false, 0)?;
                    current.get_or_insert_with(Vec::new).push(atom);
                }
                _ => {
                    self.pos += 1;
                    current
                        .get_or_insert_with(Vec::new)
                        .push(Atom::Ch { ch, quoted: false });
                }
            }
        }

        if let Some(atoms) = current {
            words.push(Word { atoms });
        }
        Ok(words)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn column(&self) -> usize {
        self.pos.saturating_add(1)
    }

    fn source(&self, start: usize, end: usize) -> String {
        self.chars.get(start..end).unwrap_or(&[]).iter().collect()
    }

    fn command_substitution_error(&self) -> ShError {
        ShError(format!(
            "command substitution is not supported at column {}",
            self.column()
        ))
    }

    fn operator_error(&self, ch: char) -> ShError {
        ShError(format!(
            "unsupported operator {} at column {}",
            show(&ch.to_string()),
            self.column()
        ))
    }

    fn parse_single_quote(&mut self) -> Result<Vec<Atom>, ShError> {
        let opening_col = self.column();
        self.pos += 1;
        let mut atoms = vec![Atom::Quote];

        while let Some(ch) = self.peek() {
            self.pos += 1;
            if ch == '\'' {
                return Ok(atoms);
            }
            atoms.push(Atom::Ch { ch, quoted: true });
        }

        Err(ShError(format!(
            "unterminated single quote starting at column {opening_col}"
        )))
    }

    fn parse_double_quote(&mut self, depth: usize) -> Result<Vec<Atom>, ShError> {
        let opening_col = self.column();
        self.pos += 1;
        let mut run = Vec::new();

        loop {
            match self.peek() {
                None => {
                    return Err(ShError(format!(
                        "unterminated double quote starting at column {opening_col}"
                    )));
                }
                Some('"') => {
                    self.pos += 1;
                    if !is_only_at_parameters(&run) {
                        run.insert(0, Atom::Quote);
                    }
                    return Ok(run);
                }
                Some('\\') => {
                    self.pos += 1;
                    match self.peek() {
                        Some('\n') => self.pos += 1,
                        Some('$' | '`' | '"' | '\\') => {
                            let ch = self.peek().unwrap_or('\\');
                            self.pos += 1;
                            run.push(Atom::Ch { ch, quoted: true });
                        }
                        Some(_) | None => {
                            run.push(Atom::Ch {
                                ch: '\\',
                                quoted: true,
                            });
                        }
                    }
                }
                Some('`') => return Err(self.command_substitution_error()),
                Some('$') => run.push(self.parse_dollar(true, depth)?),
                Some(ch) => {
                    self.pos += 1;
                    run.push(Atom::Ch { ch, quoted: true });
                }
            }
        }
    }

    fn parse_dollar(&mut self, quoted: bool, depth: usize) -> Result<Atom, ShError> {
        let start = self.pos;
        self.pos += 1;

        match self.peek() {
            Some('{') => self.parse_braced(start, quoted, depth),
            Some(ch) if is_name_start(ch) => {
                self.pos += 1;
                while self.peek().is_some_and(is_name_continue) {
                    self.pos += 1;
                }
                Ok(self.param_atom(
                    start,
                    ParamSpec::simple(Kind::Plain, self.source(start + 1, self.pos), quoted),
                ))
            }
            Some(ch @ '1'..='9') => {
                self.pos += 1;
                Ok(self.param_atom(
                    start,
                    ParamSpec::simple(Kind::Plain, ch.to_string(), quoted),
                ))
            }
            Some(ch @ ('@' | '*' | '#')) => {
                self.pos += 1;
                Ok(self.param_atom(
                    start,
                    ParamSpec::simple(Kind::Plain, ch.to_string(), quoted),
                ))
            }
            Some('0' | '?' | '$' | '!' | '-') => {
                self.pos += 1;
                Err(ShError(format!(
                    "unsupported parameter {} at column {}",
                    show(&self.source(start, self.pos)),
                    start.saturating_add(1)
                )))
            }
            Some('(') => Err(ShError(format!(
                "command substitution is not supported at column {}",
                start.saturating_add(1)
            ))),
            Some('\'' | '"') if !quoted => Err(ShError(format!(
                "unsupported quoting after $ at column {}",
                start.saturating_add(1)
            ))),
            _ => Ok(Atom::Ch { ch: '$', quoted }),
        }
    }

    fn parse_braced(&mut self, start: usize, quoted: bool, depth: usize) -> Result<Atom, ShError> {
        if depth >= 8 {
            return Err(ShError(format!(
                "nesting is deeper than 8 at column {}",
                start.saturating_add(1)
            )));
        }

        self.pos += 1;
        let nested_depth = depth + 1;

        if self.peek() == Some('#') {
            self.pos += 1;
            if self.peek() == Some('}') {
                self.pos += 1;
                return Ok(self.param_atom(
                    start,
                    ParamSpec::simple(Kind::Plain, "#".to_owned(), quoted),
                ));
            }

            let Some(name) = self.read_braced_name(false) else {
                return Err(self.bad_substitution(start));
            };
            if !valid_braced_name(&name, false) {
                return Err(self.bad_substitution(start));
            }
            if self.peek() == Some('}') {
                self.pos += 1;
                return Ok(self.param_atom(start, ParamSpec::simple(Kind::Length, name, quoted)));
            }
            if self.peek().is_none() {
                return Err(self.unterminated_parameter(start));
            }
            return Err(self.bad_substitution(start));
        }

        let Some(name) = self.read_braced_name(true) else {
            return Err(self.bad_substitution(start));
        };
        if !valid_braced_name(&name, true) {
            return Err(self.bad_substitution(start));
        }

        if self.peek() == Some('}') {
            self.pos += 1;
            return Ok(self.param_atom(start, ParamSpec::simple(Kind::Plain, name, quoted)));
        }
        if self.peek().is_none() {
            return Err(self.unterminated_parameter(start));
        }

        if name == "@" || name == "*" {
            return Err(self.bad_substitution(start));
        }

        let (kind, colon, longest) = match self.peek() {
            Some(':') => {
                self.pos += 1;
                let Some(operator) = self.peek() else {
                    return Err(self.bad_substitution(start));
                };
                let kind = match operator {
                    '-' => Kind::Default,
                    '=' => Kind::Assign,
                    '+' => Kind::Alt,
                    '?' => Kind::Error,
                    _ => return Err(self.bad_substitution(start)),
                };
                self.pos += 1;
                (kind, true, false)
            }
            Some('-') => {
                self.pos += 1;
                (Kind::Default, false, false)
            }
            Some('=') => {
                self.pos += 1;
                (Kind::Assign, false, false)
            }
            Some('+') => {
                self.pos += 1;
                (Kind::Alt, false, false)
            }
            Some('?') => {
                self.pos += 1;
                (Kind::Error, false, false)
            }
            Some('#') => {
                self.pos += 1;
                let longest = if self.peek() == Some('#') {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                (Kind::Prefix, false, longest)
            }
            Some('%') => {
                self.pos += 1;
                let longest = if self.peek() == Some('%') {
                    self.pos += 1;
                    true
                } else {
                    false
                };
                (Kind::Suffix, false, longest)
            }
            _ => return Err(self.bad_substitution(start)),
        };

        let word = self.parse_operator_word(nested_depth, quoted)?;
        if self.peek() != Some('}') {
            return Err(self.unterminated_parameter(start));
        }
        self.pos += 1;
        Ok(self.param_atom(
            start,
            ParamSpec::operated(kind, name, colon, longest, word, quoted),
        ))
    }

    fn read_braced_name(&mut self, allow_special: bool) -> Option<String> {
        let start = self.pos;
        match self.peek()? {
            ch if is_name_start(ch) => {
                self.pos += 1;
                while self.peek().is_some_and(is_name_continue) {
                    self.pos += 1;
                }
            }
            '0'..='9' => {
                while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
                    self.pos += 1;
                }
            }
            '@' | '*' if allow_special => self.pos += 1,
            _ => return None,
        }
        Some(self.source(start, self.pos))
    }

    fn parse_operator_word(
        &mut self,
        depth: usize,
        double_context: bool,
    ) -> Result<Vec<Atom>, ShError> {
        let mut atoms = Vec::new();

        loop {
            let Some(ch) = self.peek() else {
                return Ok(atoms);
            };

            if ch == '}' {
                return Ok(atoms);
            }

            if double_context {
                match ch {
                    '\\' => {
                        self.pos += 1;
                        match self.peek() {
                            Some('\n') => self.pos += 1,
                            Some('$' | '`' | '"' | '\\' | '}') => {
                                let escaped = self.peek().unwrap_or('\\');
                                self.pos += 1;
                                atoms.push(Atom::Ch {
                                    ch: escaped,
                                    quoted: true,
                                });
                            }
                            Some(_) | None => atoms.push(Atom::Ch {
                                ch: '\\',
                                quoted: false,
                            }),
                        }
                    }
                    '"' => {
                        return Err(ShError(format!(
                            "unsupported double quote inside \"${{...}}\" at column {}",
                            self.column()
                        )));
                    }
                    '`' => return Err(self.command_substitution_error()),
                    '$' => atoms.push(self.parse_dollar(true, depth)?),
                    _ => {
                        self.pos += 1;
                        atoms.push(Atom::Ch { ch, quoted: false });
                    }
                }
                continue;
            }

            match ch {
                '\'' => atoms.extend(self.parse_single_quote()?),
                '"' => atoms.extend(self.parse_double_quote(depth)?),
                '\\' => {
                    let slash_col = self.column();
                    self.pos += 1;
                    match self.peek() {
                        Some('\n') => self.pos += 1,
                        Some(next) => {
                            self.pos += 1;
                            atoms.push(Atom::Ch {
                                ch: next,
                                quoted: true,
                            });
                        }
                        None => {
                            return Err(ShError(format!(
                                "trailing backslash at column {slash_col}"
                            )));
                        }
                    }
                }
                '`' => return Err(self.command_substitution_error()),
                '$' => atoms.push(self.parse_dollar(false, depth)?),
                _ => {
                    self.pos += 1;
                    atoms.push(Atom::Ch { ch, quoted: false });
                }
            }
        }
    }

    fn bad_substitution(&self, start: usize) -> ShError {
        let end = self.chars[start..]
            .iter()
            .position(|ch| *ch == '}')
            .map_or(self.chars.len(), |offset| start + offset + 1);
        ShError(format!(
            "bad substitution {} at column {}",
            show(&self.source(start, end)),
            start.saturating_add(1)
        ))
    }

    fn unterminated_parameter(&self, start: usize) -> ShError {
        ShError(format!(
            "unterminated {} starting at column {}",
            concat!("$", "{"),
            start.saturating_add(1)
        ))
    }

    fn param_atom(&self, start: usize, spec: ParamSpec) -> Atom {
        Atom::Param(Box::new(Param {
            kind: spec.kind,
            name: spec.name,
            colon: spec.colon,
            longest: spec.longest,
            word: spec.word,
            text: self.source(start, self.pos),
            col: start.saturating_add(1),
            quoted: spec.quoted,
        }))
    }
}

fn is_blank(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\r' | '\n')
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_name_continue(ch: char) -> bool {
    is_name_start(ch) || ch.is_ascii_digit()
}

fn valid_braced_name(name: &str, allow_special: bool) -> bool {
    if allow_special && matches!(name, "@" | "*") {
        return true;
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) if is_name_start(first) => chars.all(is_name_continue),
        Some(first @ '1'..='9') => {
            first.is_ascii_digit() && chars.all(|ch| ch.is_ascii_digit()) && name.len() <= 2
        }
        _ => false,
    }
}

fn is_only_at_parameters(atoms: &[Atom]) -> bool {
    !atoms.is_empty()
        && atoms.iter().all(|atom| {
            matches!(
                atom,
                Atom::Param(param) if param.kind == Kind::Plain && param.name == "@"
            )
        })
}
