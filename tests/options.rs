mod common;

use serde_json::json;
use turbodiff::{DeepDiffOptions, ValueType};

#[test]
fn ignore_numeric_type_changes() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 1.0});
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().ignore_numeric_type_changes(true),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn ignore_string_type_changes() {
    let t1 = json!({"a": "1"});
    let t2 = json!({"a": "1".to_string()});
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().ignore_string_type_changes(true),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn ignore_order_for_lists() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([3, 2, 1]);
    let diff = common::diff_with_options(t1, t2, DeepDiffOptions::default().ignore_order(true));
    assert_eq!(diff, json!({}));
}

#[test]
fn ignore_order_still_detects_multiplicity_changes() {
    let t1 = json!([1, 1, 2]);
    let t2 = json!([1, 2, 2]);
    let diff = common::diff_with_options(t1, t2, DeepDiffOptions::default().ignore_order(true));
    let expected = json!({
        "iterable_item_added": {
            "root[2]": 2
        },
        "iterable_item_removed": {
            "root[1]": 1
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn significant_digits_suppresses_small_changes() {
    let t1 = json!(1.1234);
    let t2 = json!(1.1235);
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().significant_digits(Some(3)),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn significant_digits_handles_near_zero_values() {
    let t1 = json!(0);
    let t2 = json!(7e-7);
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().significant_digits(Some(1)),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn significant_digits_for_floats() {
    let t1 = json!([1.2344, 5.67881]);
    let t2 = json!([1.2343, 5.67882]);
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().significant_digits(Some(4)),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn math_epsilon_suppresses_small_changes() {
    let t1 = json!(1.0);
    let t2 = json!(1.0005);
    let diff =
        common::diff_with_options(t1, t2, DeepDiffOptions::default().math_epsilon(Some(0.001)));
    assert_eq!(diff, json!({}));
}

#[test]
fn atol_suppresses_small_changes() {
    let t1 = json!(1.0);
    let t2 = json!(1.0005);
    let diff = common::diff_with_options(t1, t2, DeepDiffOptions::default().atol(Some(0.001)));
    assert_eq!(diff, json!({}));
}

#[test]
fn rtol_suppresses_relative_changes() {
    let t1 = json!(1000.0);
    let t2 = json!(1000.1);
    let diff = common::diff_with_options(t1, t2, DeepDiffOptions::default().rtol(Some(1e-3)));
    assert_eq!(diff, json!({}));
}

#[test]
fn include_paths_filters() {
    let t1 = json!({"foo": {"bar": {"fruit": "apple", "veg": "potato"}}, "ingredients": ["bread"]});
    let t2 = json!({"foo": {"bar": {"fruit": "peach", "veg": "potato"}}, "ingredients": ["bread"]});
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().include_paths(vec!["root['foo']".to_string()]),
    );
    let expected = json!({
        "values_changed": {
            "root['foo']['bar']['fruit']": {"old_value": "apple", "new_value": "peach"}
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn include_paths_excludes_unrelated() {
    let t1 = json!({"foo": {"bar": {"fruit": "apple"}}, "ingredients": ["bread"]});
    let t2 = json!({"foo": {"bar": {"fruit": "peach"}}, "ingredients": ["bread"]});
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().include_paths(vec!["root['ingredients']".to_string()]),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn exclude_paths_filters() {
    let t1 = json!({"keep": {"x": 1}, "skip": {"y": 1}});
    let t2 = json!({"keep": {"x": 1}, "skip": {"y": 2}});
    let diff = common::diff_with_options(
        t1,
        t2,
        DeepDiffOptions::default().exclude_paths(vec!["root['skip']".to_string()]),
    );
    assert_eq!(diff, json!({}));
}

#[test]
fn verbose_level_zero_paths_only() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 2});
    let diff = common::diff_with_options(t1, t2, DeepDiffOptions::default().verbose_level(0));
    let expected = json!({
        "values_changed": ["root['a']"]
    });
    assert_eq!(diff, expected);
}

#[test]
fn ignore_type_in_groups_treats_bool_and_string_as_value_change() {
    let diff = common::diff_with_options(
        json!(true),
        json!("Yes"),
        DeepDiffOptions::default()
            .ignore_type_in_groups(vec![vec![ValueType::Bool, ValueType::String]]),
    );
    let expected = json!({
        "values_changed": {
            "root": {"old_value": true, "new_value": "Yes"}
        }
    });
    assert_eq!(diff, expected);
}
