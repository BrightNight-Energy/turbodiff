use crate::options::PrettyOptions;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Clone, Debug)]
struct ChangeEntry {
    segments: Vec<PathSegment>,
    kind: ChangeKind,
}

#[derive(Clone, Debug)]
enum ChangeKind {
    ValueChanged {
        old: Value,
        new: Value,
    },
    TypeChanged {
        old_type: String,
        new_type: String,
        old: Value,
        new: Value,
    },
    Added {
        value: Option<Value>,
    },
    Removed {
        value: Option<Value>,
    },
}

#[derive(Clone, Debug)]
struct PrettyNode {
    segment: Option<PathSegment>,
    children: Vec<PrettyNode>,
    change: Option<ChangeKind>,
}

impl PrettyNode {
    fn root() -> Self {
        Self {
            segment: None,
            children: Vec::new(),
            change: None,
        }
    }

    fn add_change(&mut self, segments: Vec<PathSegment>, kind: ChangeKind) {
        if segments.is_empty() {
            self.change = Some(kind);
            return;
        }
        let mut node = self;
        for segment in segments {
            let pos = node
                .children
                .iter()
                .position(|child| child.segment.as_ref() == Some(&segment));
            let idx = if let Some(idx) = pos {
                idx
            } else {
                node.children.push(PrettyNode {
                    segment: Some(segment.clone()),
                    children: Vec::new(),
                    change: None,
                });
                node.children.len() - 1
            };
            node = &mut node.children[idx];
        }
        node.change = Some(kind);
    }

    fn child(&self, segment: &PathSegment) -> Option<&PrettyNode> {
        self.children
            .iter()
            .find(|child| child.segment.as_ref() == Some(segment))
    }
}

pub(crate) fn render_pretty(
    result: &Value,
    t1: &Value,
    t2: &Value,
    options: PrettyOptions,
) -> String {
    let mut changes = collect_changes(result, t1, t2);
    if changes.is_empty() {
        return String::new();
    }

    if options.path_header {
        changes.sort_by(|a, b| {
            format_compact_path(&a.segments).cmp(&format_compact_path(&b.segments))
        });
        let mut lines = Vec::new();
        for change in changes {
            let path = format_compact_path(&change.segments);
            lines.push(path);
            append_change_lines(&mut lines, 0, &[], false, &change.kind, &options);
        }
        return lines.join("\n");
    }

    let tree = build_tree(changes);
    let mut lines = Vec::new();
    if let Some(change) = &tree.change {
        lines.push("root".to_string());
        append_change_lines(&mut lines, 0, &[], false, change, &options);
    }
    let env = RenderEnv {
        t1,
        t2,
        options: &options,
    };
    render_children(&tree, 0, &[], &[], &env, &mut lines);
    lines.join("\n")
}

