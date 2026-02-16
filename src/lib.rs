use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

#[cfg(feature = "python")]
use pyo3::exceptions::{PyTypeError, PyValueError};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyAny, PyBytes, PyDict, PyFrozenSet, PyList, PySet, PyTuple, PyType};

#[derive(Clone, Debug)]
pub struct DeepDiffOptions {
    ignore_order: bool,
    ignore_numeric_type_changes: bool,
    ignore_string_type_changes: bool,
    significant_digits: Option<u32>,
    math_epsilon: Option<f64>,
    atol: Option<f64>,
    rtol: Option<f64>,
    include_paths: Vec<String>,
    exclude_paths: Vec<String>,
    verbose_level: u8,
    ignore_type_in_groups: Vec<Vec<ValueType>>,
}

impl Default for DeepDiffOptions {
    fn default() -> Self {
        Self {
            ignore_order: false,
            ignore_numeric_type_changes: false,
            ignore_string_type_changes: false,
            significant_digits: None,
            math_epsilon: None,
            atol: None,
            rtol: None,
            include_paths: Vec::new(),
            exclude_paths: Vec::new(),
            verbose_level: 1,
            ignore_type_in_groups: Vec::new(),
        }
    }
}

impl DeepDiffOptions {
    pub fn ignore_order(mut self, value: bool) -> Self {
        self.ignore_order = value;
        self
    }

    pub fn ignore_numeric_type_changes(mut self, value: bool) -> Self {
        self.ignore_numeric_type_changes = value;
        self
    }

    pub fn ignore_string_type_changes(mut self, value: bool) -> Self {
        self.ignore_string_type_changes = value;
        self
    }

    pub fn significant_digits(mut self, value: Option<u32>) -> Self {
        self.significant_digits = value;
        self
    }

    pub fn math_epsilon(mut self, value: Option<f64>) -> Self {
        self.math_epsilon = value;
        self
    }

    pub fn atol(mut self, value: Option<f64>) -> Self {
        self.atol = value;
        self
    }

    pub fn rtol(mut self, value: Option<f64>) -> Self {
        self.rtol = value;
        self
    }

    pub fn include_paths(mut self, paths: Vec<String>) -> Self {
        self.include_paths = paths;
        self
    }

    pub fn exclude_paths(mut self, paths: Vec<String>) -> Self {
        self.exclude_paths = paths;
        self
    }

    pub fn verbose_level(mut self, value: u8) -> Self {
        self.verbose_level = value;
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum ValueType {
    Number,
    String,
    Bool,
    Null,
    Array,
    Object,
}

#[derive(Clone, Debug)]
pub struct DeepDiff {
    result: Value,
    t1: Value,
    t2: Value,
}

impl DeepDiff {
    pub fn new(t1: Value, t2: Value) -> Self {
        Self::with_options(t1, t2, DeepDiffOptions::default())
    }

    pub fn with_options(t1: Value, t2: Value, options: DeepDiffOptions) -> Self {
        let mut acc = DiffAccumulator::default();
        diff_values(&t1, &t2, "root", &options, &mut acc);
        Self {
            result: acc.into_value(options.verbose_level),
            t1,
            t2,
        }
    }

    pub fn to_value(&self) -> Value {
        self.result.clone()
    }

    pub fn to_dict(&self) -> Value {
        self.result.clone()
    }

    pub fn pretty(&self, options: PrettyOptions) -> String {
        let mut changes = collect_changes(&self.result, &self.t1, &self.t2);
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
            t1: &self.t1,
            t2: &self.t2,
            options: &options,
        };
        render_children(&tree, 0, &[], &[], &env, &mut lines);
        lines.join("\n")
    }

    #[cfg(feature = "python")]
    fn is_empty(&self) -> bool {
        matches!(&self.result, Value::Object(map) if map.is_empty())
    }
}

#[cfg(feature = "python")]
#[pyclass(name = "DeepDiff")]
struct PyDeepDiff {
    inner: DeepDiff,
}

#[cfg(feature = "python")]
#[pymethods]
impl PyDeepDiff {
    #[new]
    #[pyo3(signature = (t1, t2, **kwargs))]
    fn new(
        t1: &Bound<'_, PyAny>,
        t2: &Bound<'_, PyAny>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Self> {
        let t1_val = value_from_py(t1)?;
        let t2_val = value_from_py(t2)?;
        let options = options_from_kwargs(kwargs)?;
        Ok(Self {
            inner: DeepDiff::with_options(t1_val, t2_val, options),
        })
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        value_to_py(py, &self.inner.to_value())
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let value = value_to_py(py, &self.inner.to_value())?;
        Ok(format!("DeepDiff({})", value.bind(py).repr()?))
    }

