import starlark as sl

# Two ways to execute a denial of service attack against the interpreter.

A_STAR = """
def dos(depth):
    if depth:
        depth -= 1
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)
        dos(depth)

#dos(30)

def dos2():
    for i in range(1000):
        for i in range(1000):
            for i in range(1000):
                pass

#dos2()
"""

ast = sl.parse("a.star", A_STAR)
glb = sl.Globals.standard()
mod = sl.Module()
val = sl.eval(mod, ast, glb)
