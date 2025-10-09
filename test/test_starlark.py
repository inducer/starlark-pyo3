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

    def g(x):
        return 2 * x

    mod.add_callable("g", g)

    ast = sl.parse("python-callable.star", PYTHON_CALLABLE_STAR)

    val = sl.eval(mod, ast, glb)

    assert val == 10


def test_python_callable_with_kwargs():
    glb = sl.Globals.standard()
    mod = sl.Module()

    mod["a"] = 5

    def g(x):
        return 2 * x

    mod.add_callable("g", g)

    ast = sl.parse("python-callable-with-kwargs.star", "g(x=a)")

    val = sl.eval(mod, ast, glb)

    assert val == 10

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
    ast = sl.parse("tc.star", TC_STAR, sl.Dialect.extended())

    errs, _iface, _ = ast.typecheck(glb, {})
    for err in errs:
        print(err)
        print(err.span)

    assert len(errs) == 2

# }}}


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        exec(sys.argv[1])
    else:
        from pytest import main
        _ = main([__file__])

# vim: foldmethod=marker
