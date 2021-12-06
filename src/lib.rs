extern crate anyhow;
extern crate pyo3;
extern crate starlark;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use starlark::environment::{Globals, Module};
use starlark::eval::Evaluator;
use starlark::syntax::{AstModule, Dialect};
use starlark::values::Value;

create_exception!(starlark, StarlarkError, PyException);

// TODO: expose classes
// TODO: access to the linter

fn run_str_inner(content: &str, filename: &str) -> Result<String, anyhow::Error> {
    let ast: AstModule = AstModule::parse(filename, content.to_string(), &Dialect::Standard)?;

    let globals: Globals = Globals::standard();

    let module: Module = Module::new();

    let mut eval: Evaluator = Evaluator::new(&module);

    let res: Value = eval.eval_module(ast, &globals)?;

    let json_res = res.to_json()?;
    Ok(json_res)
}

#[pyfunction]
fn run_str(content: &str, filename: &str) -> PyResult<PyObject> {
    let res = run_str_inner(content, filename);
    Python::with_gil(|py| {
        let json = py.import("json")?;
        match res {
            Ok(s) => Ok(json.getattr("loads")?.call((s,), None)?.extract()?),
            Err(e) => Err(StarlarkError::new_err(e.to_string())),
        }
    })
}

#[pymodule]
fn starlark(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_wrapped(wrap_pyfunction!(run_str))?;
    m.add("CustomError", _py.get_type::<StarlarkError>())?;

    Ok(())
}
