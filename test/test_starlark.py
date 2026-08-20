from dataclasses import dataclass

import pytest

import starlark as sl


# {{{ linter

LINT_STAR = """
z = 3
z = 4

def f():
    for i in range(10):
        for i in range(10):
            pass
"""


def test_linter():
    ast = sl.parse("lint.star", LINT_STAR)

    severities: dict[str, int] = {}
    for lnt in ast.lint():
        print(lnt.severity, lnt)
        severities[repr(lnt.severity)] = severities.get(repr(lnt.severity), 0) + 1

    assert severities == {"Warning": 1, "Disabled": 3}

# }}}


# {{{ python callalbles

PYTHON_CALLABLE_STAR = """
g(a)
"""


def test_python_callable():
    glb = sl.Globals.standard()
    mod = sl.Module()

    mod["a"] = 5

    def g(x: int):
        return 2 * x

    mod.add_callable("g", g)

    ast = sl.parse("python-callable.star", PYTHON_CALLABLE_STAR)

    val = sl.eval(mod, ast, glb)

    assert val == 10


def test_python_callable_with_kwargs():
    glb = sl.Globals.standard()
    mod = sl.Module()

    mod["a"] = 5

    def g(x: int):
        return 2 * x

    mod.add_callable("g", g)

    ast = sl.parse("python-callable-with-kwargs.star", "g(x=a)")

    val = sl.eval(mod, ast, glb)

    assert val == 10


def test_python_callable_negative_int_preserves_type():
    # A registered Python callable that returns an int must produce an int on
    # the Starlark -> Python boundary, including negatives. The JSON-based
    # value conversion needs an i64 branch; without it, negatives fall
    # through to f64 and emerge on the Python side as float.
    glb = sl.Globals.standard()
    mod = sl.Module()

    captured: dict[str, int] = {}

    def return_int(x: int) -> int:
        captured["x"] = x
        return x

    mod.add_callable("return_int", return_int)

    cases: tuple[tuple[str, int], ...] = (
        ("-(2 * 1024 * 1024 * 1024)", -(2 * 1024 * 1024 * 1024)),
        ("-100", -100),
        ("-1", -1),
        ("0", 0),
        ("1", 1),
        ("100", 100),
        ("(2 * 1024 * 1024 * 1024) - 1", (2 * 1024 * 1024 * 1024) - 1),
    )
    for literal, expected in cases:
        ast = sl.parse("neg-int.star", f"return_int({literal})")
        result = sl.eval(mod, ast, glb)
        assert result == expected
        assert isinstance(result, int) and not isinstance(result, bool), (
            f"expected int, got {type(result).__name__}={result!r}"
        )
        assert isinstance(captured["x"], int) and not isinstance(captured["x"], bool), (
            f"callable received {type(captured['x']).__name__}={captured['x']!r}"
        )


ADD_STAR = """
def add(x, y, a, b):
    if a != "a":
        return 0
    if b != "b":
        return 0
    return x + y
"""


def test_call_starlark():
    ast = sl.parse("add.star", ADD_STAR)
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, ast, glb)
    fmod = mod.freeze()
    assert fmod.call("add", 3, 4, b="b", a="a") == 7

# }}}


# {{{ module loading

LOADING_STAR = """
load("zz.star", "zz")
zz
"""


def test_module_loading():
    glb = sl.Globals.standard()
    mod = sl.Module()

    def load(name: str):
        if name == "zz.star":
            ast = sl.parse(name, "zz = 15")
            mod = sl.Module()
            _ = sl.eval(mod, ast, glb)
            return mod.freeze()
        else:
            raise FileNotFoundError(name)

    ast = sl.parse("loading.star", LOADING_STAR)
    ld, = ast.loads()
    assert ld.module_id == "zz.star"
    assert ld.symbols == {"zz": "zz"}

    val = sl.eval(mod, ast, glb, sl.FileLoader(load))

    assert val == 15


TC_STAR = """
def f(x: int) -> int:
    return "x" * "x"  # FIXME: not an error?

def test():
    z = 0x60000000000000000000000 | 1.0

def test2():
    l = []
    l.oppend(5)  # spellchecker: disable-line
"""


