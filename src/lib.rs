extern crate anyhow;
extern crate pyo3;
extern crate starlark;

use crate::pyo3::create_exception;
use crate::pyo3::exceptions::PyException;
use crate::pyo3::prelude::*;

use crate::starlark::syntax::Dialect;
use crate::starlark::values::Value;

create_exception!(starlark, StarlarkError, PyException);

fn convert_err<T>(err: Result<T, anyhow::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

// TODO: access to the linter

#[pyclass]
struct AstModule(starlark::syntax::AstModule);

#[pyfunction]
fn parse(filename: &str, content: &str) -> PyResult<AstModule> {
    Ok(AstModule(convert_err(starlark::syntax::AstModule::parse(
        filename,
        content.to_string(),
        &Dialect::Standard,
    ))?))
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

fn value_to_pyobject(value: Value) -> PyResult<PyObject> {
    let json_val = convert_err(value.to_json())?;
    Python::with_gil(|py| {
        let json = py.import("json")?;
        json.getattr("loads")?.call((json_val,), None)?.extract()
    })
}

#[pyclass]
struct Module(starlark::environment::Module);

#[pymethods]
impl Module {
    #[new]
    fn py_new() -> PyResult<Module> {
        Ok(Module(starlark::environment::Module::new()))
    }

    //fn set(&self, name: &str, value: PyObject) -> PyResult<()>
    //{
    //    self.set(name, );
    //    Ok(())
    //}
}

#[pyfunction]
fn eval(module: &mut Module, ast: &PyCell<AstModule>, globals: &Globals) -> PyResult<PyObject> {
    let empty_ast: starlark::syntax::AstModule = convert_err(starlark::syntax::AstModule::parse(
        "<empty>",
        "".to_string(),
        &Dialect::Standard,
    ))?;

    let mut evaluator = starlark::eval::Evaluator::new(&module.0);

    // stupid: eval consumes the ast, but starlark says so
    value_to_pyobject(convert_err(
        evaluator.eval_module(ast.replace(AstModule(empty_ast)).0, &globals.0),
    )?)
}

#[pymodule]
#[pyo3(name = "starlark")]
fn starlark_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<AstModule>()?;
    m.add_class::<Globals>()?;
    m.add_class::<Module>()?;
    m.add_wrapped(wrap_pyfunction!(parse))?;
    m.add_wrapped(wrap_pyfunction!(eval))?;
    m.add("StarlarkError", _py.get_type::<StarlarkError>())?;

    Ok(())
}
