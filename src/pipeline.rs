use crate::atom::{Atom, Word};
use crate::braces;
use crate::eval::{Scope, eval};
use crate::fields;
use crate::pattern::Pattern;
use crate::vfs::Vfs;

/// Expand words through brace expansion, tilde expansion, evaluation, field splitting,
/// and virtual pathname expansion, in that order.
pub fn expand_words(
    words: &[Word],
    scope: &mut Scope,
    ifs: &str,
    vfs: &Vfs,
) -> Result<Vec<String>, crate::error::ShError> {
    let mut expanded = Vec::new();

    for word in words {
        for atoms in braces::expand(&word.atoms)? {
            let atoms = expand_tilde(atoms, scope);
            let items = eval(&atoms, scope, ifs)?;
            let split_fields = fields::split(&items, ifs);

            for field in split_fields {
                let pattern = Pattern::compile(&field);
                if pattern.magic {
                    let matches = vfs.expand(&field);
                    if matches.is_empty() {
                        expanded.push(field.iter().map(|(ch, _)| ch).collect());
                    } else {
                        expanded.extend(matches);
                    }
                } else {
                    expanded.push(field.iter().map(|(ch, _)| ch).collect());
                }
            }
        }
    }

    Ok(expanded)
}

fn expand_tilde(mut atoms: Vec<Atom>, scope: &Scope) -> Vec<Atom> {
    let eligible = matches!(
        atoms.first(),
        Some(Atom::Ch {
            ch: '~',
            quoted: false
        })
    ) && (atoms.len() == 1
        || matches!(
            atoms.get(1),
            Some(Atom::Ch {
                ch: '/',
                quoted: false
            })
        ));

    if eligible && let Some(home) = scope.vars.get("HOME") {
        let mut replacement = Vec::with_capacity(home.chars().count() + atoms.len());
        replacement.push(Atom::Quote);
        replacement.extend(home.chars().map(|ch| Atom::Ch { ch, quoted: true }));
        replacement.extend(atoms.drain(1..));
        atoms = replacement;
    }

    atoms
}
