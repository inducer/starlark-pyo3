extern crate anyhow;
extern crate pyo3;
extern crate starlark;

use crate::pyo3::create_exception;
use crate::pyo3::exceptions::PyException;
use crate::pyo3::prelude::*;
use std::cell::Cell;

use crate::starlark::syntax::Dialect;
use crate::starlark::values::Value;

create_exception!(starlark, StarlarkError, PyException);

fn convert_err<T>(err: Result<T, anyhow::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

// TODO: expose classes
// TODO: access to the linter

#[pyclass]
struct AstModule(Cell<starlark::syntax::AstModule>);

#[pyfunction]
fn parse(filename: &str, content: &str) -> PyResult<AstModule> {
    Ok(AstModule(Cell::new(convert_err(
        starlark::syntax::AstModule::parse(filename, content.to_string(), &Dialect::Standard),
    )?)))
}

#[pyclass]
struct Globals(starlark::environment::Globals);

#[pymethods]
impl Globals {
    #[new]
    fn py_new() -> PyResult<Globals> {
        Ok(Globals(starlark::environment::Globals::standard()))
    }
}

#[pyclass]
struct Module(starlark::environment::Module);

#[pymethods]
impl Module {
    #[new]
    fn py_new() -> PyResult<Module> {
        Ok(Module(starlark::environment::Module::new()))
    }
}

fn eval_inner(
    module: &mut starlark::environment::Module,
    ast: starlark::syntax::AstModule,
    globals: &starlark::environment::Globals,
) -> PyResult<String> {
    let mut evaluator = starlark::eval::Evaluator::new(&module);
    let res: Value = convert_err(evaluator.eval_module(ast, globals))?;
    let json_res = convert_err(res.to_json())?;
    Ok(json_res)
}

#[pyfunction]
fn eval(module: &mut Module, ast: &mut AstModule, globals: &Globals) -> PyResult<PyObject> {
    let empty_ast: starlark::syntax::AstModule = convert_err(starlark::syntax::AstModule::parse(
        "<empty>",
        "".to_string(),
        &Dialect::Standard,
    ))?;

    // stupid: eval consumes the ast, but that's not our fault
    let res = eval_inner(&mut module.0, ast.0.replace(empty_ast), &globals.0);

    Python::with_gil(|py| {
        let json = py.import("json")?;
        match res {
            Ok(s) => Ok(json.getattr("loads")?.call((s,), None)?.extract()?),
            Err(e) => Err(e),
        }
    })
}

#[pymodule]
#[pyo3(name="starlark")]
fn starlark_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AstModule>()?;
    m.add_class::<Globals>()?;
    m.add_class::<Module>()?;
    m.add_wrapped(wrap_pyfunction!(parse))?;
    m.add_wrapped(wrap_pyfunction!(eval))?;
    m.add("StarlarkError", _py.get_type::<StarlarkError>())?;

    Ok(())
}
