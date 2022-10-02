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

    serious = 0
    not_serious = 0
    for lnt in ast.lint():
        print(lnt.serious, lnt)
        if lnt.serious:
            serious += 1
        else:
            not_serious += 1

    assert serious == 1
    assert not_serious == 2

# }}}


# {{{ python callalbles

PYTHON_CALLABLE_STAR = """
g(a)
"""


def test_python_callable():
    pass
    glb = sl.Globals.standard()
    mod = sl.Module()

    mod["a"] = 5

    def g(x):
        return 2*x

    mod.add_callable("g", g)

    ast = sl.parse("python-callable.star", PYTHON_CALLABLE_STAR)

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

    def load(name):
        if name == "zz.star":
            ast = sl.parse(name, "zz = 15")
            mod = sl.Module()
            sl.eval(mod, ast, glb)
            return mod.freeze()
        else:
            raise FileNotFoundError(name)

    ast = sl.parse("loading.star", LOADING_STAR)

    val = sl.eval(mod, ast, glb, sl.FileLoader(load))

    assert val == 15

# }}}


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        exec(sys.argv[1])
    else:
        from pytest import main
        main([__file__])

# vim: foldmethod=marker