fn collect_changes(result: &Value, t1: &Value, t2: &Value) -> Vec<ChangeEntry> {
    let mut changes = Vec::new();
    let Value::Object(map) = result else {
        return changes;
    };

    if let Some(Value::Object(values_changed)) = map.get("values_changed") {
        for (path, entry) in values_changed {
            if let Some(segments) = parse_path(path) {
                let old = get_value_at_path(t1, &segments)
                    .cloned()
                    .or_else(|| entry.get("old_value").cloned())
                    .unwrap_or(Value::Null);
                let new = get_value_at_path(t2, &segments)
                    .cloned()
                    .or_else(|| entry.get("new_value").cloned())
                    .unwrap_or(Value::Null);
                changes.push(ChangeEntry {
                    segments,
                    kind: ChangeKind::ValueChanged { old, new },
                });
            }
        }
    } else if let Some(Value::Array(values_changed)) = map.get("values_changed") {
        for path in values_changed {
            if let Value::String(path) = path {
                if let Some(segments) = parse_path(path) {
                    let old = get_value_at_path(t1, &segments)
                        .cloned()
                        .unwrap_or(Value::Null);
                    let new = get_value_at_path(t2, &segments)
                        .cloned()
                        .unwrap_or(Value::Null);
                    changes.push(ChangeEntry {
                        segments,
                        kind: ChangeKind::ValueChanged { old, new },
                    });
                }
            }
        }
    }

    if let Some(Value::Object(type_changes)) = map.get("type_changes") {
        for (path, entry) in type_changes {
            if let Some(segments) = parse_path(path) {
                let old_type = entry
                    .get("old_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let new_type = entry
                    .get("new_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let old = get_value_at_path(t1, &segments)
                    .cloned()
                    .or_else(|| entry.get("old_value").cloned())
                    .unwrap_or(Value::Null);
                let new = get_value_at_path(t2, &segments)
                    .cloned()
                    .or_else(|| entry.get("new_value").cloned())
                    .unwrap_or(Value::Null);
                changes.push(ChangeEntry {
                    segments,
                    kind: ChangeKind::TypeChanged {
                        old_type,
                        new_type,
                        old,
                        new,
                    },
                });
            }
        }
    }

    if let Some(Value::Array(added)) = map.get("dictionary_item_added") {
        for path in added {
            if let Value::String(path) = path {
                if let Some(segments) = parse_path(path) {
                    let value = get_value_at_path(t2, &segments).cloned();
                    changes.push(ChangeEntry {
                        segments,
                        kind: ChangeKind::Added { value },
                    });
                }
            }
        }
    }

    if let Some(Value::Array(removed)) = map.get("dictionary_item_removed") {
        for path in removed {
            if let Value::String(path) = path {
                if let Some(segments) = parse_path(path) {
                    let value = get_value_at_path(t1, &segments).cloned();
                    changes.push(ChangeEntry {
                        segments,
                        kind: ChangeKind::Removed { value },
                    });
                }
            }
        }
    }

    if let Some(Value::Object(added)) = map.get("iterable_item_added") {
        for (path, value) in added {
            if let Some(segments) = parse_path(path) {
                let value = get_value_at_path(t2, &segments)
                    .cloned()
                    .or_else(|| Some(value.clone()));
                changes.push(ChangeEntry {
                    segments,
                    kind: ChangeKind::Added { value },
                });
            }
        }
    }

    if let Some(Value::Object(removed)) = map.get("iterable_item_removed") {
        for (path, value) in removed {
            if let Some(segments) = parse_path(path) {
                let value = get_value_at_path(t1, &segments)
                    .cloned()
                    .or_else(|| Some(value.clone()));
                changes.push(ChangeEntry {
                    segments,
                    kind: ChangeKind::Removed { value },
                });
            }
        }
    }

    changes
}

fn build_tree(changes: Vec<ChangeEntry>) -> PrettyNode {
    let mut root = PrettyNode::root();
    for change in changes {
        root.add_change(change.segments, change.kind);
    }
    root
}

fn parse_path(path: &str) -> Option<Vec<PathSegment>> {
    if !path.starts_with("root") {
        return None;
    }
    let mut segments = Vec::new();
    let mut i = 4;
    while i < path.len() {
        if path[i..].starts_with("['") {
            i += 2;
            let end = path[i..].find("']")?;
            let key = &path[i..i + end];
            segments.push(PathSegment::Key(key.to_string()));
            i += end + 2;
        } else if path.as_bytes().get(i) == Some(&b'[') {
            i += 1;
            let end = path[i..].find(']')?;
            let idx = path[i..i + end].parse::<usize>().ok()?;
            segments.push(PathSegment::Index(idx));
            i += end + 1;
        } else {
            break;
        }
    }
    Some(segments)
}

fn get_value_at_path<'a>(root: &'a Value, segments: &[PathSegment]) -> Option<&'a Value> {
    let mut current = root;
    for segment in segments {
        match (segment, current) {
            (PathSegment::Key(key), Value::Object(map)) => {
                current = map.get(key)?;
            }
            (PathSegment::Index(idx), Value::Array(list)) => {
                current = list.get(*idx)?;
            }
            _ => return None,
        }
    }
    Some(current)
}

