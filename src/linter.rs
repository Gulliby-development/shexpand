use crate::atom::{Atom, Kind, Word};

/// A lint diagnostic tied to the source column of its parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub col: usize,
    pub code: &'static str,
    pub message: String,
}

/// Find risky parameters, including parameters nested in operator words.
pub fn lint(words: &[Word]) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut pending: Vec<(&[Atom], bool)> = words
        .iter()
        .rev()
        .map(|word| (word.atoms.as_slice(), false))
        .collect();

    while let Some((atoms, inherited_quoted)) = pending.pop() {
        for atom in atoms {
            let Atom::Param(param) = atom else {
                continue;
            };

            let quoted = param.quoted || inherited_quoted;
            match param.name.as_str() {
                "#" => {}
                "@" | "*" if !quoted => findings.push(Finding {
                    col: param.col,
                    code: "W2",
                    message: format!(
                        "unquoted ${} splits and matches its parameters again; write \"$@\"",
                        param.name
                    ),
                }),
                "*" if quoted => findings.push(Finding {
                    col: param.col,
                    code: "W3",
                    message: "\"$*\" joins the parameters into one word; \"$@\" keeps them apart"
                        .to_owned(),
                }),
                _ if !quoted && param.kind != Kind::Length => findings.push(Finding {
                    col: param.col,
                    code: "W1",
                    message: format!(
                        "unquoted parameter {} is subject to splitting and pattern matching",
                        param.name
                    ),
                }),
                _ => {}
            }

            if !param.word.is_empty() {
                pending.push((param.word.as_slice(), quoted));
            }
        }
    }

    findings.sort_by(|left, right| {
        left.col
            .cmp(&right.col)
            .then_with(|| left.code.cmp(right.code))
    });
    findings
}
