<img align="center" alt="turbodiff logo" src=".github/assets/turbodiff-logo.png" />

# Turbodiff

![PyPI - Version](https://img.shields.io/pypi/v/turbodiff)
![GitHub CI](https://github.com/BrightNight-Energy/turbodiff/actions/workflows/ci.yml/badge.svg)

##### Zero dependencies ✨ Rust-based 🦀 Super fast 🚀

Turbodiff is a super fast diffing library built from the ground up in Rust for
speed and consistency. It focuses on core diffing behavior and exposes both a
Rust API and Python bindings.

<img align="center" alt="turbodiff speed" src=".github/assets/turbodiff-speed-comparison.png" />

## Credits

This project is inspired by and compatible with the design of 
[DeepDiff](https://github.com/seperman/deepdiff).

It was vibe-coded with gpt-5.2-codex. Contributions are welcome.

## Features

- DeepDiff-style output keys: `values_changed`, `dictionary_item_added`,
  `dictionary_item_removed`, `iterable_item_added`, `iterable_item_removed`,
  `type_changes`
- Options for order ignoring, numeric type tolerance, significant digits,
  epsilon comparisons, include/exclude paths, and verbose level
- Rust core + Python bindings via `pyo3`/`maturin`

## Status

This is an early, focused implementation. It targets the most common DeepDiff
use cases and is intended as a drop-in replacement for the supported subset.

## Installation (Python)

Install from PyPI with pip:

```bash
pip install turbodiff
```

For local development builds, the Python package is built with `maturin`:

```bash
maturin develop --features python
```

## Usage (Python)

```python
from turbodiff import DeepDiff

diff = DeepDiff({"a": 1}, {"a": 2})
print(diff.to_dict())
# {'values_changed': {"root['a']": {'old_value': 1, 'new_value': 2}}}

# Truthiness follows DeepDiff semantics
assert not diff # will raise AssertionError
```

## Supported keyword options (Python)

All options are passed as keyword arguments to `DeepDiff(...)`.

- `ignore_order: bool`
- `ignore_numeric_type_changes: bool`
- `ignore_string_type_changes: bool`
- `significant_digits: int | None`
- `math_epsilon: float | None`
- `include_paths: list[str]`
- `exclude_paths: list[str]`
- `verbose_level: int` (0 or 1)

`verbose_level=0` returns paths only for `values_changed`.

## Usage (Rust)

```rust
use serde_json::json;
use turbodiff::{DeepDiff, DeepDiffOptions};

let t1 = json!({"a": 1});
let t2 = json!({"a": 2});
let diff = DeepDiff::new(t1, t2);
println!("{}", diff.to_value());

let options = DeepDiffOptions::default().ignore_order(true);
let diff = DeepDiff::with_options(json!([1, 2]), json!([2, 1]), options);
assert_eq!(diff.to_value(), json!({}));
```

## Development

- Rust tests: `cargo test`
- Python tests: `pytest`

## Contributing

Issues and PRs are welcome. Please include a minimal repro and tests where
possible.
