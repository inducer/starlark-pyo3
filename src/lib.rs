/*
 * Copyright 2022 University of Illinois Board of Trustees
 * Copyright 2018 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

extern crate anyhow;
extern crate gazebo;
extern crate pyo3;
extern crate serde_json;
extern crate starlark;
extern crate thiserror;

use std::fmt::{self, Display};

use crate::pyo3::create_exception;
use crate::pyo3::exceptions::PyException;
use crate::pyo3::prelude::*;

use crate::starlark::collections::SmallMap;
use crate::starlark::syntax::Dialect;
use crate::starlark::values::Heap;
use crate::starlark::values::Value;
use gazebo::prelude::*;
use starlark::values::dict::Dict;
use thiserror::Error;

create_exception!(starlark, StarlarkError, PyException);

// {{{ value conversion

// {{{ copied from starlark stdlib

// https://github.com/facebookexperimental/starlark-rust/blob/6b2954ef1aba09b88fcbac346885fc4128eb22e0/starlark/src/stdlib/json.rs

#[derive(Debug, Error)]
enum JsonError {
    #[error("Number can't be represented, perhaps a float value that is too precise, got `{0}")]
    UnrepresentableNumber(String),
}

fn serde_to_starlark<'v>(x: serde_json::Value, heap: &'v Heap) -> anyhow::Result<Value<'v>> {
    match x {
        serde_json::Value::Null => Ok(Value::new_none()),
        serde_json::Value::Bool(x) => Ok(Value::new_bool(x)),
        serde_json::Value::Number(x) => {
            if let Some(x) = x.as_u64() {
                Ok(heap.alloc(x))
            } else if let Some(x) = x.as_f64() {
                Ok(heap.alloc(x))
            //} else if let Ok(x) = BigInt::from_str(&x.to_string()) {
            // Ok(StarlarkBigInt::alloc_bigint(x, heap))
            } else {
                Err(JsonError::UnrepresentableNumber(x.to_string()).into())
            }
        }
        serde_json::Value::String(x) => Ok(heap.alloc(x)),
        serde_json::Value::Array(x) => {
            Ok(heap.alloc_list_iter(x.into_try_map(|v| serde_to_starlark(v, heap))?))
        }
        serde_json::Value::Object(x) => {
            let mut mp = SmallMap::with_capacity(x.len());
            for (k, v) in x {
                let k = heap.alloc_str(&k).get_hashed_value();
                let v = serde_to_starlark(v, heap)?;
                mp.insert_hashed(k, v);
            }
            Ok(heap.alloc(Dict::new(mp)))
        }
    }
}

// }}}

fn value_to_pyobject(value: Value) -> PyResult<PyObject> {
    let json_val = convert_err(value.to_json())?;
    Python::with_gil(|py| {
        let json = py.import("json")?;
        json.getattr("loads")?.call((json_val,), None)?.extract()
    })
}

fn pyobject_to_value<'v>(obj: PyObject, heap: &'v Heap) -> PyResult<Value<'v>> {
    Python::with_gil(|py| -> PyResult<Value<'v>> {
        let json = py.import("json")?;
        let json_str: String = json.getattr("dumps")?.call((obj,), None)?.extract()?;
        convert_err(serde_to_starlark(
            convert_serde_err(serde_json::from_str(&json_str))?,
            heap,
        ))
    })
}

// }}}

// {{{ result conversions

fn convert_err<T>(err: Result<T, anyhow::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

fn convert_serde_err<T>(err: Result<T, serde_json::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(format!("{}", e))),
    }
}

// }}}

// {{{ ResolvedSpan

#[pyclass]
struct ResolvedSpan(starlark::codemap::ResolvedSpan);

#[pymethods]
impl ResolvedSpan {
    #[getter]
    fn begin_line(&self) -> usize {
        self.0.begin_line
    }
    fn begin_column(&self) -> usize {
        self.0.begin_column
    }
    fn end_line(&self) -> usize {
        self.0.end_line
    }
    fn end_column(&self) -> usize {
        self.0.end_column
    }
}

// }}}

// {{{ ResolvedFileSpan

#[pyclass]
struct ResolvedFileSpan(starlark::codemap::ResolvedFileSpan);

#[pymethods]
impl ResolvedFileSpan {
    #[getter]
    fn file(&self) -> String {
        self.0.file.clone()
    }
    #[getter]
    fn span(&self) -> ResolvedSpan {
        ResolvedSpan(self.0.span)
    }
}

// }}}

// {{{ Lint

#[pyclass]
struct Lint {
    pub location: starlark::codemap::FileSpan,
    #[pyo3(get)]
    pub short_name: String,
    #[pyo3(get)]
    pub serious: bool,
    #[pyo3(get)]
    pub problem: String,
    #[pyo3(get)]
    pub original: String,
}

impl Display for Lint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.problem)
    }
}

#[pymethods]
impl Lint {
    #[getter]
    fn resolved_location(&self) -> ResolvedFileSpan {
        ResolvedFileSpan(self.location.resolve())
    }
    fn __str__(&self) -> String {
        self.to_string()
    }
}


// }}}

// {{{ AstModule

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

#[pymethods]
impl AstModule {
    fn lint(&self) -> Vec<Lint> {
        self.0.lint(None).map(|lint| Lint {
            location: lint.location.dupe(),
            short_name: lint.short_name.clone(),
            serious: lint.serious,
            problem: lint.problem.clone(),
            original: lint.original.clone(),
        })
    }
}

// }}}

// {{{ Globals

#[pyclass]
struct Globals(starlark::environment::Globals);

#[pymethods]
impl Globals {
    #[new]
    fn py_new() -> PyResult<Globals> {
        Ok(Globals(starlark::environment::Globals::standard()))
    }
}

// }}}

// {{{ Module

#[pyclass]
struct Module(starlark::environment::Module);

#[pymethods]
impl Module {
    #[new]
    fn py_new() -> PyResult<Module> {
        Ok(Module(starlark::environment::Module::new()))
    }

    fn __setitem__(&self, name: &str, obj: PyObject) -> PyResult<()> {
        self.0.set(name, pyobject_to_value(obj, self.0.heap())?);
        Ok(())
    }
}

// }}}

// {{{ eval

#[pyfunction]
fn eval(module: &mut Module, ast: &PyCell<AstModule>, globals: &Globals) -> PyResult<PyObject> {
    let empty_ast: starlark::syntax::AstModule = convert_err(starlark::syntax::AstModule::parse(
        "<empty>",
        "".to_string(),
        &Dialect::Standard,
    ))?;

    let mut evaluator = starlark::eval::Evaluator::new(&module.0);

    // Stupid: eval_module consumes the AST.
    // Python would like it to live on,  but starlark-rust says no.
    value_to_pyobject(convert_err(
        evaluator.eval_module(ast.replace(AstModule(empty_ast)).0, &globals.0),
    )?)
}

// }}}

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

// vim: foldmethod=marker
