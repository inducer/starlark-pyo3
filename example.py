import starlark as sl

glb = sl.Globals()
mod = sl.Module()
mod["a"] = 5
ast = sl.parse("a.py", "a+4")
val = sl.eval(mod, ast, glb)
print(val)
