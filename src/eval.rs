use std::collections::BTreeMap;

use crate::atom::{Atom, Kind, Param};
use crate::error::ShError;
use crate::fields::Item;
use crate::pattern::{Pattern, trim};

/// Mutable parameter state shared by all words in an expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scope {
    pub vars: BTreeMap<String, String>,
    pub positional: Vec<String>,
    pub assigned: Vec<(String, String)>,
}

impl Scope {
    pub fn new(vars: BTreeMap<String, String>, positional: Vec<String>) -> Self {
        Self {
            vars,
            positional,
            assigned: Vec::new(),
        }
    }
}

/// Evaluate a sequence of atoms without allowing literal source characters to split.
pub fn eval(atoms: &[Atom], scope: &mut Scope, ifs: &str) -> Result<Vec<Item>, ShError> {
    words(atoms, false, false, scope, ifs)
}

fn words(
    atoms: &[Atom],
    quote_context: bool,
    split_context: bool,
    scope: &mut Scope,
    ifs: &str,
) -> Result<Vec<Item>, ShError> {
    let mut result = Vec::new();

    for atom in atoms {
        match atom {
            Atom::Ch { ch, quoted } => {
                let quoted = *quoted || quote_context;
                result.push(Item::Ch {
                    ch: *ch,
                    quoted,
                    split: split_context && !quoted,
                });
            }
            Atom::Quote => result.push(Item::Null),
            Atom::Param(param) => {
                result.extend(parameter(param, quote_context, scope, ifs)?);
            }
        }
    }

    Ok(result)
}

fn parameter(
    param: &Param,
    quote_context: bool,
    scope: &mut Scope,
    ifs: &str,
) -> Result<Vec<Item>, ShError> {
    let quoted = param.quoted || quote_context;

    match param.kind {
        Kind::Plain if param.name == "@" => {
            return Ok(positional_items(&scope.positional, quoted));
        }
        Kind::Plain if param.name == "*" && !quoted => {
            return Ok(positional_items(&scope.positional, false));
        }
        Kind::Plain if param.name == "*" => {
            let separator = ifs.chars().next();
            let mut value = String::new();
            for (index, positional) in scope.positional.iter().enumerate() {
                if index > 0
                    && let Some(separator) = separator
                {
                    value.push(separator);
                }
                value.push_str(positional);
            }
            return Ok(value_items(&value, true));
        }
        Kind::Plain => {
            let value = parameter_value(&param.name, scope).unwrap_or_default();
            return Ok(value_items(&value, quoted));
        }
        Kind::Length => {
            let value = parameter_value(&param.name, scope).unwrap_or_default();
            let length = value.chars().count().to_string();
            return Ok(value_items(&length, quoted));
        }
        Kind::Prefix | Kind::Suffix => {
            let value = parameter_value(&param.name, scope).unwrap_or_default();
            let evaluated = words(&param.word, false, false, scope, ifs)?;
            let cells = evaluated
                .into_iter()
                .filter_map(|item| match item {
                    Item::Ch { ch, quoted, .. } => Some((ch, quoted)),
                    Item::Break | Item::Null => None,
                })
                .collect::<Vec<_>>();
            let pattern = Pattern::compile(&cells);
            let prefix = matches!(param.kind, Kind::Prefix);
            let value = trim(&value, &pattern, prefix, param.longest);
            return Ok(value_items(&value, quoted));
        }
        Kind::Default | Kind::Assign | Kind::Alt | Kind::Error => {}
    }

    let current = parameter_value(&param.name, scope);
    let empty_or_unset = current
        .as_ref()
        .is_none_or(|value| param.colon && value.is_empty());

    match param.kind {
        Kind::Default if empty_or_unset => words(&param.word, quoted, true, scope, ifs),
        Kind::Default => Ok(value_items(current.as_deref().unwrap_or_default(), quoted)),
        Kind::Alt if !empty_or_unset => words(&param.word, quoted, true, scope, ifs),
        Kind::Alt => Ok(Vec::new()),
        Kind::Assign if empty_or_unset => {
            if is_numeric_name(&param.name) {
                return Err(ShError(format!(
                    "{}: cannot assign in this way",
                    param.name
                )));
            }
            let value = flatten(&words(&param.word, true, false, scope, ifs)?);
            scope.vars.insert(param.name.clone(), value.clone());
            scope.assigned.push((param.name.clone(), value.clone()));
            Ok(value_items(&value, quoted))
        }
        Kind::Assign => Ok(value_items(current.as_deref().unwrap_or_default(), quoted)),
        Kind::Error if empty_or_unset => {
            let message = flatten(&words(&param.word, true, false, scope, ifs)?);
            let message = if message.is_empty() {
                if param.colon {
                    "parameter null or not set"
                } else {
                    "parameter not set"
                }
            } else {
                message.as_str()
            };
            Err(ShError(format!("{}: {message}", param.name)))
        }
        Kind::Error => Ok(value_items(current.as_deref().unwrap_or_default(), quoted)),
        Kind::Plain | Kind::Length | Kind::Prefix | Kind::Suffix => {
            Ok(value_items(current.as_deref().unwrap_or_default(), quoted))
        }
    }
}

fn parameter_value(name: &str, scope: &Scope) -> Option<String> {
    if name == "#" {
        return Some(scope.positional.len().to_string());
    }

    if name.chars().all(|ch| ch.is_ascii_digit()) && !name.is_empty() {
        let index = name.parse::<usize>().ok()?.checked_sub(1)?;
        return scope.positional.get(index).cloned();
    }

    scope.vars.get(name).cloned()
}

fn positional_items(positionals: &[String], quoted: bool) -> Vec<Item> {
    let mut items = Vec::new();

    for (index, value) in positionals.iter().enumerate() {
        if index > 0 {
            items.push(Item::Break);
        }
        if quoted {
            items.push(Item::Null);
        }
        items.extend(value.chars().map(|ch| Item::Ch {
            ch,
            quoted,
            split: !quoted,
        }));
    }

    items
}

fn value_items(value: &str, quoted: bool) -> Vec<Item> {
    let mut items = Vec::new();
    if quoted {
        items.push(Item::Null);
    }
    items.extend(value.chars().map(|ch| Item::Ch {
        ch,
        quoted,
        split: !quoted,
    }));
    items
}

fn flatten(items: &[Item]) -> String {
    let mut value = String::new();
    for item in items {
        match item {
            Item::Ch { ch, .. } => value.push(*ch),
            Item::Break => value.push(' '),
            Item::Null => {}
        }
    }
    value
}

fn is_numeric_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|ch| ch.is_ascii_digit())
}
