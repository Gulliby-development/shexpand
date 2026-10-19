/// An item produced while expanding a shell word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Ch { ch: char, quoted: bool, split: bool },
    Break,
    Null,
}

/// Split expanded items using the supplied `IFS` characters.
///
/// `split` controls whether an individual character can act as a separator;
/// `quoted` is retained on characters that become part of a field.
pub fn split(items: &[Item], ifs: &str) -> Vec<Vec<(char, bool)>> {
    let mut fields = Vec::new();
    let mut field = Vec::new();
    let mut field_exists = false;
    // IFS whitespace adjacent to a non-whitespace delimiter is part of that
    // delimiter rather than an additional field boundary.
    let mut whitespace_ended_field = false;

    for item in items {
        match item {
            Item::Ch {
                ch, split: true, ..
            } if ifs.contains(*ch) => {
                if is_ifs_whitespace(*ch) {
                    if field_exists {
                        fields.push(std::mem::take(&mut field));
                        field_exists = false;
                        whitespace_ended_field = true;
                    }
                } else {
                    if field_exists {
                        fields.push(std::mem::take(&mut field));
                        field_exists = false;
                    } else if !whitespace_ended_field {
                        fields.push(Vec::new());
                    }
                    whitespace_ended_field = false;
                }
                // Separator characters never become part of a field.
            }
            Item::Ch { ch, quoted, .. } => {
                field.push((*ch, *quoted));
                field_exists = true;
                whitespace_ended_field = false;
            }
            Item::Null => {
                field_exists = true;
                whitespace_ended_field = false;
            }
            Item::Break => {
                if field_exists {
                    fields.push(std::mem::take(&mut field));
                    field_exists = false;
                }
                whitespace_ended_field = false;
            }
        }
    }

    if field_exists {
        fields.push(field);
    }

    fields
}

fn is_ifs_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n')
}
