use crate::atom::Atom;
use crate::error::ShError;

const MAX_WORDS: usize = 1000;
const OVER_LIMIT: usize = MAX_WORDS + 1;
const MAX_DEPTH: usize = 8;

#[derive(Debug)]
struct Plan {
    parts: Vec<Part>,
}

#[derive(Debug)]
enum Part {
    Literal { start: usize, end: usize },
    Choice(Vec<Plan>),
    Range(Range),
}

#[derive(Debug, Clone)]
enum Range {
    Integer {
        start: i64,
        end: i64,
        step: u64,
        width: usize,
        padded: bool,
    },
    Letter {
        start: u8,
        end: u8,
        step: u64,
    },
}

#[derive(Debug)]
struct Candidate {
    close: usize,
    commas: Vec<usize>,
}

/// Expand plain brace alternatives and ranges without changing opaque atoms.
pub fn expand(atoms: &[Atom]) -> Result<Vec<Vec<Atom>>, ShError> {
    let plan = parse_plan(atoms, 0, atoms.len(), 0)?;
    if count_plan(&plan) > MAX_WORDS {
        return Err(too_many_words());
    }
    Ok(emit_plan(atoms, &plan))
}

fn too_many_words() -> ShError {
    ShError("brace expansion makes more than 1000 words".to_owned())
}

fn too_deep() -> ShError {
    ShError("nesting is deeper than 8".to_owned())
}

fn plain_char(atom: &Atom, expected: char) -> bool {
    matches!(atom, Atom::Ch { ch, quoted: false } if *ch == expected)
}

fn parse_plan(atoms: &[Atom], start: usize, end: usize, depth: usize) -> Result<Plan, ShError> {
    let mut parts = Vec::new();
    let mut literal_start = start;
    let mut scan = start;
    let mut fresh_start = start;

    while scan < end {
        let Some(open) = (scan..end).find(|&index| plain_char(&atoms[index], '{')) else {
            break;
        };

        // An adjacent empty pair at the beginning of a fresh slice is literal.
        if open == fresh_start && open + 1 < end && plain_char(&atoms[open + 1], '}') {
            parts.push(Part::Literal {
                start: open,
                end: open + 2,
            });
            literal_start = open + 2;
            scan = open + 2;
            continue;
        }

        let Some(candidate) = scan_candidate(atoms, open, end, depth)? else {
            scan = open + 1;
            continue;
        };

        let part = if candidate.commas.is_empty() {
            let Some(range) = parse_range(&atoms[open + 1..candidate.close]) else {
                scan = open + 1;
                continue;
            };
            Part::Range(range)
        } else {
            let mut choices = Vec::with_capacity(candidate.commas.len() + 1);
            let mut item_start = open + 1;
            for comma in candidate.commas {
                choices.push(parse_plan(atoms, item_start, comma, depth + 1)?);
                item_start = comma + 1;
            }
            choices.push(parse_plan(atoms, item_start, candidate.close, depth + 1)?);
            Part::Choice(choices)
        };

        if literal_start < open {
            parts.push(Part::Literal {
                start: literal_start,
                end: open,
            });
        }
        parts.push(part);
        literal_start = candidate.close + 1;
        fresh_start = literal_start;
        scan = literal_start;
    }

    if literal_start < end {
        parts.push(Part::Literal {
            start: literal_start,
            end,
        });
    }

    Ok(Plan { parts })
}

fn scan_candidate(
    atoms: &[Atom],
    open: usize,
    end: usize,
    enclosing_depth: usize,
) -> Result<Option<Candidate>, ShError> {
    if enclosing_depth >= MAX_DEPTH {
        return Err(too_deep());
    }

    let mut nested = 0usize;
    let mut eligible = false;
    let mut commas = Vec::new();
    let mut index = open + 1;

    while index < end {
        if plain_char(&atoms[index], '{') {
            nested += 1;
            if enclosing_depth + 1 + nested > MAX_DEPTH {
                return Err(too_deep());
            }
        } else if plain_char(&atoms[index], '}') {
            if nested > 0 {
                nested -= 1;
            } else if eligible {
                return Ok(Some(Candidate {
                    close: index,
                    commas,
                }));
            }
        } else if nested == 0 && plain_char(&atoms[index], ',') {
            commas.push(index);
            eligible = true;
        } else if nested == 0
            && plain_char(&atoms[index], '.')
            && index + 1 < end
            && plain_char(&atoms[index + 1], '.')
            && !(index + 2 < end && plain_char(&atoms[index + 2], '}'))
        {
            eligible = true;
        }

        index += 1;
    }

    Ok(None)
}

fn parse_range(atoms: &[Atom]) -> Option<Range> {
    let mut text = String::new();
    for atom in atoms {
        match atom {
            Atom::Ch { ch, quoted: false } => text.push(*ch),
            _ => return None,
        }
    }

    let mut fields = text.split("..");
    let first = fields.next()?;
    let second = fields.next()?;
    let third = fields.next();
    if fields.next().is_some() {
        return None;
    }

    if let (Some((start, start_padded)), Some((end, end_padded))) =
        (parse_integer(first), parse_integer(second))
    {
        let distance = start.abs_diff(end);
        let step = match third {
            Some(text) => parse_step(text, distance)?,
            None => 1,
        };
        return Some(Range::Integer {
            start,
            end,
            step,
            width: first.len().max(second.len()),
            padded: start_padded || end_padded,
        });
    }

    let start_letter = parse_letter(first)?;
    let end_letter = parse_letter(second)?;
    if start_letter.is_ascii_lowercase() != end_letter.is_ascii_lowercase() {
        return None;
    }

    let distance = u64::from(start_letter.abs_diff(end_letter));
    let step = match third {
        Some(text) => parse_step(text, distance)?,
        None => 1,
    };
    Some(Range::Letter {
        start: start_letter,
        end: end_letter,
        step,
    })
}

