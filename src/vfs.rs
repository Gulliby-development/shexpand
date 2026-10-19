use std::collections::BTreeMap;

use crate::error::ShError;
use crate::pattern::Pattern;
use crate::text::show;

#[derive(Default)]
struct Node {
    is_file: bool,
    children: BTreeMap<String, usize>,
}

/// An in-memory collection of relative and absolute virtual paths.
pub struct Vfs {
    nodes: Vec<Node>,
    relative_root: usize,
    absolute_root: usize,
}

impl Vfs {
    /// Build independent path trees from file names, adding implied directories.
    pub fn new(paths: &[String]) -> Result<Self, ShError> {
        let mut vfs = Self {
            nodes: vec![Node::default(), Node::default()],
            relative_root: 0,
            absolute_root: 1,
        };

        for path in paths {
            let Some((absolute, names)) = parse_path(path) else {
                return Err(ShError(format!(
                    "invalid --file {} (expected a path of names separated by single slashes)",
                    show(path)
                )));
            };

            let root = if absolute {
                vfs.absolute_root
            } else {
                vfs.relative_root
            };
            vfs.insert(root, &names, path)?;
        }

        Ok(vfs)
    }

    /// Expand slash-separated pattern cells against the virtual file trees.
    pub fn expand(&self, cells: &[(char, bool)]) -> Vec<String> {
        if cells.is_empty() {
            return Vec::new();
        }

        if cells.len() == 1 && cells[0].0 == '/' {
            return vec!["/".to_owned()];
        }

        let absolute = cells.first().is_some_and(|(ch, _)| *ch == '/');
        let trailing_slash = cells.last().is_some_and(|(ch, _)| *ch == '/');
        let mut parts = Vec::new();
        let mut part = Vec::new();

        for &(ch, quoted) in cells {
            if ch == '/' {
                parts.push(std::mem::take(&mut part));
            } else {
                part.push((ch, quoted));
            }
        }
        parts.push(part);

        if absolute {
            parts.remove(0);
        }
        if trailing_slash {
            parts.pop();
        }
        if parts.is_empty() || parts.iter().any(Vec::is_empty) {
            return Vec::new();
        }

        let root = if absolute {
            self.absolute_root
        } else {
            self.relative_root
        };
        let mut candidates = vec![(
            root,
            if absolute {
                "/".to_owned()
            } else {
                String::new()
            },
        )];

        for (part_index, part) in parts.iter().enumerate() {
            let is_last = part_index + 1 == parts.len();
            let pattern = Pattern::compile(part);
            let starts_with_dot = part.first().is_some_and(|(ch, _)| *ch == '.');
            let mut next = Vec::new();

            for (parent_index, parent_path) in candidates {
                let parent = &self.nodes[parent_index];
                for (name, &node_index) in &parent.children {
                    if pattern.magic && name.starts_with('.') && !starts_with_dot {
                        continue;
                    }
                    let name_chars: Vec<char> = name.chars().collect();
                    if !pattern.matches(&name_chars) {
                        continue;
                    }

                    let node = &self.nodes[node_index];
                    if (!is_last || trailing_slash) && node.children.is_empty() {
                        continue;
                    }

                    let path = append_path(&parent_path, name);
                    next.push((node_index, path));
                }
            }

            candidates = next;
            if candidates.is_empty() {
                return Vec::new();
            }
        }

        candidates
            .into_iter()
            .map(|(_, mut path)| {
                if trailing_slash && !path.ends_with('/') {
                    path.push('/');
                }
                path
            })
            .collect()
    }

    fn insert(&mut self, root: usize, names: &[&str], original: &str) -> Result<(), ShError> {
        let mut current = root;

        for (index, name) in names.iter().enumerate() {
            let is_last = index + 1 == names.len();
            let next = if let Some(&child) = self.nodes[current].children.get(*name) {
                child
            } else {
                let child = self.nodes.len();
                self.nodes.push(Node::default());
                self.nodes[current]
                    .children
                    .insert((*name).to_owned(), child);
                child
            };

            if is_last {
                if !self.nodes[next].children.is_empty() {
                    return Err(ShError(format!(
                        "{} is both a file and a directory",
                        show(original)
                    )));
                }
                self.nodes[next].is_file = true;
            } else if self.nodes[next].is_file {
                return Err(ShError(format!(
                    "{} is both a file and a directory",
                    show(original)
                )));
            }

            current = next;
        }

        Ok(())
    }
}

fn parse_path(path: &str) -> Option<(bool, Vec<&str>)> {
    if path.is_empty() || path.ends_with('/') {
        return None;
    }

    let absolute = path.starts_with('/');
    let body = if absolute {
        path.strip_prefix('/')?
    } else {
        path
    };
    if body.is_empty() {
        return None;
    }

    let names: Vec<&str> = body.split('/').collect();
    if names
        .iter()
        .any(|name| name.is_empty() || *name == "." || *name == "..")
    {
        return None;
    }

    Some((absolute, names))
}

fn append_path(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    }
}