fn format_compact_path(segments: &[PathSegment]) -> String {
    if segments.is_empty() {
        return "root".to_string();
    }
    format_compact_segments(segments)
}

fn format_compact_segments(segments: &[PathSegment]) -> String {
    let mut out = String::new();
    for (idx, segment) in segments.iter().enumerate() {
        match segment {
            PathSegment::Key(key) => {
                if idx == 0 {
                    if is_simple_identifier(key) {
                        out.push_str(key);
                    } else {
                        out.push_str("['");
                        out.push_str(key);
                        out.push_str("']");
                    }
                } else if is_simple_identifier(key) {
                    out.push('.');
                    out.push_str(key);
                } else {
                    out.push_str("['");
                    out.push_str(key);
                    out.push_str("']");
                }
            }
            PathSegment::Index(i) => {
                out.push('[');
                out.push_str(&i.to_string());
                out.push(']');
            }
        }
    }
    out
}

fn is_simple_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn format_segment_label(segment: &PathSegment) -> String {
    match segment {
        PathSegment::Key(key) => key.to_string(),
        PathSegment::Index(i) => format_index_label(*i),
    }
}

fn format_index_label(index: usize) -> String {
    format!("[{}]", index)
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::String(s) => format!("'{}'", escape_string(s)),
        Value::Array(arr) => {
            let inner: Vec<String> = arr.iter().map(format_value).collect();
            format!("[{}]", inner.join(", "))
        }
        Value::Object(obj) => {
            let mut parts = Vec::with_capacity(obj.len());
            for (k, v) in obj {
                parts.push(format!("'{}': {}", escape_string(k), format_value(v)));
            }
            format!("{{{}}}", parts.join(", "))
        }
    }
}

fn escape_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\'' => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

enum RenderEntry<'a> {
    Node(&'a PrettyNode),
    Ellipsis,
    ContextIndex(usize),
}

struct RenderEnv<'a> {
    t1: &'a Value,
    t2: &'a Value,
    options: &'a PrettyOptions,
}

fn render_children(
    node: &PrettyNode,
    depth: usize,
    branches: &[bool],
    path: &[PathSegment],
    env: &RenderEnv<'_>,
    lines: &mut Vec<String>,
) {
    if node.children.is_empty() {
        return;
    }

    let v1 = get_value_at_path(env.t1, path);
    let v2 = get_value_at_path(env.t2, path);

    if let Some(keys) = object_keys_union(v1, v2) {
        let mut changed = HashSet::new();
        for child in &node.children {
            if let Some(PathSegment::Key(key)) = &child.segment {
                changed.insert(key.clone());
            }
        }

        let mut entries = Vec::new();
        let mut seen = false;
        let mut pending_ellipsis = false;
        for key in keys {
            if changed.contains(&key) {
                if pending_ellipsis {
                    entries.push(RenderEntry::Ellipsis);
                    pending_ellipsis = false;
                }
                if let Some(child) = node.child(&PathSegment::Key(key.clone())) {
                    entries.push(RenderEntry::Node(child));
                }
                seen = true;
            } else if seen {
                pending_ellipsis = true;
            }
        }
        render_entries(entries, depth, branches, path, env, lines);
        return;
    }

    if let Some(len) = array_length_union(v1, v2) {
        let mut changed_indices = HashSet::new();
        for child in &node.children {
            if let Some(PathSegment::Index(idx)) = child.segment.as_ref() {
                changed_indices.insert(*idx);
            }
        }

        let mut display_indices = HashSet::new();
        if env.options.context == 0 {
            display_indices = changed_indices.clone();
        } else {
            for idx in &changed_indices {
                let start = idx.saturating_sub(env.options.context);
                let end = idx
                    .saturating_add(env.options.context)
                    .min(len.saturating_sub(1));
                for i in start..=end {
                    display_indices.insert(i);
                }
            }
        }

        let mut entries = Vec::new();
        let mut seen = false;
        let mut pending_ellipsis = false;
        for idx in 0..len {
            if display_indices.contains(&idx) {
                if pending_ellipsis {
                    entries.push(RenderEntry::Ellipsis);
                    pending_ellipsis = false;
                }
                if let Some(child) = node.child(&PathSegment::Index(idx)) {
                    entries.push(RenderEntry::Node(child));
                } else {
                    entries.push(RenderEntry::ContextIndex(idx));
                }
                seen = true;
            } else if seen {
                pending_ellipsis = true;
            }
        }
        render_entries(entries, depth, branches, path, env, lines);
        return;
    }

    let mut ordered_children: Vec<&PrettyNode> = node.children.iter().collect();
    ordered_children.sort_by(|a, b| {
        format_segment_label(a.segment.as_ref().expect("segment must exist")).cmp(
            &format_segment_label(b.segment.as_ref().expect("segment must exist")),
        )
    });
    let entries: Vec<RenderEntry<'_>> = ordered_children
        .into_iter()
        .map(RenderEntry::Node)
        .collect();
    render_entries(entries, depth, branches, path, env, lines);
}