fn parse_integer(text: &str) -> Option<(i64, bool)> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text),
    };
    if digits.is_empty() || digits.len() > 9 || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut value = 0i64;
    for byte in digits.bytes() {
        value = value * 10 + i64::from(byte - b'0');
    }
    if negative {
        value = -value;
    }
    let padded = digits.len() > 1 && digits.starts_with('0');
    Some((value, padded))
}

fn parse_letter(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    if bytes.len() == 1 && bytes[0].is_ascii_alphabetic() {
        Some(bytes[0])
    } else {
        None
    }
}

fn parse_step(text: &str, distance: u64) -> Option<u64> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !matches!(bytes[0], b'1'..=b'9') {
        return None;
    }

    let cap = distance + 1;
    let mut value = 0u64;
    for byte in bytes {
        if !byte.is_ascii_digit() {
            return None;
        }
        value = value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
            .min(cap);
    }
    Some(value)
}

fn count_plan(plan: &Plan) -> usize {
    let mut count = 1usize;
    for part in &plan.parts {
        count = capped_product(count, count_part(part));
    }
    count
}

fn count_part(part: &Part) -> usize {
    match part {
        Part::Literal { .. } => 1,
        Part::Range(range) => range.count(),
        Part::Choice(choices) => {
            let mut count = 0usize;
            for choice in choices {
                count = capped_sum(count, count_plan(choice));
            }
            count
        }
    }
}

fn capped_sum(left: usize, right: usize) -> usize {
    left.saturating_add(right).min(OVER_LIMIT)
}

fn capped_product(left: usize, right: usize) -> usize {
    if left == 0 || right == 0 {
        return 0;
    }
    if left > MAX_WORDS / right {
        OVER_LIMIT
    } else {
        left * right
    }
}

impl Range {
    fn count(&self) -> usize {
        let count = match self {
            Range::Integer {
                start, end, step, ..
            } => start.abs_diff(*end) / step + 1,
            Range::Letter { start, end, step } => u64::from(start.abs_diff(*end)) / step + 1,
        };
        if count > MAX_WORDS as u64 {
            OVER_LIMIT
        } else {
            count as usize
        }
    }
}

fn emit_plan(atoms: &[Atom], plan: &Plan) -> Vec<Vec<Atom>> {
    let mut words = vec![Vec::new()];
    for part in &plan.parts {
        let suffixes = emit_part(atoms, part);
        let mut combined = Vec::new();
        for prefix in words {
            for suffix in &suffixes {
                let mut word = prefix.clone();
                word.extend_from_slice(suffix);
                combined.push(word);
            }
        }
        words = combined;
    }
    words
}

fn emit_part(atoms: &[Atom], part: &Part) -> Vec<Vec<Atom>> {
    match part {
        Part::Literal { start, end } => vec![atoms[*start..*end].to_vec()],
        Part::Range(range) => emit_range(range),
        Part::Choice(choices) => {
            let mut words = Vec::new();
            for choice in choices {
                words.extend(emit_plan(atoms, choice));
            }
            words
        }
    }
}

fn emit_range(range: &Range) -> Vec<Vec<Atom>> {
    match range {
        Range::Integer {
            start,
            end,
            step,
            width,
            padded,
        } => {
            let mut values = Vec::new();
            let mut current = *start;
            loop {
                let text = if *padded {
                    padded_integer(current, *width)
                } else {
                    current.to_string()
                };
                values.push(text_atoms(&text));
                if current < *end {
                    let remaining = current.abs_diff(*end);
                    if *step > remaining {
                        break;
                    }
                    current += *step as i64;
                } else if current > *end {
                    let remaining = current.abs_diff(*end);
                    if *step > remaining {
                        break;
                    }
                    current -= *step as i64;
                } else {
                    break;
                }
            }
            values
        }
        Range::Letter { start, end, step } => {
            let mut values = Vec::new();
            let mut current = i64::from(*start);
            let end = i64::from(*end);
            loop {
                values.push(text_atoms(&char::from(current as u8).to_string()));
                if current < end {
                    let remaining = (end - current) as u64;
                    if *step > remaining {
                        break;
                    }
                    current += *step as i64;
                } else if current > end {
                    let remaining = (current - end) as u64;
                    if *step > remaining {
                        break;
                    }
                    current -= *step as i64;
                } else {
                    break;
                }
            }
            values
        }
    }
}

fn padded_integer(value: i64, width: usize) -> String {
    let number = value.to_string();
    let (sign, digits) = match number.strip_prefix('-') {
        Some(digits) => (Some('-'), digits),
        None => (None, number.as_str()),
    };
    let occupied = digits.len() + usize::from(sign.is_some());
    let mut result = String::with_capacity(width.max(occupied));
    if let Some(sign) = sign {
        result.push(sign);
    }
    result.extend(std::iter::repeat_n('0', width.saturating_sub(occupied)));
    result.push_str(digits);
    result
}

fn text_atoms(text: &str) -> Vec<Atom> {
    text.chars()
        .map(|ch| Atom::Ch { ch, quoted: false })
        .collect()
}
