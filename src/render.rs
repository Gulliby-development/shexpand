use crate::atom::Atom;
use crate::text::cell;

/// Render atoms as plain, quoted, and opaque parameter parts.
pub fn parts(atoms: &[Atom]) -> Vec<String> {
    let mut rendered = Vec::new();
    let mut run: Option<(bool, String)> = None;

    for atom in atoms {
        match atom {
            Atom::Ch { ch, quoted } => {
                ensure_run(&mut rendered, &mut run, *quoted);
                if let Some((_, text)) = run.as_mut() {
                    text.push(*ch);
                }
            }
            Atom::Quote => ensure_run(&mut rendered, &mut run, true),
            Atom::Param(param) => {
                flush_run(&mut rendered, &mut run);
                let marker = if param.quoted { 'P' } else { 'p' };
                rendered.push(format!("{marker}{}", cell(&param.text)));
            }
        }
    }

    flush_run(&mut rendered, &mut run);
    rendered
}

fn ensure_run(rendered: &mut Vec<String>, run: &mut Option<(bool, String)>, quoted: bool) {
    if run
        .as_ref()
        .is_some_and(|(run_quoted, _)| *run_quoted != quoted)
    {
        flush_run(rendered, run);
    }

    run.get_or_insert_with(|| (quoted, String::new()));
}

fn flush_run(rendered: &mut Vec<String>, run: &mut Option<(bool, String)>) {
    if let Some((quoted, text)) = run.take() {
        let marker = if quoted { 'q' } else { 'u' };
        rendered.push(format!("{marker}{}", cell(&text)));
    }
}

/// Concatenate source characters and opaque parameter text without sanitizing it.
pub fn atom_text(atoms: &[Atom]) -> String {
    let mut text = String::new();
    for atom in atoms {
        match atom {
            Atom::Ch { ch, .. } => text.push(*ch),
            Atom::Quote => {}
            Atom::Param(param) => text.push_str(&param.text),
        }
    }
    text
}
