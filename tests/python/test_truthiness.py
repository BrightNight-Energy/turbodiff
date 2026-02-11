from turbodiff import DeepDiff


def test_empty_diff_is_falsy():
    assert not DeepDiff({"a": 5}, {"a": 5})


def test_non_empty_diff_is_truthy():
    assert DeepDiff({"a": 5}, {"a": 6})
