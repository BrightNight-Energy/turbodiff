use turbodiff::{DeepDiff, DeepDiffOptions};
use serde_json::json;

#[test]
fn values_changed_basic() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 2});
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "values_changed": {
            "root['a']": {"old_value": 1, "new_value": 2}
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn dictionary_item_added_removed() {
    let t1 = json!({"a": 1, "b": 2});
    let t2 = json!({"a": 1, "c": 3});
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "dictionary_item_added": ["root['c']"],
        "dictionary_item_removed": ["root['b']"]
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn iterable_item_added_removed() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([1, 4, 3, 5]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "values_changed": {
            "root[1]": {"old_value": 2, "new_value": 4}
        },
        "iterable_item_added": {
            "root[3]": 5
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn type_changes_basic() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": "1"});
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "type_changes": {
            "root['a']": {
                "old_type": "int",
                "new_type": "str",
                "old_value": 1,
                "new_value": "1"
            }
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn ignore_numeric_type_changes() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 1.0});
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().ignore_numeric_type_changes(true));
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn ignore_string_type_changes() {
    let t1 = json!({"a": "1"});
    let t2 = json!({"a": "1".to_string()});
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().ignore_string_type_changes(true));
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn ignore_order_for_lists() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([3, 2, 1]);
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().ignore_order(true));
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn significant_digits_suppresses_small_changes() {
    let t1 = json!(1.1234);
    let t2 = json!(1.1235);
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().significant_digits(Some(3)));
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn math_epsilon_suppresses_small_changes() {
    let t1 = json!(1.0);
    let t2 = json!(1.0005);
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().math_epsilon(Some(0.001)));
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn include_and_exclude_paths() {
    let t1 = json!({"keep": {"x": 1}, "skip": {"y": 1}});
    let t2 = json!({"keep": {"x": 2}, "skip": {"y": 2}});

    let diff_include = DeepDiff::with_options(
        t1.clone(),
        t2.clone(),
        DeepDiffOptions::default().include_paths(vec!["root['keep']['x']".to_string()]),
    );
    let expected_include = json!({
        "values_changed": {
            "root['keep']['x']": {"old_value": 1, "new_value": 2}
        }
    });
    assert_eq!(diff_include.to_value(), expected_include);

    let diff_exclude = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().exclude_paths(vec!["root['skip']".to_string()]),
    );
    let expected_exclude = json!({
        "values_changed": {
            "root['keep']['x']": {"old_value": 1, "new_value": 2}
        }
    });
    assert_eq!(diff_exclude.to_value(), expected_exclude);
}

#[test]
fn verbose_level_zero_paths_only() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 2});
    let diff = DeepDiff::with_options(t1, t2, DeepDiffOptions::default().verbose_level(0));
    let expected = json!({
        "values_changed": ["root['a']"]
    });
    assert_eq!(diff.to_value(), expected);
}