def test_type_check():
    glb = sl.Globals.standard()
    dialect = sl.Dialect.extended()
    dialect.enable_types = sl.DialectTypes.ENABLE
    ast = sl.parse("tc.star", TC_STAR, dialect)

    errs, _iface, _ = ast.typecheck(glb, {})
    for err in errs:
        print(err)
        print(err.span)

    assert len(errs) == 2


EXT_TYPE_STAR = """
FlowersEnum = enum("daisies", "roses", "posies")

EmployeeRecord = record(
    id=int,
    name=str,
    salary=float,
)

def get_custom_types():
    return (
        FlowersEnum("daisies"),
        EmployeeRecord(id=1, name="John Doe", salary=5.0),
        struct(id=1, name="John Doe", salary=5.0),
    )
"""


def test_ext_type_conversion():
    ast = sl.parse("ext-type.star", EXT_TYPE_STAR)
    glb = (sl.Globals.standard().extended_by([
                sl.LibraryExtension.StructType,
                sl.LibraryExtension.EnumType,
                sl.LibraryExtension.RecordType,
            ]))
    mod = sl.Module()
    sl.eval(mod, ast, glb)
    fmod = mod.freeze()
    retval = fmod.call("get_custom_types")
    empl = {"id": 1, "name": "John Doe", "salary": 5.0}
    assert retval == ["daisies", empl, empl]


@dataclass
class MyObj:
    x: int


def test_opaue_python_obj():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("ext-type.star", "def identity(x): return x")
    sl.eval(mod, ast, glb)
    fmod = mod.freeze()
    myobj = MyObj(5)
    myobj2 = fmod.call("identity", sl.OpaquePythonObject(myobj))
    assert myobj is myobj2

# }}}


# {{{ evaluation options

INFINITE_LOOP_STAR = """
def busy():
    x = 0
    for _ in range(1000000000):
        x = x + 1
    return x
busy()
"""


RECURSE_STAR = """
def f(n):
    if n == 0:
        return 0
    return f(n - 1) + 1
f(30)
"""


def test_check_cancelled_aborts_evaluation():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("cancel.star", INFINITE_LOOP_STAR)

    counter: dict[str, int] = {"n": 0}

    def cancel():
        counter["n"] += 1
        return counter["n"] >= 1

    with pytest.raises(sl.StarlarkError):
        sl.eval_with(sl.EvalOptions(check_cancelled=cancel), mod, ast, glb)

    assert counter["n"] >= 1


def test_check_cancelled_never_triggers_for_quick_eval():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("quick.star", "1 + 2")

    counter: dict[str, int] = {"n": 0}

    def cancel():
        counter["n"] += 1
        return False

    result = sl.eval_with(sl.EvalOptions(check_cancelled=cancel), mod, ast, glb)
    assert result.value == 3
    assert counter["n"] == 0


def test_check_cancelled_aborts_on_truthy_int():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("truthy.star", INFINITE_LOOP_STAR)

    def cancel():
        return 1

    with pytest.raises(sl.StarlarkError):
        # Intentional non-bool return: verifies is_truthy semantics.
        sl.eval_with(sl.EvalOptions(check_cancelled=cancel), mod, ast, glb)  # pyright: ignore[reportArgumentType]


def test_check_cancelled_rejects_non_callable():
    with pytest.raises(TypeError, match="callable"):
        # Intentional non-callable: verifies runtime rejection at
        # EvalOptions construction.
        sl.EvalOptions(check_cancelled=42)  # pyright: ignore[reportArgumentType]


def test_check_cancelled_propagates_python_exception():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("cancel-raises.star", INFINITE_LOOP_STAR)

    class BoomError(RuntimeError):
        pass

    def cancel():
        raise BoomError("kaboom")

    with pytest.raises(BoomError, match="kaboom"):
        sl.eval_with(sl.EvalOptions(check_cancelled=cancel), mod, ast, glb)


def test_max_callstack_size_limits_recursion():
    # Positive control: depth 30 completes under starlark-rust's default
    # 50-frame call stack limit, so a plain sl.eval() succeeds.
    glb = sl.Globals.standard()
    ctrl_mod = sl.Module()
    ctrl_ast = sl.parse("recurse.star", RECURSE_STAR)
    assert sl.eval(ctrl_mod, ctrl_ast, glb) == 30

    # With max_callstack_size=10, the same script overflows the imposed limit.
    mod = sl.Module()
    ast = sl.parse("recurse.star", RECURSE_STAR)
    with pytest.raises(sl.StarlarkError, match="call stack overflow"):
        sl.eval_with(sl.EvalOptions(max_callstack_size=10), mod, ast, glb)


