use crate::options::{DeepDiffOptions, ValueType};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub(crate) struct DiffAccumulator {
    values_changed: BTreeMap<String, Value>,
    dictionary_item_added: Vec<String>,
    dictionary_item_removed: Vec<String>,
    iterable_item_added: BTreeMap<String, Value>,
    iterable_item_removed: BTreeMap<String, Value>,
    type_changes: BTreeMap<String, Value>,
}

impl DiffAccumulator {
    pub(crate) fn into_value(self, verbose_level: u8) -> Value {
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

pub(crate) fn diff_values(
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

    for (key, indices1) in &map1 {
        let indices2 = map2.get(key).cloned().unwrap_or_default();
        if indices1.len() > indices2.len() {
            for idx in indices1[indices2.len()..].iter().copied() {
                let child_path = format!("{}[{}]", path, idx);
                acc.iterable_item_removed
                    .insert(child_path, list1[idx].clone());
            }
        }
    }

    for (key, indices2) in &map2 {
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

pub(crate) fn canonical_string(value: &Value) -> String {
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
                let val = obj
                    .get(key)
                    .expect("key gathered from object keys must exist");
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