fn render_entries(
    entries: Vec<RenderEntry<'_>>,
    depth: usize,
    branches: &[bool],
    path: &[PathSegment],
    env: &RenderEnv<'_>,
    lines: &mut Vec<String>,
) {
    let len = entries.len();
    for (idx, entry) in entries.into_iter().enumerate() {
        let is_last = idx + 1 == len;
        match entry {
            RenderEntry::Node(child) => {
                render_node(child, depth, is_last, branches, path, env, lines);
            }
            RenderEntry::Ellipsis => lines.push(format_node_line(depth, branches, is_last, "...")),
            RenderEntry::ContextIndex(item_idx) => {
                render_context_item(depth, branches, is_last, path, item_idx, env, lines);
            }
        }
    }
}

fn render_node(
    node: &PrettyNode,
    depth: usize,
    is_last: bool,
    branches: &[bool],
    parent_path: &[PathSegment],
    env: &RenderEnv<'_>,
    lines: &mut Vec<String>,
) {
    let (label, node_ref, node_path) = if env.options.compact {
        compress_node(node, parent_path)
    } else {
        let segment = node
            .segment
            .as_ref()
            .expect("non-root node must have a segment");
        let mut next_path = parent_path.to_vec();
        next_path.push(segment.clone());
        (format_segment_label(segment), node, next_path)
    };

    lines.push(format_node_line(depth, branches, is_last, &label));

    if let Some(change) = &node_ref.change {
        append_change_lines(lines, depth, branches, !is_last, change, env.options);
    }

    let mut child_branches = branches.to_vec();
    if depth > 0 {
        child_branches.push(!is_last);
    }

    if depth >= env.options.max_depth {
        if !node_ref.children.is_empty() {
            lines.push(format_node_line(depth + 1, &child_branches, true, "..."));
        }
        return;
    }

    render_children(node_ref, depth + 1, &child_branches, &node_path, env, lines);
}

fn compress_node<'a>(
    node: &'a PrettyNode,
    parent_path: &[PathSegment],
) -> (String, &'a PrettyNode, Vec<PathSegment>) {
    let mut parts = Vec::new();
    let mut current = node;
    let mut path = parent_path.to_vec();

    if let Some(segment) = &current.segment {
        parts.push(segment.clone());
        path.push(segment.clone());
    }

    while current.change.is_none() && current.children.len() == 1 {
        let child = &current.children[0];
        if let Some(segment) = &child.segment {
            parts.push(segment.clone());
            path.push(segment.clone());
        }
        current = child;
        if current.change.is_some() || current.children.len() != 1 {
            break;
        }
    }

    (format_compact_segments(&parts), current, path)
}

fn format_node_line(depth: usize, branches: &[bool], is_last: bool, label: &str) -> String {
    if depth == 0 {
        label.to_string()
    } else {
        let mut out = tree_prefix(branches);
        out.push_str(if is_last { "╰── " } else { "├── " });
        out.push_str(label);
        out
    }
}