def test_eval_options_max_callstack_size_zero_rejected():
    with pytest.raises(ValueError, match="positive"):
        sl.EvalOptions(max_callstack_size=0)


def test_eval_options_getters_readable():
    def cb() -> bool:
        return False

    opts = sl.EvalOptions(check_cancelled=cb, max_callstack_size=42)
    assert opts.check_cancelled is cb
    assert opts.max_callstack_size == 42

    empty = sl.EvalOptions()
    assert empty.check_cancelled is None
    assert empty.max_callstack_size is None


FROZEN_LOOP_STAR = """
def loop():
    for _ in range(1000000000):
        pass
    return 0
"""


FROZEN_RECURSE_STAR = """
def f(n):
    if n == 0:
        return 0
    return f(n - 1) + 1
"""


def test_frozen_module_call_with_check_cancelled_aborts():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("frozen-loop.star", FROZEN_LOOP_STAR), glb)
    fmod = mod.freeze()

    counter: dict[str, int] = {"n": 0}

    def cancel():
        counter["n"] += 1
        return counter["n"] >= 1

    with pytest.raises(sl.StarlarkError):
        fmod.call_with(sl.EvalOptions(check_cancelled=cancel), "loop")
    assert counter["n"] >= 1


def test_frozen_module_call_with_max_callstack_size_limits_recursion():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("frozen-recurse.star", FROZEN_RECURSE_STAR), glb)
    fmod = mod.freeze()

    # Positive control: depth 30 completes under the default 50-frame limit.
    assert fmod.call_with(sl.EvalOptions(), "f", 30).value == 30

    # With max_callstack_size=10, the same call overflows.
    with pytest.raises(sl.StarlarkError, match="call stack overflow"):
        fmod.call_with(sl.EvalOptions(max_callstack_size=10), "f", 30)


def test_frozen_module_call_with_propagates_python_exception():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("frozen-loop2.star", FROZEN_LOOP_STAR), glb)
    fmod = mod.freeze()

    class BoomError(RuntimeError):
        pass

    def cancel():
        raise BoomError("kaboom")

    with pytest.raises(BoomError, match="kaboom"):
        fmod.call_with(sl.EvalOptions(check_cancelled=cancel), "loop")


NAMESPACE_STAR = """
def f(check_cancelled, max_callstack_size, options, name):
    return check_cancelled + max_callstack_size + options + name
"""


def test_frozen_module_call_with_kwargs_pristine():
    # Every kwarg name that would collide on a flat-kwarg design must
    # flow through to the Starlark callee unmodified.
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("nc.star", NAMESPACE_STAR), glb)
    fmod = mod.freeze()

    result = fmod.call_with(
        sl.EvalOptions(),
        "f",
        check_cancelled=1,
        max_callstack_size=2,
        options=3,
        name=4,
    )
    assert result.value == 10


def test_eval_with_returns_eval_result():
    glb = sl.Globals.standard()
    mod = sl.Module()
    ast = sl.parse("basic.star", "1 + 2")

    result = sl.eval_with(sl.EvalOptions(), mod, ast, glb)
    assert isinstance(result, sl.EvalResult)
    assert result.value == 3


def test_frozen_module_call_with_returns_eval_result():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("basic.star", "def f(): return 42"), glb)
    fmod = mod.freeze()

    result = fmod.call_with(sl.EvalOptions(), "f")
    assert isinstance(result, sl.EvalResult)
    assert result.value == 42

# }}}


# {{{ cyclic values

def _eval_module(content, glb=None):
    if glb is None:
        glb = sl.Globals.standard()
    mod = sl.Module()
    dialect = sl.Dialect.standard()
    dialect.enable_top_level_stmt = True
    ast = sl.parse("cycle.star", content, dialect)
    sl.eval(mod, ast, glb)
    return mod


def test_cyclic_list_raises_instead_of_crashing():
    # A cyclic list used to overflow the C stack and crash the interpreter
    # (SIGSEGV) when read back from the module.
    mod = _eval_module("l = []\nl.append(l)\nresult = l\n")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        _ = mod["result"]


def test_cyclic_dict_raises():
    mod = _eval_module("d = {}\nd['self'] = d\nresult = d\n")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        _ = mod["result"]


