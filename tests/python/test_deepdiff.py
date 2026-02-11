import numpy as np
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

    diff = DeepDiff(0, 7e-7, significant_digits=1)
    assert diff.to_dict() == {}


def test_python_math_epsilon():
    diff = DeepDiff(1.0, 1.0005, math_epsilon=0.001)
    assert diff.to_dict() == {}


def test_python_atol_rtol():
    diff = DeepDiff(1.0, 1.0005, atol=0.001)
    assert diff.to_dict() == {}
    diff = DeepDiff(1000.0, 1000.1, rtol=1e-3)
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


def test_python_ignore_type_in_groups_bool_str():
    diff = DeepDiff(True, "Yes", ignore_type_in_groups=[(bool, str)])
    assert diff.to_dict() == {
        "values_changed": {"root": {"old_value": True, "new_value": "Yes"}}
    }


def test_python_ignore_type_in_groups_numbers():
    diff = DeepDiff(10, 10.0, ignore_type_in_groups=[(int, float)])
    assert diff.to_dict() == {}


def test_python_ignore_type_in_groups_numpy():
    diff = DeepDiff(10, np.float64(10), ignore_type_in_groups=[(int, np.float64)])
    assert diff.to_dict() == {}

    diff = DeepDiff([10, 5], np.array([10, 5]), ignore_type_in_groups=[(int, np.array)])
    assert diff.to_dict() == {}

    diff = DeepDiff([10, 5], np.array([10, 5.6]))
    assert diff.to_dict() == {
        "values_changed": {
            "root[0]": {"new_value": 10.0, "old_value": 10},
            "root[1]": {"new_value": 5.6, "old_value": 5},
        }
    }


def test_empty_diff_is_falsy():
    assert not DeepDiff({"a": 5}, {"a": 5})


def test_non_empty_diff_is_truthy():
    assert DeepDiff({"a": 5}, {"a": 6})
