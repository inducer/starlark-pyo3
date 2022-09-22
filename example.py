import starlark as sl

glb = sl.Globals()
mod = sl.Module()
mod["a"] = 5

ast = sl.parse("a.star", """
z = 3
z = 4

def f(x):
    z = 0
    for i in range(13):
        z += i*x
    return x*x - 5 + z


f(a - z)
""")

for lnt in ast.lint():
    print(lnt)
    print(lnt.serious)

val = sl.eval(mod, ast, glb)
print(val)
