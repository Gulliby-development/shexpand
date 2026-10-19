/// A compiled shell-style pattern.
pub struct Pattern {
    /// Whether the pattern contains a wildcard or a valid character set.
    pub magic: bool,
    tokens: Vec<Token>,
}

#[derive(Clone, Debug)]
enum Token {
    Literal(char),
    AnySequence,
    AnyCharacter,
    Set { negated: bool, members: Vec<Member> },
}

#[derive(Clone, Copy, Debug)]
enum Member {
    Literal(char),
    Range(char, char),
}

impl Pattern {
    /// Compile character cells, preserving the distinction between quoted and
    /// unquoted metacharacters.
    pub fn compile(cells: &[(char, bool)]) -> Self {
        let mut tokens = Vec::with_capacity(cells.len());
        let mut magic = false;
        let mut index = 0;

        while index < cells.len() {
            let (ch, quoted) = cells[index];
            if !quoted {
                match ch {
                    '*' => {
                        if !matches!(tokens.last(), Some(Token::AnySequence)) {
                            tokens.push(Token::AnySequence);
                        }
                        magic = true;
                        index += 1;
                        continue;
                    }
                    '?' => {
                        tokens.push(Token::AnyCharacter);
                        magic = true;
                        index += 1;
                        continue;
                    }
                    '[' => {
                        if let Some((token, next)) = compile_set(cells, index) {
                            tokens.push(token);
                            magic = true;
                            index = next;
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            tokens.push(Token::Literal(ch));
            index += 1;
        }

        Self { magic, tokens }
    }

    /// Match the entire character slice, not just a prefix or substring.
    pub fn matches(&self, text: &[char]) -> bool {
        let mut reachable = vec![false; text.len() + 1];
        reachable[0] = true;

        for token in &self.tokens {
            let mut next = vec![false; text.len() + 1];
            match token {
                Token::AnySequence => {
                    next[0] = reachable[0];
                    for position in 1..=text.len() {
                        next[position] = reachable[position] || next[position - 1];
                    }
                }
                Token::Literal(expected) => {
                    for position in 1..=text.len() {
                        next[position] = reachable[position - 1] && text[position - 1] == *expected;
                    }
                }
                Token::AnyCharacter => {
                    next[1..].copy_from_slice(&reachable[..text.len()]);
                }
                Token::Set { negated, members } => {
                    for position in 1..=text.len() {
                        let matches_member = members
                            .iter()
                            .any(|member| member.matches(text[position - 1]));
                        next[position] = reachable[position - 1] && (matches_member != *negated);
                    }
                }
            }
            reachable = next;
        }

        reachable[text.len()]
    }
}

impl Member {
    fn matches(self, ch: char) -> bool {
        match self {
            Self::Literal(expected) => ch == expected,
            Self::Range(low, high) => low <= ch && ch <= high,
        }
    }
}

fn compile_set(cells: &[(char, bool)], open: usize) -> Option<(Token, usize)> {
    let mut start = open.checked_add(1)?;
    let mut negated = false;

    if let Some(&(ch, quoted)) = cells.get(start)
        && ch == '!'
        && !quoted
    {
        negated = true;
        start = start.checked_add(1)?;
    }

    // Search after the first member; a leading plain ] is a member too.
    let close_search_from = start.checked_add(1)?;
    let close = (close_search_from..cells.len())
        .find(|&position| cells[position].0 == ']' && !cells[position].1)?;

    let content = cells.get(start..close)?;
    let members = compile_members(content);

    Some((Token::Set { negated, members }, close.checked_add(1)?))
}

fn compile_members(cells: &[(char, bool)]) -> Vec<Member> {
    let mut members = Vec::with_capacity(cells.len());
    let mut index = 0;

    while index < cells.len() {
        let (ch, _) = cells[index];
        let range_end = index.checked_add(2);
        if let Some(end) = range_end
            && end < cells.len()
            && cells[index + 1].0 == '-'
            && !cells[index + 1].1
            && !(index == 0 && ch == '-')
        {
            members.push(Member::Range(ch, cells[end].0));
            index = end + 1;
            continue;
        }

        members.push(Member::Literal(ch));
        index += 1;
    }

    members
}

/// Remove the shortest or longest matching prefix or suffix from a value.
pub fn trim(value: &str, pattern: &Pattern, prefix: bool, longest: bool) -> String {
    let chars: Vec<char> = value.chars().collect();

    if prefix {
        if longest {
            for length in (0..=chars.len()).rev() {
                if pattern.matches(&chars[..length]) {
                    return chars[length..].iter().collect();
                }
            }
        } else {
            for length in 0..=chars.len() {
                if pattern.matches(&chars[..length]) {
                    return chars[length..].iter().collect();
                }
            }
        }
    } else if longest {
        for length in (0..=chars.len()).rev() {
            let start = chars.len() - length;
            if pattern.matches(&chars[start..]) {
                return chars[..start].iter().collect();
            }
        }
    } else {
        for length in 0..=chars.len() {
            let start = chars.len() - length;
            if pattern.matches(&chars[start..]) {
                return chars[..start].iter().collect();
            }
        }
    }

    value.to_owned()
}
