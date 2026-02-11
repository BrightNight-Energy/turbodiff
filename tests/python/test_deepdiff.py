import pytest
from turbodiff import DeepDiff


def test_python_value_change():
    diff = DeepDiff({"a": 1}, {"a": 2})
    assert diff.to_dict() == {
        "values_changed": {"root['a']": {"old_value": 1, "new_value": 2}}
    }


def test_python_type_change():
    diff = DeepDiff({"a": 1}, {"a": "1"})
    assert diff.to_dict() == {
        "type_changes": {
            "root['a']": {
                "old_type": "int",
                "new_type": "str",
                "old_value": 1,
                "new_value": "1",
            }
        }
    }


def test_python_verbose_level_zero():
    diff = DeepDiff({"a": 1}, {"a": 2}, verbose_level=0)
    assert diff.to_dict() == {"values_changed": ["root['a']"]}


def test_python_ignore_order():
    diff = DeepDiff([1, 2, 3], [3, 2, 1], ignore_order=True)
    assert diff.to_dict() == {}


def test_python_ignore_numeric_type_changes():
    diff = DeepDiff(10, 10.0, ignore_numeric_type_changes=True)
    assert diff.to_dict() == {}


def test_python_significant_digits():
    diff = DeepDiff(1.1234, 1.1235, significant_digits=3)
    assert diff.to_dict() == {}


def test_python_math_epsilon():
    diff = DeepDiff(1.0, 1.0005, math_epsilon=0.001)
    assert diff.to_dict() == {}


def test_python_include_paths():
    t1 = {"foo": {"bar": {"fruit": "apple"}}, "ingredients": ["bread"]}
    t2 = {"foo": {"bar": {"fruit": "peach"}}, "ingredients": ["bread"]}
    diff = DeepDiff(t1, t2, include_paths=["root['foo']"])
    assert diff.to_dict() == {
        "values_changed": {
            "root['foo']['bar']['fruit']": {"old_value": "apple", "new_value": "peach"}
        }
    }


def test_python_exclude_paths():
    t1 = {"keep": {"x": 1}, "skip": {"y": 1}}
    t2 = {"keep": {"x": 1}, "skip": {"y": 2}}
    diff = DeepDiff(t1, t2, exclude_paths=["root['skip']"])
    assert diff.to_dict() == {}


def test_python_unknown_param():
    with pytest.raises(ValueError):
        DeepDiff(1, 2, wrong_param=True)


def test_empty_diff_is_falsy():
    assert not DeepDiff({"a": 5}, {"a": 5})


def test_non_empty_diff_is_truthy():
    assert DeepDiff({"a": 5}, {"a": 6})