fn append_change_lines(
    lines: &mut Vec<String>,
    depth: usize,
    branches: &[bool],
    node_has_more: bool,
    change: &ChangeKind,
    options: &PrettyOptions,
) {
    let indent = branch_indent(depth, branches, node_has_more);
    match change {
        ChangeKind::ValueChanged { old, new } => {
            lines.push(format!(
                "{}{}",
                indent,
                colorize(&format!("- {}", format_value(old)), "31", !options.no_color)
            ));
            lines.push(format!(
                "{}{}",
                indent,
                colorize(&format!("+ {}", format_value(new)), "32", !options.no_color)
            ));
        }
        ChangeKind::TypeChanged {
            old_type,
            new_type,
            old,
            new,
        } => {
            lines.push(format!(
                "{}{}",
                indent,
                colorize(
                    &format!("- ({}) {}", old_type, format_value(old)),
                    "31",
                    !options.no_color
                )
            ));
            lines.push(format!(
                "{}{}",
                indent,
                colorize(
                    &format!("+ ({}) {}", new_type, format_value(new)),
                    "32",
                    !options.no_color
                )
            ));
        }
        ChangeKind::Added { value } => {
            let rendered = value
                .as_ref()
                .map(format_value)
                .unwrap_or_else(|| "<added>".to_string());
            lines.push(format!(
                "{}{}",
                indent,
                colorize(&format!("+ {}", rendered), "32", !options.no_color)
            ));
        }
        ChangeKind::Removed { value } => {
            let rendered = value
                .as_ref()
                .map(format_value)
                .unwrap_or_else(|| "<removed>".to_string());
            lines.push(format!(
                "{}{}",
                indent,
                colorize(&format!("- {}", rendered), "31", !options.no_color)
            ));
        }
    }
}

fn render_context_item(
    depth: usize,
    branches: &[bool],
    is_last: bool,
    parent_path: &[PathSegment],
    idx: usize,
    env: &RenderEnv<'_>,
    lines: &mut Vec<String>,
) {
    lines.push(format_node_line(
        depth,
        branches,
        is_last,
        &format_index_label(idx),
    ));
    let mut path = parent_path.to_vec();
    path.push(PathSegment::Index(idx));
    let value = get_value_at_path(env.t2, &path)
        .or_else(|| get_value_at_path(env.t1, &path))
        .cloned()
        .unwrap_or(Value::Null);
    let indent = branch_indent(depth, branches, !is_last);
    lines.push(format!("{}= {}", indent, format_value(&value)));
}

fn colorize(text: &str, code: &str, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{}m{}\x1b[0m", code, text)
    } else {
        text.to_string()
    }
}

fn object_keys_union(v1: Option<&Value>, v2: Option<&Value>) -> Option<Vec<String>> {
    let mut keys = Vec::new();
    let mut seen = HashSet::new();

    if let Some(Value::Object(map)) = v2 {
        for key in map.keys() {
            if seen.insert(key.clone()) {
                keys.push(key.clone());
            }
        }
    }
    if let Some(Value::Object(map)) = v1 {
        for key in map.keys() {
            if seen.insert(key.clone()) {
                keys.push(key.clone());
            }
        }
    }

    if keys.is_empty() {
        None
    } else {
        Some(keys)
    }
}

fn array_length_union(v1: Option<&Value>, v2: Option<&Value>) -> Option<usize> {
    let len1 = match v1 {
        Some(Value::Array(list)) => list.len(),
        _ => 0,
    };
    let len2 = match v2 {
        Some(Value::Array(list)) => list.len(),
        _ => 0,
    };
    let len = len1.max(len2);
    if len == 0 {
        None
    } else {
        Some(len)
    }
}

fn tree_prefix(branches: &[bool]) -> String {
    let mut out = String::new();
    for has_more in branches {
        if *has_more {
            out.push_str("│   ");
        } else {
            out.push_str("    ");
        }
    }
    out
}

fn branch_indent(depth: usize, branches: &[bool], node_has_more: bool) -> String {
    let mut out = tree_prefix(branches);
    if depth == 0 || node_has_more {
        out.push_str("│   ");
    } else {
        out.push_str("    ");
    }
    out
}
