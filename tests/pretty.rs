use serde_json::{json, Map, Value};
use turbodiff::{DeepDiff, PrettyOptions};

#[test]
fn pretty_empty_diff_returns_empty_string() {
    let diff = DeepDiff::new(json!({"a": 1}), json!({"a": 1}));
    assert_eq!(diff.pretty(PrettyOptions::default()), "");
}

#[test]
fn pretty_simple_change() {
    let diff = DeepDiff::new(json!({"a": {"b": 1}}), json!({"a": {"b": 2}}));
    let output = diff.pretty(PrettyOptions {
        no_color: true,
        ..PrettyOptions::default()
    });
    assert_eq!(output, "a\n╰── b\n    - 1\n    + 2");
}

#[test]
fn pretty_list_change() {
    let diff = DeepDiff::new(json!(["a", "b"]), json!(["c", "d"]));
    let output = diff.pretty(PrettyOptions {
        no_color: true,
        ..PrettyOptions::default()
    });
    assert_eq!(
        output,
        "[0]\n│   - 'a'\n│   + 'c'\n[1]\n│   - 'b'\n│   + 'd'"
    );
}

#[test]
fn pretty_path_header() {
    let diff = DeepDiff::new(json!({"a": {"b": 1}}), json!({"a": {"b": 2}}));
    let output = diff.pretty(PrettyOptions {
        no_color: true,
        path_header: true,
        ..PrettyOptions::default()
    });
    assert_eq!(output, "a.b\n│   - 1\n│   + 2");
}

#[test]
fn pretty_continuation_with_ellipsis() {
    let mut inner = Map::new();
    for key in "abcdefghijkl".chars() {
        inner.insert(key.to_string(), json!(1));
    }

    let mut changed_inner = inner.clone();
    changed_inner.insert("b".to_string(), json!(2));
    changed_inner.insert("j".to_string(), json!(2));

    let t1 = Value::Object(
        [("a".to_string(), Value::Object(inner))]
            .into_iter()
            .collect(),
    );
    let t2 = Value::Object(
        [("a".to_string(), Value::Object(changed_inner))]
            .into_iter()
            .collect(),
    );

    let output = DeepDiff::new(t1, t2).pretty(PrettyOptions {
        no_color: true,
        ..PrettyOptions::default()
    });
    assert_eq!(
        output,
        "a\n├── b\n│   - 1\n│   + 2\n├── ...\n╰── j\n    - 1\n    + 2"
    );
}
