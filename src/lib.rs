mod engine;
mod options;
mod pretty;

#[cfg(feature = "python")]
mod python;

pub use options::{DeepDiffOptions, PrettyOptions, ValueType};

use serde_json::Value;

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
        let mut acc = engine::DiffAccumulator::default();
        engine::diff_values(&t1, &t2, "root", &options, &mut acc);
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
        pretty::render_pretty(&self.result, &self.t1, &self.t2, options)
    }

    #[cfg(feature = "python")]
    pub(crate) fn is_empty(&self) -> bool {
        matches!(&self.result, Value::Object(map) if map.is_empty())
    }
}

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn turbodiff(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register_module(m)
}