def test_mutual_list_dict_cycle_raises():
    mod = _eval_module("l = []\nd = {}\nl.append(d)\nd['l'] = l\nresult = l\n")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        _ = mod["result"]


def test_cycle_through_tuple_raises():
    # A tuple itself cannot contain itself (it is immutable), but a list
    # and a tuple can form a cycle between them.
    mod = _eval_module("l = []\nt = (l,)\nl.append(t)\nresult = l\n")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        _ = mod["result"]


def test_cyclic_list_as_eval_result_raises():
    glb = sl.Globals.standard()
    mod = sl.Module()
    dialect = sl.Dialect.standard()
    dialect.enable_top_level_stmt = True
    ast = sl.parse("cycle-eval.star", "l = []\nl.append(l)\nl\n", dialect)
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        sl.eval(mod, ast, glb)


CYCLIC_FN_STAR = """
def get_cycle():
    l = []
    l.append(l)
    return l
"""


def test_cyclic_list_via_frozen_call_raises():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("cycle-fn.star", CYCLIC_FN_STAR), glb)
    fmod = mod.freeze()
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        fmod.call("get_cycle")


def test_cyclic_list_into_python_callable_raises():
    glb = sl.Globals.standard()
    mod = sl.Module()
    mod.add_callable("consume", lambda x: 0)
    ast = sl.parse("cycle-arg.star", "l = []\nl.append(l)\nconsume(l)\n")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        sl.eval(mod, ast, glb)


def test_deep_non_cyclic_nesting_within_limit_succeeds():
    mod = _eval_module("result = [1]\nfor i in range(500):\n    result = [result]\n")
    r = mod["result"]
    for _ in range(500):
        assert isinstance(r, list) and len(r) == 1
        r = r[0]
    assert r == [1]


def test_deep_non_cyclic_nesting_beyond_limit_raises():
    mod = _eval_module("result = [1]\nfor i in range(1500):\n    result = [result]\n")
    with pytest.raises(sl.StarlarkError, match="Maximum depth"):
        _ = mod["result"]


def test_shared_reference_is_not_a_false_cycle():
    # Referencing the same value twice is a DAG, not a cycle, and must
    # convert normally.
    mod = _eval_module("x = [1, 2]\nresult = [x, x]\n")
    assert mod["result"] == [[1, 2], [1, 2]]


def test_cyclic_python_list_raises_instead_of_crashing():
    # Cyclic Python objects used to overflow the native stack and crash the
    # interpreter (SIGSEGV) when converted into the module.
    mod = sl.Module()
    lst = []
    lst.append(lst)
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        mod["x"] = lst


def test_cyclic_python_dict_raises():
    mod = sl.Module()
    d = {}
    d["self"] = d
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        mod["x"] = d


def test_cyclic_python_list_via_frozen_call_raises():
    glb = sl.Globals.standard()
    mod = sl.Module()
    sl.eval(mod, sl.parse("cycle-arg.star", "def identity(x): return x"), glb)
    fmod = mod.freeze()
    lst = []
    lst.append(lst)
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        fmod.call("identity", lst)
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        fmod.call("identity", a=lst)


def test_cyclic_python_result_from_callable_raises():
    glb = sl.Globals.standard()
    mod = sl.Module()

    def make_cycle():
        lst = []
        lst.append(lst)
        return lst

    mod.add_callable("make_cycle", make_cycle)
    ast = sl.parse("cycle-ret.star", "make_cycle()")
    with pytest.raises(sl.StarlarkError, match="Cycle detected"):
        sl.eval(mod, ast, glb)


def test_deep_python_nesting_within_limit_succeeds():
    mod = sl.Module()
    lst = [1]
    for _ in range(500):
        lst = [lst]
    mod["x"] = lst
    r = mod["x"]
    for _ in range(500):
        r = r[0]
    # One wrap remains: the original [1] list.
    assert r == [1]


def test_deep_python_nesting_beyond_limit_raises():
    mod = sl.Module()
    lst = [1]
    for _ in range(1500):
        lst = [lst]
    with pytest.raises(sl.StarlarkError, match="Maximum depth"):
        mod["x"] = lst

# }}}


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        exec(sys.argv[1])
    else:
        from pytest import main
        _ = main([__file__])

# vim: foldmethod=marker
