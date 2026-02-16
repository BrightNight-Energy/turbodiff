mod common;

use serde_json::json;

#[test]
fn same_objects_no_diff() {
    let t1 = json!({"a": 1, "b": [1, 2, 3]});
    let diff = common::diff(t1.clone(), t1);
    assert_eq!(diff, json!({}));
}

#[test]
fn values_changed_basic() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 2});
    let diff = common::diff(t1, t2);
    let expected = json!({
        "values_changed": {
            "root['a']": {"old_value": 1, "new_value": 2}
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn to_dict_matches_to_value() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": 2});
    let deepdiff = turbodiff::DeepDiff::new(t1, t2);
    assert_eq!(deepdiff.to_dict(), deepdiff.to_value());
}

#[test]
fn dictionary_item_added_removed() {
    let t1 = json!({"a": 1, "b": 2});
    let t2 = json!({"a": 1, "c": 3});
    let diff = common::diff(t1, t2);
    let expected = json!({
        "dictionary_item_added": ["root['c']"],
        "dictionary_item_removed": ["root['b']"]
    });
    assert_eq!(diff, expected);
}

#[test]
fn iterable_item_added_removed() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([1, 4, 3, 5]);
    let diff = common::diff(t1, t2);
    let expected = json!({
        "values_changed": {
            "root[1]": {"old_value": 2, "new_value": 4}
        },
        "iterable_item_added": {
            "root[3]": 5
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn type_changes_basic() {
    let t1 = json!({"a": 1});
    let t2 = json!({"a": "1"});
    let diff = common::diff(t1, t2);
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
    assert_eq!(diff, expected);
}

#[test]
fn string_difference() {
    let t1 = json!({"a": "hello", "b": "world"});
    let t2 = json!({"a": "hello", "b": "world!"});
    let diff = common::diff(t1, t2);
    let expected = json!({
        "values_changed": {
            "root['b']": {"old_value": "world", "new_value": "world!"}
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn list_difference_add() {
    let t1 = json!([1, 2]);
    let t2 = json!([1, 2, 3, 5]);
    let diff = common::diff(t1, t2);
    let expected = json!({
        "iterable_item_added": {
            "root[2]": 3,
            "root[3]": 5
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn list_difference_remove() {
    let t1 = json!([1, 2, 3]);
    let t2 = json!([1, 2]);
    let diff = common::diff(t1, t2);
    let expected = json!({
        "iterable_item_removed": {
            "root[2]": 3
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn list_difference_with_changes_and_truncation() {
    let t1 = json!([1, 2, 3, 10, 12]);
    let t2 = json!([1, 3, 2]);
    let diff = common::diff(t1, t2);
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
    assert_eq!(diff, expected);
}

#[test]
fn list_of_booleans() {
    let t1 = json!([false, false, true, true]);
    let t2 = json!([false, false, false, true]);
    let diff = common::diff(t1, t2);
    let expected = json!({
        "values_changed": {
            "root[2]": {"old_value": true, "new_value": false}
        }
    });
    assert_eq!(diff, expected);
}

#[test]
fn dict_none_item_removed() {
    let t1 = json!({"a": null, "b": 2});
    let t2 = json!({"b": 2});
    let diff = common::diff(t1, t2);
    let expected = json!({
        "dictionary_item_removed": ["root['a']"]
    });
    assert_eq!(diff, expected);
}

#[test]
fn list_none_item_removed() {
    let t1 = json!([1, 2, null]);
    let t2 = json!([1, 2]);
    let diff = common::diff(t1, t2);
    let expected = json!({
        "iterable_item_removed": {
            "root[2]": null
        }
    });
    assert_eq!(diff, expected);
}
