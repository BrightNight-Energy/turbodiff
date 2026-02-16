<img align="center" alt="turbodiff logo" src=".github/assets/logo.png" />

# Turbodiff

![PyPI - Version](https://img.shields.io/pypi/v/turbodiff)
![GitHub CI](https://github.com/BrightNight-Energy/turbodiff/actions/workflows/ci.yml/badge.svg)

##### Zero dependencies ✨ Rust-based 🦀 Super fast 🚀

Turbodiff is a super fast diffing library built from the ground up in Rust for
speed and consistency. It focuses on core diffing behavior and exposes both a
Rust API and Python bindings.

<img align="center" alt="turbodiff speed" src=".github/assets/speed-comparison.png" />

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

## Pretty Output (Python)

```python
from turbodiff import DeepDiff

diff = DeepDiff({"a": {"b": 1}}, {"a": {"b": 2}})
print(diff.pretty())

# Optional controls
print(diff.pretty(no_color=True, compact=True, max_depth=5, context=0, path_header=False))
```

## Supported keyword options (Python)

All options are passed as keyword arguments to `DeepDiff(...)`.

| Option | Type | Behavior |
| --- | --- | --- |
| `ignore_order` | `bool` | Treat arrays as multisets (order-insensitive). |
| `ignore_numeric_type_changes` | `bool` | Treat `int`/`float` type changes as value changes. |
| `ignore_string_type_changes` | `bool` | Treat `str`/`bytes` type changes as value changes. |
| `ignore_type_in_groups` | `list[tuple[type, ...]]` | Treat types in each group as compatible (type changes become value changes). Example: `[(int, float), (bool, str)]`. |
| `significant_digits` | `int \| None` | Compare numbers rounded to N significant digits. |
| `math_epsilon` | `float \| None` | Absolute tolerance for numeric comparison (alias for `atol`). |
| `atol` | `float \| None` | Absolute tolerance for numeric comparison. |
| `rtol` | `float \| None` | Relative tolerance for numeric comparison. Uses `abs(a-b) <= max(atol, rtol * max(abs(a), abs(b)))`. |
| `include_paths` | `list[str]` | Only diff paths that match these prefixes. |
| `exclude_paths` | `list[str]` | Skip any paths that match these prefixes. |
| `verbose_level` | `int` (0 or 1) | `0` returns paths only for `values_changed`. |

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
