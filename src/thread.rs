//! Thread construction (mutt `sort = threads` style) and tree flattening.

use std::collections::HashMap;

use chrono::{DateTime, Local};

use crate::mail::Message;

/// A single row in the threaded index view.
#[derive(Debug, Clone)]
pub struct Row {
    /// The message shown on this row.
    pub message: Message,
    /// Tree-drawing prefix placed before the subject (empty for roots).
    pub prefix: String,
}

/// Tree links for one message, indexed in parallel with the message list.
struct Node {
    children: Vec<usize>,
    parent: Option<usize>,
}

/// Build threads from messages and flatten them into display rows.
///
/// Threads are ordered newest-first by the most recent message in the thread;
/// replies within a thread are ordered oldest-first, like mutt.
pub fn build_rows(messages: Vec<Message>) -> Vec<Row> {
    let n = messages.len();
    let mut nodes: Vec<Node> = (0..n)
        .map(|_| Node {
            children: Vec::new(),
            parent: None,
        })
        .collect();

    // Message-ID -> index. On duplicates, keep the first one seen.
    let mut by_id: HashMap<&str, usize> = HashMap::new();
    for (i, m) in messages.iter().enumerate() {
        if let Some(id) = &m.message_id {
            by_id.entry(id.as_str()).or_insert(i);
        }
    }

    // Link each message to its nearest known ancestor.
    for i in 0..n {
        let parent = messages[i]
            .references
            .iter()
            .rev()
            .filter_map(|r| by_id.get(r.as_str()).copied())
            .find(|&p| p != i && !is_ancestor(&nodes, i, p));
        if let Some(p) = parent {
            nodes[i].parent = Some(p);
            nodes[p].children.push(i);
        }
    }

    for node in nodes.iter_mut() {
        node.children.sort_by_key(|&c| messages[c].date);
    }

    // Roots ordered by latest activity in the thread, newest first.
    let mut roots: Vec<(usize, Option<DateTime<Local>>)> = (0..n)
        .filter(|&i| nodes[i].parent.is_none())
        .map(|i| (i, latest_date(&nodes, &messages, i)))
        .collect();
    roots.sort_by(|a, b| b.1.cmp(&a.1));

    let mut rows = Vec::with_capacity(n);
    for (root, _) in roots {
        flatten(&nodes, &messages, root, "", "", &mut rows);
    }
    rows
}

/// True if `candidate` is an ancestor of `of` (guards against reference cycles).
fn is_ancestor(nodes: &[Node], candidate: usize, of: usize) -> bool {
    let mut cur = nodes[of].parent;
    while let Some(p) = cur {
        if p == candidate {
            return true;
        }
        cur = nodes[p].parent;
    }
    false
}

/// Newest date in the subtree rooted at `i`; used to order threads.
fn latest_date(nodes: &[Node], messages: &[Message], i: usize) -> Option<DateTime<Local>> {
    let mut best = messages[i].date;
    for &c in &nodes[i].children {
        best = best.max(latest_date(nodes, messages, c));
    }
    best
}

/// Depth-first walk emitting rows with mutt-style tree prefixes.
fn flatten(
    nodes: &[Node],
    messages: &[Message],
    i: usize,
    indent: &str,
    branch: &str,
    rows: &mut Vec<Row>,
) {
    rows.push(Row {
        message: messages[i].clone(),
        prefix: format!("{indent}{branch}"),
    });
    let children = &nodes[i].children;
    for (k, &c) in children.iter().enumerate() {
        let last = k + 1 == children.len();
        // Children of a root are indented at column 0; deeper levels extend the
        // parent's indent with a bar (or blank if the parent was the last child).
        let child_indent = if branch.is_empty() {
            indent.to_string()
        } else if branch.starts_with('└') {
            format!("{indent}  ")
        } else {
            format!("{indent}│ ")
        };
        let child_branch = if last { "└─>" } else { "├─>" };
        flatten(nodes, messages, c, &child_indent, child_branch, rows);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn msg(uid: u32, day: u32, id: &str, refs: &[&str]) -> Message {
        Message {
            uid,
            date: Some(Local.with_ymd_and_hms(2026, 8, day, 12, 0, 0).unwrap()),
            sender: String::new(),
            subject: format!("m{uid}"),
            message_id: Some(id.to_string()),
            references: refs.iter().map(|s| s.to_string()).collect(),
            unread: false,
        }
    }

    #[test]
    fn threads_are_newest_first_and_replies_nested() {
        let messages = vec![
            msg(1, 1, "a", &[]),
            msg(2, 5, "b", &[]),
            msg(3, 2, "a1", &["a"]),
            msg(4, 3, "a2", &["a", "a1"]),
            msg(5, 4, "a3", &["a"]),
        ];
        let rows = build_rows(messages);
        let got: Vec<(u32, &str)> = rows.iter().map(|r| (r.message.uid, r.prefix.as_str())).collect();
        assert_eq!(
            got,
            vec![
                (2, ""),
                (1, ""),
                (3, "├─>"),
                (4, "│ └─>"),
                (5, "└─>"),
            ]
        );
    }
}
