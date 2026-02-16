from typing import Any, Iterable

__all__: list[str]

class DeepDiff:
    def __init__(
        self,
        t1: Any,
        t2: Any,
        *,
        ignore_order: bool = ...,
        ignore_numeric_type_changes: bool = ...,
        ignore_string_type_changes: bool = ...,
        ignore_type_in_groups: Iterable[Iterable[type]] | None = ...,
        significant_digits: int | None = ...,
        math_epsilon: float | None = ...,
        atol: float | None = ...,
        rtol: float | None = ...,
        include_paths: Iterable[str] | None = ...,
        exclude_paths: Iterable[str] | None = ...,
        verbose_level: int = ...,
    ) -> None: ...
    def to_dict(self) -> dict[str, Any]: ...
    def pretty(
        self,
        *,
        compact: bool = ...,
        max_depth: int = ...,
        context: int = ...,
        no_color: bool = ...,
        path_header: bool = ...,
    ) -> str: ...
    def __repr__(self) -> str: ...
    def __bool__(self) -> bool: ...
    def __len__(self) -> int: ...
