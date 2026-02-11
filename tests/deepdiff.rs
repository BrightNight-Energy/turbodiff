use serde_json::json;
use turbodiff::{DeepDiff, DeepDiffOptions};

#[test]
fn same_objects_no_diff() {
    let t1 = json!({"a": 1, "b": [1, 2, 3]});
    let diff = DeepDiff::new(t1.clone(), t1);
    assert_eq!(diff.to_value(), json!({}));
}

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
fn string_difference() {
    let t1 = json!({"a": "hello", "b": "world"});
    let t2 = json!({"a": "hello", "b": "world!"});
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "values_changed": {
            "root['b']": {"old_value": "world", "new_value": "world!"}
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn list_difference_add() {
    let t1 = json!([1, 2]);
    let t2 = json!([1, 2, 3, 5]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "iterable_item_added": {
            "root[2]": 3,
            "root[3]": 5
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn list_difference_remove() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([1, 2]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "iterable_item_removed": {
            "root[2]": 3
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn list_difference_with_changes_and_truncation() {
    let t1 = json!([1, 2, 3, 10, 12]);
    let t2 = json!([1, 3, 2]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "values_changed": {
            "root[1]": {"old_value": 2, "new_value": 3},
            "root[2]": {"old_value": 3, "new_value": 2}
        },
        "iterable_item_removed": {
            "root[3]": 10,
            "root[4]": 12
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn list_of_booleans() {
    let t1 = json!([false, false, true, true]);
    let t2 = json!([false, false, false, true]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "values_changed": {
            "root[2]": {"old_value": true, "new_value": false}
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn dict_none_item_removed() {
    let t1 = json!({"a": null, "b": 2});
    let t2 = json!({"b": 2});
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "dictionary_item_removed": ["root['a']"]
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn list_none_item_removed() {
    let t1 = json!([1, 2, null]);
    let t2 = json!([1, 2]);
    let diff = DeepDiff::new(t1, t2);
    let expected = json!({
        "iterable_item_removed": {
            "root[2]": null
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn ignore_numeric_type_changes() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 1.0});
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().ignore_numeric_type_changes(true),
    );
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn ignore_string_type_changes() {
    let t1 = json!({"a": "1"});
    let t2 = json!({"a": "1".to_string()});
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().ignore_string_type_changes(true),
    );
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
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().significant_digits(Some(3)),
    );
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn significant_digits_for_floats() {
    let t1 = json!([1.2344, 5.67881]);
    let t2 = json!([1.2343, 5.67882]);
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().significant_digits(Some(4)),
    );
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
fn include_paths_filters() {
    let t1 = json!({"foo": {"bar": {"fruit": "apple", "veg": "potato"}}, "ingredients": ["bread"]});
    let t2 = json!({"foo": {"bar": {"fruit": "peach", "veg": "potato"}}, "ingredients": ["bread"]});
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().include_paths(vec!["root['foo']".to_string()]),
    );
    let expected = json!({
        "values_changed": {
            "root['foo']['bar']['fruit']": {"old_value": "apple", "new_value": "peach"}
        }
    });
    assert_eq!(diff.to_value(), expected);
}

#[test]
fn include_paths_excludes_unrelated() {
    let t1 = json!({"foo": {"bar": {"fruit": "apple"}}, "ingredients": ["bread"]});
    let t2 = json!({"foo": {"bar": {"fruit": "peach"}}, "ingredients": ["bread"]});
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().include_paths(vec!["root['ingredients']".to_string()]),
    );
    assert_eq!(diff.to_value(), json!({}));
}

#[test]
fn exclude_paths_filters() {
    let t1 = json!({"keep": {"x": 1}, "skip": {"y": 1}});
    let t2 = json!({"keep": {"x": 1}, "skip": {"y": 2}});
    let diff = DeepDiff::with_options(
        t1,
        t2,
        DeepDiffOptions::default().exclude_paths(vec!["root['skip']".to_string()]),
    );
    assert_eq!(diff.to_value(), json!({}));
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
