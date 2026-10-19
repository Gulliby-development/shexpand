#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Ch { ch: char, quoted: bool },
    Quote,
    Param(Box<Param>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Plain,
    Length,
    Default,
    Assign,
    Alt,
    Error,
    Prefix,
    Suffix,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub kind: Kind,
    pub name: String,
    pub colon: bool,
    pub longest: bool,
    pub word: Vec<Atom>,
    pub text: String,
    pub col: usize,
    pub quoted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub atoms: Vec<Atom>,
}
