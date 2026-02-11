use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

#[cfg(feature = "python")]
use pyo3::exceptions::{PyTypeError, PyValueError};
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyAny, PyDict, PyList, PyTuple};

#[derive(Clone, Debug)]
pub struct DeepDiffOptions {
    ignore_order: bool,
    ignore_numeric_type_changes: bool,
    ignore_string_type_changes: bool,
    significant_digits: Option<u32>,
    math_epsilon: Option<f64>,
    include_paths: Vec<String>,
    exclude_paths: Vec<String>,
    verbose_level: u8,
}

impl Default for DeepDiffOptions {
    fn default() -> Self {
        Self {
            ignore_order: false,
            ignore_numeric_type_changes: false,
            ignore_string_type_changes: false,
            significant_digits: None,
            math_epsilon: None,
            include_paths: Vec::new(),
            exclude_paths: Vec::new(),
            verbose_level: 1,
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

#[derive(Clone, Debug)]
pub struct DeepDiff {
    result: Value,
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
        }
    }

    pub fn to_value(&self) -> Value {
        self.result.clone()
    }

    pub fn to_dict(&self) -> Value {
        self.result.clone()
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
    fn new(t1: &Bound<'_, PyAny>, t2: &Bound<'_, PyAny>, kwargs: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
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
                result.insert("values_changed".to_string(), Value::Array(paths.into_iter().map(Value::String).collect()));
            } else {
                result.insert("values_changed".to_string(), map_to_value(self.values_changed));
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
            result.insert("iterable_item_added".to_string(), map_to_value(self.iterable_item_added));
        }
        if !self.iterable_item_removed.is_empty() {
            result.insert("iterable_item_removed".to_string(), map_to_value(self.iterable_item_removed));
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

fn diff_values(t1: &Value, t2: &Value, path: &str, options: &DeepDiffOptions, acc: &mut DiffAccumulator) {
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
                    for idx in min_len..list1.len() {
                        let child_path = format!("{}[{}]", path, idx);
                        acc.iterable_item_removed.insert(child_path, list1[idx].clone());
                    }
                }
                if list2.len() > list1.len() {
                    for idx in min_len..list2.len() {
                        let child_path = format!("{}[{}]", path, idx);
                        acc.iterable_item_added.insert(child_path, list2[idx].clone());
                    }
                }
            }
        }
        _ => {
            if types_compatible(t1, t2) {
                acc.values_changed.insert(path.to_string(), json_obj(old_new_value(t1, t2)));
            } else {
                acc.type_changes
                    .insert(path.to_string(), json_obj(type_change_value(t1, t2)));
            }
        }
    }
}

fn diff_arrays_ignore_order(list1: &[Value], list2: &[Value], path: &str, _options: &DeepDiffOptions, acc: &mut DiffAccumulator) {
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
                acc.iterable_item_removed.insert(child_path, list1[idx].clone());
            }
        }
    }

    for (key, indices2) in map2.iter() {
        let indices1 = map1.get(key).cloned().unwrap_or_default();
        if indices2.len() > indices1.len() {
            for idx in indices2[indices1.len()..].iter().copied() {
                let child_path = format!("{}[{}]", path, idx);
                acc.iterable_item_added.insert(child_path, list2[idx].clone());
            }
        }
    }

    if !acc.iterable_item_added.is_empty() || !acc.iterable_item_removed.is_empty() {
        return;
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

fn numbers_equal(n1: &serde_json::Number, n2: &serde_json::Number, options: &DeepDiffOptions) -> bool {
    let f1 = n1.as_f64();
    let f2 = n2.as_f64();

    if let (Some(a), Some(b)) = (f1, f2) {
        if options.ignore_numeric_type_changes && (a - b).abs() <= f64::EPSILON {
            return true;
        }
        if let Some(eps) = options.math_epsilon {
            if (a - b).abs() <= eps {
                return true;
            }
        }
        if let Some(sig) = options.significant_digits {
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

fn types_compatible(t1: &Value, t2: &Value) -> bool {
    match (t1, t2) {
        (Value::Number(_), Value::Number(_)) => true,
        (Value::String(_), Value::String(_)) => true,
        (Value::Bool(_), Value::Bool(_)) => true,
        (Value::Null, Value::Null) => true,
        _ => false,
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
    Value::Object(entries.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
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
    } else {
        Err(PyTypeError::new_err("Expected a list or tuple of strings"))
    }
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
    if let Ok(dict) = value.downcast::<PyDict>() {
        let mut map = serde_json::Map::with_capacity(dict.len());
        for (k, v) in dict.iter() {
            let key: String = k.extract()?;
            map.insert(key, value_from_py(&v)?);
        }
        return Ok(Value::Object(map));
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
