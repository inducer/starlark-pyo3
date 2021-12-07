extern crate anyhow;
extern crate pyo3;
extern crate starlark;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::Dialect;
use starlark::values::Value;

create_exception!(starlark, StarlarkError, PyException);

// TODO: expose classes
// TODO: access to the linter

#[pyclass]
struct AstModule(starlark::syntax::AstModule);

fn convert_err<T>(err: Result<T, anyhow::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

#[pyfunction]
fn parse(content: &str, filename: &str) -> PyResult<AstModule> {
    Ok(AstModule(convert_err(starlark::syntax::AstModule::parse(
        filename,
        content.to_string(),
        &Dialect::Standard,
    ))?))
}

fn run_str_inner(content: &str, filename: &str) -> PyResult<String> {
    let ast: starlark::syntax::AstModule = convert_err(starlark::syntax::AstModule::parse(
        filename,
        content.to_string(),
        &Dialect::Standard,
    ))?;

    let globals: Globals = Globals::standard();

    let module: Module = Module::new();

    let mut eval: Evaluator = Evaluator::new(&module);

    let res: Value = convert_err(eval.eval_module(ast, &globals))?;

    let json_res = convert_err(res.to_json())?;
    Ok(json_res)
}

#[pyfunction]
fn run_str(content: &str, filename: &str) -> PyResult<PyObject> {
    let res = run_str_inner(content, filename);
    Python::with_gil(|py| {
        let json = py.import("json")?;
        match res {
            Ok(s) => Ok(json.getattr("loads")?.call((s,), None)?.extract()?),
            Err(e) => Err(e),
        }
    })
}

#[pymodule]
fn starlark(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AstModule>()?;
    m.add_wrapped(wrap_pyfunction!(parse))?;
    m.add_wrapped(wrap_pyfunction!(run_str))?;
    m.add("StarlarkError", _py.get_type::<StarlarkError>())?;

    Ok(())
}
