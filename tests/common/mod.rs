#![allow(dead_code)]

use serde_json::Value;
use turbodiff::{DeepDiff, DeepDiffOptions};

pub fn diff(t1: Value, t2: Value) -> Value {
    DeepDiff::new(t1, t2).to_value()
}

pub fn diff_with_options(t1: Value, t2: Value, options: DeepDiffOptions) -> Value {
    DeepDiff::with_options(t1, t2, options).to_value()
}
