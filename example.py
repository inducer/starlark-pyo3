import starlark as sl

glb = sl.Globals()
mod = sl.Module()
mod["a"] = 5
ast = sl.parse("a.py", """
z = 3
z = 4

def f(x):
    return x*x - 5

f(a - z)
""")
for lnt in ast.lint():
    print(lnt)
    print(lnt.serious)
val = sl.eval(mod, ast, glb)
print(val)
