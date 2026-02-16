import pytest


def pytest_addoption(parser):
    parser.addoption(
        "--print",
        action="store_true",
        default=False,
        help="Print DeepDiff.pretty() output during tests.",
    )


@pytest.fixture
def pretty_print(request):
    return request.config.getoption("--print")
