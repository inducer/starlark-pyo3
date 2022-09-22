import starlark as sl

ast = sl.parse("a.py", "3+4")
mod = sl.Module()
glb = sl.Globals()
val = sl.eval(mod, ast, glb)
print(val)


