from collections.abc import Sequence
from typing import Any, Callable, final

__all__: Sequence[str] = [
    "AstModule",
    "Dialect",
    "DialectTypes",
    "EvalSeverity",
    "FileLoader",
    "FrozenModule",
    "Globals",
    "LibraryExtension",
    "Lint",
    "Module",
    "ResolvedFileSpan",
    "ResolvedPos",
    "ResolvedSpan",
    "StarlarkError",
    "eval",
    "parse",
]

@final
class ResolvedPos:
    line: int
    column: int

@final
class ResolvedSpan:
    begin: ResolvedPos
    end: ResolvedPos

@final
class ResolvedFileSpan:
    file: str
    span: ResolvedSpan

class StarlarkError(Exception): ...

@final
class EvalSeverity:
    Error: EvalSeverity
    Warning: EvalSeverity
    Advice: EvalSeverity
    Disabled: EvalSeverity

@final
class Lint:
    resolved_location: ResolvedFileSpan
    short_name: str
    severity: EvalSeverity
    problem: str
    original: str

@final
class DialectTypes:
    DISABLE: DialectTypes
    PARSE_ONLY: DialectTypes
    ENABLE: DialectTypes

@final
class Dialect:
    enable_def: bool
    enable_lambda: bool
    enable_load: bool
    enable_keyword_only_arguments: bool
    enable_types: DialectTypes
    enable_load_reexport: bool
    enable_top_level_stmt: bool
    enable_f_strings: bool

    @staticmethod
    def standard() -> Dialect: ...
    @staticmethod
    def extended() -> Dialect: ...

@final
class AstModule:
    def lint(self) -> list[Lint]: ...

@final
class LibraryExtension:
    StructType: LibraryExtension
    RecordType: LibraryExtension
    EnumType: LibraryExtension
    Map: LibraryExtension
    Filter: LibraryExtension
    Partial: LibraryExtension
    Debug: LibraryExtension
    Print: LibraryExtension
    Pprint: LibraryExtension
    Breakpoint: LibraryExtension
    Json: LibraryExtension
    Typing: LibraryExtension
    Internal: LibraryExtension
    CallStack: LibraryExtension

@final
class Globals:
    @staticmethod
    def standard() -> Globals: ...
    @staticmethod
    def extended_by(extensions: list[LibraryExtension]) -> Globals: ...

@final
class FrozenModule: ...

@final
class Module:
    def __getitem__(self, key: str, /) -> Any: ...
    def __setitem__(self, key: str, value: Any, /) -> None: ...
    def add_callable(self, name: str, callable: Callable) -> None: ...
    def freeze(self) -> FrozenModule: ...

@final
class FileLoader:
    def __init__(self, load_func: Callable[[str], FrozenModule]) -> None: ...

def parse(filename: str, content: str, dialect: Dialect | None = None) -> AstModule: ...
def eval(
    module: Module,
    ast: AstModule,
    globals: Globals,
    file_loader: FileLoader | None = None,
) -> object: ...
