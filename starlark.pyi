from typing import Any, Callable, Optional

class ResolvedPos:
    line: int
    column: int

class ResolvedSpan:
    begin: ResolvedPos
    end: ResolvedPos

class ResolvedFileSpan:
    file: str
    span: ResolvedSpan

class StarlarkError(Exception): ...

class EvalSeverity:
    Error: EvalSeverity
    Warning: EvalSeverity
    Advice: EvalSeverity
    Disabled: EvalSeverity

class Lint:
    resolved_location: ResolvedFileSpan
    short_name: str
    severity: EvalSeverity
    problem: str
    original: str

class DialectTypes:
    DISABLE: DialectTypes
    PARSE_ONLY: DialectTypes
    ENABLE: DialectTypes

class Dialect:
    enable_def: bool
    enable_lambda: bool
    enable_load: bool
    enable_keyword_only_arguments: bool
    enable_types: DialectTypes
    enable_load_reexport: bool
    enable_top_level_stmt: bool
    enable_f_strings: bool

    @classmethod
    def standard(cls) -> "Dialect": ...
    @classmethod
    def extended(cls) -> "Dialect": ...

class AstModule:
    def lint(self) -> list[Lint]: ...

class LibraryExtension:
    StructType: LibraryExtension
    RecordType: LibraryExtension
    EnumType: LibraryExtension
    Map: LibraryExtension
    Filter: LibraryExtension
    Partial: LibraryExtension
    ExperimentalRegex: LibraryExtension
    Debug: LibraryExtension
    Print: LibraryExtension
    Pprint: LibraryExtension
    Breakpoint: LibraryExtension
    Json: LibraryExtension
    Typing: LibraryExtension
    Internal: LibraryExtension
    CallStack: LibraryExtension

class Globals:
    @classmethod
    def standard(cls) -> "Globals": ...
    @classmethod
    def extended_by(cls, extensions: list[LibraryExtension]) -> "Globals": ...

class FrozenModule: ...

class Module:
    def __getitem__(self, key: str, /) -> Any: ...
    def __setitem__(self, key: str, value: Any, /) -> None: ...
    def add_callable(self, name: str, callable: Callable) -> None: ...
    def freeze(self) -> FrozenModule: ...

class FileLoader:
    def __init__(self, load_func: Callable[[str], FrozenModule]) -> None: ...

def parse(
    filename: str, content: str, dialect: Optional[Dialect] = None
) -> AstModule: ...
def eval(
    module: Module,
    ast: AstModule,
    globals: Globals,
    file_loader: Optional[FileLoader] = None,
) -> object: ...