    fn __bool__(&self) -> PyResult<bool> {
        Ok(!self.inner.is_empty())
    }

    fn __len__(&self) -> PyResult<usize> {
        Ok(match &self.inner.to_value() {
            Value::Object(map) => map.len(),
            _ => 0,
        })
    }

    #[pyo3(signature = (*, compact = false, max_depth = 5, context = 0, no_color = false, path_header = false))]
    fn pretty(
        &self,
        compact: bool,
        max_depth: usize,
        context: usize,
        no_color: bool,
        path_header: bool,
    ) -> PyResult<String> {
        Ok(self.inner.pretty(PrettyOptions {
            compact,
            max_depth,
            context,
            no_color,
            path_header,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct PrettyOptions {
    pub compact: bool,
    pub max_depth: usize,
    pub context: usize,
    pub no_color: bool,
    pub path_header: bool,
}

impl Default for PrettyOptions {
    fn default() -> Self {
        Self {
            compact: false,
            max_depth: 5,
            context: 0,
            no_color: false,
            path_header: false,
        }
    }
}

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
        format_segment_label(a.segment.as_ref().unwrap())
            .cmp(&format_segment_label(b.segment.as_ref().unwrap()))
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
        let segment = node.segment.as_ref().unwrap();
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

#[cfg(feature = "python")]
#[pymodule]
fn turbodiff(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDeepDiff>()?;
    Ok(())
}

#[derive(Default)]
struct DiffAccumulator {
    values_changed: BTreeMap<String, Value>,
    dictionary_item_added: Vec<String>,
    dictionary_item_removed: Vec<String>,
    iterable_item_added: BTreeMap<String, Value>,
    iterable_item_removed: BTreeMap<String, Value>,
    type_changes: BTreeMap<String, Value>,
}

impl DiffAccumulator {
    fn into_value(self, verbose_level: u8) -> Value {
        let mut result = IndexMap::new();

        if !self.values_changed.is_empty() {
            if verbose_level == 0 {
                let mut paths: Vec<String> = self.values_changed.keys().cloned().collect();
                paths.sort();
                result.insert(
                    "values_changed".to_string(),
                    Value::Array(paths.into_iter().map(Value::String).collect()),
                );
            } else {
                result.insert(
                    "values_changed".to_string(),
                    map_to_value(self.values_changed),
                );
            }
        }
        if !self.dictionary_item_added.is_empty() {
            let mut paths = self.dictionary_item_added;
            paths.sort();
            result.insert(
                "dictionary_item_added".to_string(),
                Value::Array(paths.into_iter().map(Value::String).collect()),
            );
        }
        if !self.dictionary_item_removed.is_empty() {
            let mut paths = self.dictionary_item_removed;
            paths.sort();
            result.insert(
                "dictionary_item_removed".to_string(),
                Value::Array(paths.into_iter().map(Value::String).collect()),
            );
        }
        if !self.iterable_item_added.is_empty() {
            result.insert(
                "iterable_item_added".to_string(),
                map_to_value(self.iterable_item_added),
            );
        }
        if !self.iterable_item_removed.is_empty() {
            result.insert(
                "iterable_item_removed".to_string(),
                map_to_value(self.iterable_item_removed),
            );
        }
        if !self.type_changes.is_empty() {
            result.insert("type_changes".to_string(), map_to_value(self.type_changes));
        }

        Value::Object(result.into_iter().collect())
    }
}

fn map_to_value(map: BTreeMap<String, Value>) -> Value {
    Value::Object(map.into_iter().collect())
}

fn diff_values(
    t1: &Value,
    t2: &Value,
    path: &str,
    options: &DeepDiffOptions,
    acc: &mut DiffAccumulator,
) {
    if !path_allowed(path, options) {
        return;
    }

    if values_equal(t1, t2, options) {
        return;
    }

    match (t1, t2) {
        (Value::Object(map1), Value::Object(map2)) => {
            for (key, value1) in map1 {
                if let Some(value2) = map2.get(key) {
                    let child_path = format!("{}['{}']", path, key);
                    diff_values(value1, value2, &child_path, options, acc);
                } else {
                    let child_path = format!("{}['{}']", path, key);
                    acc.dictionary_item_removed.push(child_path);
                }
            }
            for key in map2.keys() {
                if !map1.contains_key(key) {
                    let child_path = format!("{}['{}']", path, key);
                    acc.dictionary_item_added.push(child_path);
                }
            }
        }
        (Value::Array(list1), Value::Array(list2)) => {
            if options.ignore_order {
                diff_arrays_ignore_order(list1, list2, path, options, acc);
            } else {
                let min_len = list1.len().min(list2.len());
                for idx in 0..min_len {
                    let child_path = format!("{}[{}]", path, idx);
                    diff_values(&list1[idx], &list2[idx], &child_path, options, acc);
                }
                if list1.len() > list2.len() {
                    for (idx, item) in list1.iter().enumerate().skip(min_len) {
                        let child_path = format!("{}[{}]", path, idx);
                        acc.iterable_item_removed.insert(child_path, item.clone());
                    }
                }
                if list2.len() > list1.len() {
                    for (idx, item) in list2.iter().enumerate().skip(min_len) {
                        let child_path = format!("{}[{}]", path, idx);
                        acc.iterable_item_added.insert(child_path, item.clone());
                    }
                }
            }
        }
        _ => {
            if types_compatible(t1, t2, options) {
                acc.values_changed
                    .insert(path.to_string(), json_obj(old_new_value(t1, t2)));
            } else {
                acc.type_changes
                    .insert(path.to_string(), json_obj(type_change_value(t1, t2)));
            }
        }
    }
}

fn diff_arrays_ignore_order(
    list1: &[Value],
    list2: &[Value],
    path: &str,
    _options: &DeepDiffOptions,
    acc: &mut DiffAccumulator,
) {
    let mut map1: HashMap<String, Vec<usize>> = HashMap::new();
    let mut map2: HashMap<String, Vec<usize>> = HashMap::new();

    for (idx, item) in list1.iter().enumerate() {
        let key = canonical_string(item);
        map1.entry(key).or_default().push(idx);
    }
    for (idx, item) in list2.iter().enumerate() {
        let key = canonical_string(item);
        map2.entry(key).or_default().push(idx);
    }

    for (key, indices1) in map1.iter() {
        let indices2 = map2.get(key).cloned().unwrap_or_default();
        if indices1.len() > indices2.len() {
            for idx in indices1[indices2.len()..].iter().copied() {
                let child_path = format!("{}[{}]", path, idx);
                acc.iterable_item_removed
                    .insert(child_path, list1[idx].clone());
            }
        }
    }

    for (key, indices2) in map2.iter() {
        let indices1 = map1.get(key).cloned().unwrap_or_default();
        if indices2.len() > indices1.len() {
            for idx in indices2[indices1.len()..].iter().copied() {
                let child_path = format!("{}[{}]", path, idx);
                acc.iterable_item_added
                    .insert(child_path, list2[idx].clone());
            }
        }
    }
}

fn values_equal(t1: &Value, t2: &Value, options: &DeepDiffOptions) -> bool {
    match (t1, t2) {
        (Value::Number(n1), Value::Number(n2)) => numbers_equal(n1, n2, options),
        (Value::String(s1), Value::String(s2)) => s1 == s2,
        (Value::Bool(b1), Value::Bool(b2)) => b1 == b2,
        (Value::Null, Value::Null) => true,
        (Value::Array(a1), Value::Array(a2)) => {
            if options.ignore_order {
                let mut counts1: HashMap<String, usize> = HashMap::new();
                let mut counts2: HashMap<String, usize> = HashMap::new();
                for item in a1 {
                    *counts1.entry(canonical_string(item)).or_insert(0) += 1;
                }
                for item in a2 {
                    *counts2.entry(canonical_string(item)).or_insert(0) += 1;
                }
                counts1 == counts2
            } else {
                a1 == a2
            }
        }
        (Value::Object(o1), Value::Object(o2)) => o1 == o2,
        _ => false,
    }
}

fn numbers_equal(
    n1: &serde_json::Number,
    n2: &serde_json::Number,
    options: &DeepDiffOptions,
) -> bool {
    let f1 = n1.as_f64();
    let f2 = n2.as_f64();

    if let (Some(a), Some(b)) = (f1, f2) {
        if options.ignore_numeric_type_changes && (a - b).abs() <= f64::EPSILON {
            return true;
        }
        let atol = options.atol.or(options.math_epsilon).unwrap_or(0.0);
        let rtol = options.rtol.unwrap_or(0.0);
        if atol > 0.0 || rtol > 0.0 {
            let tol = atol.max(rtol * a.abs().max(b.abs()));
            if (a - b).abs() <= tol {
                return true;
            }
        }
        if let Some(sig) = options.significant_digits {
            if a == 0.0 || b == 0.0 {
                let threshold = 10f64.powi(-(sig as i32));
                return (a - b).abs() <= threshold;
            }
            let ra = round_significant(a, sig);
            let rb = round_significant(b, sig);
            return (ra - rb).abs() <= f64::EPSILON;
        }
    }

    n1 == n2
}

fn round_significant(value: f64, digits: u32) -> f64 {
    if value == 0.0 {
        return 0.0;
    }
    let abs = value.abs();
    let log10 = abs.log10().floor();
    let scale = 10f64.powf(digits as f64 - 1.0 - log10);
    (value * scale).round() / scale
}

fn types_compatible(t1: &Value, t2: &Value, options: &DeepDiffOptions) -> bool {
    if matches!(
        (t1, t2),
        (Value::Number(_), Value::Number(_))
            | (Value::String(_), Value::String(_))
            | (Value::Bool(_), Value::Bool(_))
            | (Value::Null, Value::Null)
    ) {
        return true;
    }
    if options.ignore_type_in_groups.is_empty() {
        return false;
    }
    let vt1 = value_type(t1);
    let vt2 = value_type(t2);
    if vt1 == vt2 {
        return true;
    }
    options
        .ignore_type_in_groups
        .iter()
        .any(|group| group.contains(&vt1) && group.contains(&vt2))
}

fn value_type(value: &Value) -> ValueType {
    match value {
        Value::Number(_) => ValueType::Number,
        Value::String(_) => ValueType::String,
        Value::Bool(_) => ValueType::Bool,
        Value::Null => ValueType::Null,
        Value::Array(_) => ValueType::Array,
        Value::Object(_) => ValueType::Object,
    }
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                "int"
            } else {
                "float"
            }
        }
        Value::String(_) => "str",
        Value::Array(_) => "list",
        Value::Object(_) => "dict",
    }
}

fn type_change_value(t1: &Value, t2: &Value) -> Vec<(&'static str, Value)> {
    vec![
        ("old_type", Value::String(type_name(t1).to_string())),
        ("new_type", Value::String(type_name(t2).to_string())),
        ("old_value", t1.clone()),
        ("new_value", t2.clone()),
    ]
}

fn old_new_value(t1: &Value, t2: &Value) -> Vec<(&'static str, Value)> {
    vec![("old_value", t1.clone()), ("new_value", t2.clone())]
}

fn json_obj(entries: Vec<(&'static str, Value)>) -> Value {
    Value::Object(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn canonical_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("bool:{}", b),
        Value::Number(n) => format!("num:{}", n),
        Value::String(s) => format!("str:{}", s),
        Value::Array(arr) => {
            let inner: Vec<String> = arr.iter().map(canonical_string).collect();
            format!("list:[{}]", inner.join(","))
        }
        Value::Object(obj) => {
            let mut keys: Vec<&String> = obj.keys().collect();
            keys.sort();
            let mut parts = Vec::with_capacity(keys.len());
            for key in keys {
                let val = obj.get(key).unwrap();
                parts.push(format!("{}:{}", key, canonical_string(val)));
            }
            format!("dict:{{{}}}", parts.join(","))
        }
    }
}

fn path_allowed(path: &str, options: &DeepDiffOptions) -> bool {
    for exclude in &options.exclude_paths {
        if path == exclude || path.starts_with(exclude) {
            return false;
        }
    }
    if options.include_paths.is_empty() {
        return true;
    }
    options
        .include_paths
        .iter()
        .any(|include| path == include || include.starts_with(path) || path.starts_with(include))
}

#[cfg(feature = "python")]
fn options_from_kwargs(kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<DeepDiffOptions> {
    let mut options = DeepDiffOptions::default();

    if let Some(kwargs) = kwargs {
        for (key_any, value) in kwargs.iter() {
            let key: String = key_any.extract()?;
            match key {
                key if key == "ignore_order" => {
                    options = options.ignore_order(value.extract::<bool>()?);
                }
                key if key == "ignore_numeric_type_changes" => {
                    options = options.ignore_numeric_type_changes(value.extract::<bool>()?);
                }
                key if key == "ignore_string_type_changes" => {
                    options = options.ignore_string_type_changes(value.extract::<bool>()?);
                }
                key if key == "significant_digits" => {
                    if value.is_none() {
                        options = options.significant_digits(None);
                    } else {
                        options = options.significant_digits(Some(value.extract::<u32>()?));
                    }
                }
                key if key == "math_epsilon" => {
                    if value.is_none() {
                        options = options.math_epsilon(None);
                    } else {
                        options = options.math_epsilon(Some(value.extract::<f64>()?));
                    }
                }
                key if key == "math_absilon" => {
                    if value.is_none() {
                        options = options.math_epsilon(None);
                    } else {
                        options = options.math_epsilon(Some(value.extract::<f64>()?));
                    }
                }
                key if key == "atol" => {
                    if value.is_none() {
                        options = options.atol(None);
                    } else {
                        options = options.atol(Some(value.extract::<f64>()?));
                    }
                }
                key if key == "rtol" => {
                    if value.is_none() {
                        options = options.rtol(None);
                    } else {
                        options = options.rtol(Some(value.extract::<f64>()?));
                    }
                }
                key if key == "include_paths" => {
                    let paths = extract_string_list(&value)?;
                    options = options.include_paths(paths);
                }
                key if key == "exclude_paths" => {
                    let paths = extract_string_list(&value)?;
                    options = options.exclude_paths(paths);
                }
                key if key == "verbose_level" => {
                    options = options.verbose_level(value.extract::<u8>()?);
                }
                key if key == "ignore_type_in_groups" => {
                    let (groups, ignore_numeric, ignore_string) = extract_type_groups(&value)?;
                    options.ignore_type_in_groups = groups;
                    if ignore_numeric {
                        options = options.ignore_numeric_type_changes(true);
                    }
                    if ignore_string {
                        options = options.ignore_string_type_changes(true);
                    }
                }
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "Unsupported option: {}",
                        key
                    )));
                }
            }
        }
    }

