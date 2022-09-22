import starlark as sl
import timeit

A_STAR ="""
load("zz.star", "zz")
z = 3
z = 4

def f(x):
    z = 0
    for i in range(13):
        z += i*x
    return x*x - 5 + z

res = f(a - z + zz)
res
"""


glb = sl.Globals.standard()
mod = sl.Module()
mod["a"] = 5

ast = sl.parse("a.star", A_STAR)

def load(name):
    if name == "zz.star":
        ast = sl.parse(name, "zz = 15")
        mod = sl.Module()
        sl.eval(mod, ast, glb)
        return mod.freeze()
    else:
        raise FileNotFoundError(name)

for lnt in ast.lint():
    #print(lnt)
    #print(lnt.serious)
    pass

val = sl.eval(mod, ast, glb, sl.FileLoader(load))
print(val)

print(mod["res"])
