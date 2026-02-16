use crate::engine::canonical_string;
use crate::options::{DeepDiffOptions, PrettyOptions, ValueType};
use crate::DeepDiff;
use pyo3::exceptions::{PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBytes, PyDict, PyFrozenSet, PyList, PySet, PyTuple, PyType};
use serde_json::Value;

#[pyclass(name = "DeepDiff")]
struct PyDeepDiff {
    inner: DeepDiff,
}

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

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }

    fn __len__(&self) -> usize {
        match &self.inner.to_value() {
            Value::Object(map) => map.len(),
            _ => 0,
        }
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

pub(crate) fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyDeepDiff>()?;
    Ok(())
}

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
        items.sort_by_key(canonical_string);
        return Ok(Value::Array(items));
    }
    if let Ok(set) = value.downcast::<PyFrozenSet>() {
        let mut items = Vec::with_capacity(set.len());
        for item in set.iter() {
            items.push(value_from_py(&item)?);
        }
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