    Ok(options)
}

#[cfg(feature = "python")]
fn extract_string_list(value: &Bound<'_, PyAny>) -> PyResult<Vec<String>> {
    if let Ok(list) = value.downcast::<PyList>() {
        list.iter().map(|item| item.extract::<String>()).collect()
    } else if let Ok(tuple) = value.downcast::<PyTuple>() {
        tuple.iter().map(|item| item.extract::<String>()).collect()
    } else if let Ok(set) = value.downcast::<PySet>() {
        set.iter().map(|item| item.extract::<String>()).collect()
    } else if let Ok(set) = value.downcast::<PyFrozenSet>() {
        set.iter().map(|item| item.extract::<String>()).collect()
    } else {
        Err(PyTypeError::new_err(
            "Expected a list, tuple, or set of strings",
        ))
    }
}

#[cfg(feature = "python")]
fn extract_type_groups(value: &Bound<'_, PyAny>) -> PyResult<(Vec<Vec<ValueType>>, bool, bool)> {
    let groups_any = if let Ok(list) = value.downcast::<PyList>() {
        list.iter().collect::<Vec<_>>()
    } else if let Ok(tuple) = value.downcast::<PyTuple>() {
        tuple.iter().collect::<Vec<_>>()
    } else {
        return Err(PyTypeError::new_err(
            "Expected a list or tuple of type groups",
        ));
    };

    let py = value.py();
    let type_int = py.get_type_bound::<pyo3::types::PyLong>();
    let type_float = py.get_type_bound::<pyo3::types::PyFloat>();
    let type_bool = py.get_type_bound::<pyo3::types::PyBool>();
    let type_str = py.get_type_bound::<pyo3::types::PyString>();
    let type_bytes = py.get_type_bound::<PyBytes>();
    let type_none = py.get_type_bound::<pyo3::types::PyNone>();
    let type_list = py.get_type_bound::<PyList>();
    let type_tuple = py.get_type_bound::<PyTuple>();
    let type_dict = py.get_type_bound::<PyDict>();
    let numbers_mod = py.import_bound("numbers")?;
    let number_obj = numbers_mod.getattr("Number")?;
    let number_type = number_obj.downcast::<PyType>()?;
    let numpy_mod = py.import_bound("numpy").ok();

    let mut groups: Vec<Vec<ValueType>> = Vec::new();
    let mut ignore_numeric = false;
    let mut ignore_string = false;

    for group_any in groups_any {
        let items = if let Ok(list) = group_any.downcast::<PyList>() {
            list.iter().collect::<Vec<_>>()
        } else if let Ok(tuple) = group_any.downcast::<PyTuple>() {
            tuple.iter().collect::<Vec<_>>()
        } else {
            return Err(PyTypeError::new_err(
                "Each ignore_type_in_groups entry must be a list or tuple of types",
            ));
        };

        let mut group: Vec<ValueType> = Vec::new();
        let mut has_int = false;
        let mut has_float = false;
        let mut has_str = false;
        let mut has_bytes = false;

        for item in items {
            let ty = match item.downcast::<PyType>() {
                Ok(ty) => ty,
                Err(_) => {
                    let module: Option<String> = item
                        .getattr("__module__")
                        .ok()
                        .and_then(|m| m.extract().ok());
                    let name: Option<String> =
                        item.getattr("__name__").ok().and_then(|n| n.extract().ok());
                    if module.as_deref().unwrap_or("").starts_with("numpy")
                        && name.as_deref().unwrap_or("") == "array"
                    {
                        group.push(ValueType::Array);
                        continue;
                    }
                    return Err(PyTypeError::new_err(
                        "Unsupported type in ignore_type_in_groups",
                    ));
                }
            };
            let vt = if ty.is(&type_int) {
                has_int = true;
                ValueType::Number
            } else if ty.is(&type_float) {
                has_float = true;
                ValueType::Number
            } else if ty.is(&type_bool) {
                ValueType::Bool
            } else if ty.is(&type_str) {
                has_str = true;
                ValueType::String
            } else if ty.is(&type_bytes) {
                has_bytes = true;
                ValueType::String
            } else if ty.is(&type_none) {
                ValueType::Null
            } else if ty.is(&type_list) || ty.is(&type_tuple) {
                ValueType::Array
            } else if ty.is(&type_dict) {
                ValueType::Object
            } else if {
                let module: String = ty.getattr("__module__")?.extract()?;
                module.starts_with("numpy")
            } {
                let is_ndarray = if let Some(np) = numpy_mod.as_ref() {
                    if let Ok(ndarray) = np.getattr("ndarray") {
                        if let Ok(ndarray) = ndarray.downcast::<PyType>() {
                            ty.is_subclass(ndarray).unwrap_or(false)
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                let name = ty.name()?.to_lowercase();
                if is_ndarray || name.contains("ndarray") {
                    ValueType::Array
                } else if name.contains("bool") {
                    ValueType::Bool
                } else {
                    if name.contains("float") || name.contains("floating") {
                        has_float = true;
                    } else if name.contains("int")
                        || name.contains("integer")
                        || name.contains("uint")
                        || name.contains("number")
                    {
                        has_int = true;
                    } else {
                        has_float = true;
                    }
                    ValueType::Number
                }
            } else if ty.is_subclass(number_type)? {
                let module: String = ty.getattr("__module__")?.extract()?;
                let name = ty.name()?.to_lowercase();
                if module == "numpy" {
                    if name.contains("float") || name.contains("floating") {
                        has_float = true;
                    } else if name.contains("int")
                        || name.contains("integer")
                        || name.contains("uint")
                    {
                        has_int = true;
                    } else {
                        has_float = true;
                    }
                } else {
                    has_float = true;
                }
                ValueType::Number
            } else {
                return Err(PyTypeError::new_err(
                    "Unsupported type in ignore_type_in_groups",
                ));
            };
            group.push(vt);
        }

        if has_int && has_float {
            ignore_numeric = true;
        }
        if has_str && has_bytes {
            ignore_string = true;
        }

        groups.push(group);
    }

    Ok((groups, ignore_numeric, ignore_string))
}

#[cfg(feature = "python")]
fn value_from_py(value: &Bound<'_, PyAny>) -> PyResult<Value> {
    if value.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = value.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(u) = value.extract::<u64>() {
        return Ok(Value::Number(u.into()));
    }
    if let Ok(f) = value.extract::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(f) {
            return Ok(Value::Number(num));
        }
        return Err(PyValueError::new_err("Float value is not finite"));
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(Value::String(s));
    }
    if let Ok(list) = value.downcast::<PyList>() {
        let mut items = Vec::with_capacity(list.len());
        for item in list.iter() {
            items.push(value_from_py(&item)?);
        }
        return Ok(Value::Array(items));
    }
    if let Ok(tuple) = value.downcast::<PyTuple>() {
        let mut items = Vec::with_capacity(tuple.len());
        for item in tuple.iter() {
            items.push(value_from_py(&item)?);
        }
        return Ok(Value::Array(items));
    }
    if let Ok(set) = value.downcast::<PySet>() {
        let mut items = Vec::with_capacity(set.len());
        for item in set.iter() {
            items.push(value_from_py(&item)?);
        }
        // Sets are unordered; canonical sorting yields stable diffs.
        items.sort_by_key(canonical_string);
        return Ok(Value::Array(items));
    }
    if let Ok(set) = value.downcast::<PyFrozenSet>() {
        let mut items = Vec::with_capacity(set.len());
        for item in set.iter() {
            items.push(value_from_py(&item)?);
        }
        // Frozen sets are unordered; canonical sorting yields stable diffs.
        items.sort_by_key(canonical_string);
        return Ok(Value::Array(items));
    }
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut map = serde_json::Map::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let key: String = match k.extract::<String>() {
                Ok(val) => val,
                Err(_) => k
                    .str()
                    .and_then(|s| s.extract::<String>())
                    .map_err(|_| PyTypeError::new_err("Unsupported dict key type for DeepDiff"))?,
            };
            map.insert(key, value_from_py(&v)?);
        }
        return Ok(Value::Object(map));
    }
    if value
        .get_type()
        .getattr("__module__")?
        .extract::<String>()?
        .starts_with("pandas")
    {
        if let Ok(to_dict) = value.getattr("to_dict") {
            let py = value.py();
            let kwargs = PyDict::new_bound(py);
            kwargs.set_item("orient", "list")?;
            if let Ok(res) = to_dict.call((), Some(&kwargs)) {
                return value_from_py(&res);
            }
            let res = to_dict.call0()?;
            return value_from_py(&res);
        }
        if let Ok(to_numpy) = value.getattr("to_numpy") {
            let res = to_numpy.call0()?;
            return value_from_py(&res);
        }
    }
    if value.hasattr("model_dump")? {
        let py = value.py();
        let kwargs = PyDict::new_bound(py);
        kwargs.set_item("mode", "json")?;
        if let Ok(dumped) = value.call_method("model_dump", (), Some(&kwargs)) {
            return value_from_py(&dumped);
        }
        let dumped = value.call_method0("model_dump")?;
        return value_from_py(&dumped);
    }
    if value.hasattr("dict")? {
        let dumped = value.call_method0("dict")?;
        return value_from_py(&dumped);
    }
    if value
        .get_type()
        .getattr("__module__")?
        .extract::<String>()?
        .starts_with("numpy")
    {
        if let Ok(tolist) = value.call_method0("tolist") {
            return value_from_py(&tolist);
        }
    }

    Err(PyTypeError::new_err("Unsupported Python type for DeepDiff"))
}

#[cfg(feature = "python")]
fn value_to_py(py: Python<'_>, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Err(PyValueError::new_err("Invalid number"))
            }
        }
        Value::String(s) => Ok(s.into_py(py)),
        Value::Array(arr) => {
            let list = PyList::empty_bound(py);
            for item in arr {
                list.append(value_to_py(py, item)?)?;
            }
            Ok(list.into_py(py))
        }
        Value::Object(obj) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in obj {
                dict.set_item(k, value_to_py(py, v)?)?;
            }
            Ok(dict.into_py(py))
        }
    }
}
