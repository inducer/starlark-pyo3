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
#![feature(iterator_try_collect)]
#![feature(trivial_bounds)]

extern crate allocative;
extern crate anyhow;
extern crate dupe;
extern crate gazebo;
extern crate pyo3;
extern crate serde_json;
extern crate starlark;
extern crate thiserror;

use std::fmt::{self, Display};

use crate::pyo3::create_exception;
use crate::pyo3::exceptions::PyException;
use crate::pyo3::prelude::*;

use gazebo::prelude::*;

use crate::starlark::collections::SmallMap;
use allocative::Allocative;
use dupe::Dupe;
use pyo3::types::PyTuple;
use starlark::eval::Arguments;
use starlark::values::dict::Dict;
use starlark::values::list::AllocList;
use starlark::values::{Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value};
use starlark::{starlark_simple_value, starlark_type};
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
            Ok(heap.alloc(AllocList(x.into_try_map(|v| serde_to_starlark(v, heap))?)))
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
        json.getattr("loads")?.call1((json_val,))?.extract()
    })
}

fn pyobject_to_value<'v>(obj: PyObject, heap: &'v Heap) -> PyResult<Value<'v>> {
    Python::with_gil(|py| -> PyResult<Value<'v>> {
        let json = py.import("json")?;
        let json_str: String = json.getattr("dumps")?.call1((obj,))?.extract()?;
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

fn convert_to_anyhow<T>(err: Result<T, PyErr>) -> Result<T, anyhow::Error> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(anyhow::Error::new(e)),
    }
}

// }}}

// {{{ ResolvedSpan

/// .. autoattribute:: begin_line
///
///     A :class:`int`.
/// .. autoattribute:: begin_column
///
///     A :class:`int`.
/// .. autoattribute:: end_line
///
///     A :class:`int`.
/// .. autoattribute:: end_column
///
///     A :class:`int`.
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

/// .. autoattribute:: file
///
///     A :class:`str`.
/// .. autoattribute:: span
///
///     A :class:`ResolvedSpan`.
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

/// .. automethod:: __str__
///
/// .. autoattribute:: resolved_location
///
///     A :class:`ResolvedFileSpan`.
/// .. autoattribute:: short_name
///
///     A :class:`str`.
/// .. autoattribute:: serious
///
///     A :class:`bool`.
/// .. autoattribute:: problem
///
///     A :class:`str`.
/// .. autoattribute:: original
///
///     A :class:`str`.
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

// {{{ DialectTypes

/// .. attribute:: DISABLE
/// .. attribute:: PARSE_ONLY
/// .. attribute:: ENABLE
#[pyclass]
#[derive(Clone)]
struct DialectTypes(starlark::syntax::DialectTypes);

#[pymethods]
impl DialectTypes {
    #[classattr]
    #[allow(non_snake_case)]
    fn DISABLE() -> Self {
        DialectTypes(starlark::syntax::DialectTypes::Disable)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn PARSE_ONLY() -> Self {
        DialectTypes(starlark::syntax::DialectTypes::ParseOnly)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn ENABLE() -> Self {
        DialectTypes(starlark::syntax::DialectTypes::Enable)
    }
}

// }}}

// {{{ Dialect

/// .. automethod:: standard
/// .. automethod:: extended
/// .. autoattribute:: enable_def
///
///     A :class:`bool`.
/// .. autoattribute:: enable_lambda
///
///     A :class:`bool`.
/// .. autoattribute:: enable_keyword_only_arguments
///
///     A :class:`bool`.
/// .. autoattribute:: enable_types
///
///     A value of type :class:`DialectTypes`.
/// .. autoattribute:: enable_tabs
///
///     A :class:`bool`.
/// .. autoattribute:: enable_load_reexport
///
///     A :class:`bool`.
/// .. autoattribute:: enable_top_level_stmt
///
///     A :class:`bool`.
///
/// .. note::
///
///     These attributes are only writable (not readable) for the moment.
#[pyclass]
#[derive(Clone)]
struct Dialect(starlark::syntax::Dialect);

#[pymethods]
impl Dialect {
    #[staticmethod]
    #[pyo3(text_signature = "() -> Dialect")]
    fn standard() -> Self {
        Dialect(starlark::syntax::Dialect::Standard)
    }
    #[staticmethod]
    #[pyo3(text_signature = "() -> Dialect")]
    fn extended() -> Self {
        Dialect(starlark::syntax::Dialect::Extended)
    }

    #[setter]
    fn enable_def(&mut self, value: bool) {
        self.0.enable_def = value;
    }
    #[setter]
    fn enable_lambda(&mut self, value: bool) {
        self.0.enable_lambda = value;
    }
    #[setter]
    fn enable_load(&mut self, value: bool) {
        self.0.enable_load = value;
    }
    #[setter]
    fn enable_keyword_only_arguments(&mut self, value: bool) {
        self.0.enable_keyword_only_arguments = value;
    }
    #[setter]
    fn enable_types(&mut self, value: DialectTypes) {
        self.0.enable_types = value.0;
    }
    #[setter]
    fn enable_tabs(&mut self, value: bool) {
        self.0.enable_tabs = value;
    }
    #[setter]
    fn enable_load_reexport(&mut self, value: bool) {
        self.0.enable_load_reexport = value;
    }
    #[setter]
    fn enable_top_level_stmt(&mut self, value: bool) {
        self.0.enable_top_level_stmt = value;
    }
}

// }}}

// {{{ AstModule

/// See :func:`parse` to create objects of this type,
/// and :func:`eval` to evaluate them.
///
/// .. automethod:: lint
#[pyclass]
struct AstModule(starlark::syntax::AstModule);

/// Parse Starlark source code as a string and return an AST.
#[pyfunction]
#[pyo3(
    text_signature = "(filename: str, content: str, dialect: Optional[Dialect] = None) -> AstModule"
)]
fn parse(filename: &str, content: &str, dialect_opt: Option<Dialect>) -> PyResult<AstModule> {
    let dialect = match dialect_opt {
        Some(dialect) => dialect.0,
        None => starlark::syntax::Dialect::Standard,
    };
    Ok(AstModule(convert_err(starlark::syntax::AstModule::parse(
        filename,
        content.to_string(),
        &dialect,
    ))?))
}

/// .. automethod:: lint
#[pymethods]
impl AstModule {
    #[pyo3(text_signature = "(self) -> list[Lint]")]
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

/// .. automethod:: standard
/// .. automethod:: extended
#[pyclass]
struct Globals(starlark::environment::Globals);

#[pymethods]
impl Globals {
    #[staticmethod]
    #[pyo3(text_signature = "() -> Globals")]
    fn standard() -> PyResult<Globals> {
        Ok(Globals(starlark::environment::Globals::standard()))
    }

    #[staticmethod]
    #[pyo3(text_signature = "() -> Globals")]
    fn extended() -> PyResult<Globals> {
        Ok(Globals(starlark::environment::Globals::extended()))
    }
}

// }}}

// {{{ PythonCallableValue

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
struct PythonCallableValue {
    #[allocative(skip)]
    callable: PyObject,
}
starlark_simple_value!(PythonCallableValue);

impl Display for PythonCallableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<python callable>")
    }
}

impl<'v> StarlarkValue<'v> for PythonCallableValue {
    starlark_type!("python_callable_value");

    fn invoke(
        &self,
        _me: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut starlark::eval::Evaluator<'v, '_>,
    ) -> anyhow::Result<Value<'v>> {
        Python::with_gil(|py| -> anyhow::Result<Value<'v>> {
            args.no_named_args()?;
            let py_args: Vec<PyObject> = (args
                .positions(eval.heap())?
                .map(|v| -> PyResult<PyObject> { value_to_pyobject(v) }))
            .try_collect()?;
            convert_to_anyhow(pyobject_to_value(
                self.callable.call1(py, PyTuple::new(py, py_args))?,
                eval.heap(),
            ))
        })
    }
}

// }}}

// {{{ Module

/// .. automethod:: __getitem__
/// .. automethod:: __setitem__
/// .. automethod:: add_callable
/// .. automethod:: freeze
#[pyclass]
#[pyo3(text_signature = "() -> None")]
struct Module(starlark::environment::Module);

#[pymethods]
impl Module {
    #[new]
    fn py_new() -> PyResult<Module> {
        Ok(Module(starlark::environment::Module::new()))
    }

    fn __getitem__(&self, name: &str) -> PyResult<PyObject> {
        Python::with_gil(|py| match self.0.get(name) {
            Some(val) => Ok(value_to_pyobject(val)?),
            None => Ok(py.None()),
        })
    }

    fn __setitem__(&self, name: &str, obj: PyObject) -> PyResult<()> {
        self.0.set(name, pyobject_to_value(obj, self.0.heap())?);
        Ok(())
    }

    #[pyo3(text_signature = "(self, name: str, callable: Callable) -> None")]
    fn add_callable(&self, name: &str, callable: PyObject) {
        let b = self
            .0
            .heap()
            .alloc(PythonCallableValue { callable: callable });
        self.0.set(name, b);
    }

    #[pyo3(text_signature = "(self) -> FrozenModule")]
    fn freeze(mod_cell: &PyCell<Module>) -> PyResult<FrozenModule> {
        let module = mod_cell
            .replace(Module(starlark::environment::Module::new()))
            .0;
        Ok(FrozenModule(convert_err(module.freeze())?))
    }
}

// }}}

// {{{ FrozenModule

#[pyclass]
struct FrozenModule(starlark::environment::FrozenModule);

// }}}

// {{{ FileLoader

#[pyclass]
#[pyo3(text_signature = "(load_func: Callable[[str], FrozenModule]) -> None")]
struct FileLoader {
    callable: PyObject,
}

#[pymethods]
impl FileLoader {
    #[new]
    fn py_new(callable: PyObject) -> FileLoader {
        FileLoader { callable: callable }
    }
}

impl starlark::eval::FileLoader for FileLoader {
    fn load(&self, path: &str) -> anyhow::Result<starlark::environment::FrozenModule> {
        Python::with_gil(
            |py| -> anyhow::Result<starlark::environment::FrozenModule> {
                let fmod: Py<FrozenModule> =
                    self.callable.call1(py, (path.to_string(),))?.extract(py)?;
                // FIXME: Can this be done without cloning the module?
                let fmod_clone = fmod.borrow(py).0.clone();
                Ok(fmod_clone)
            },
        )
    }
}

// }}}

// {{{ eval

fn empty_ast() -> AstModule {
    AstModule(
        starlark::syntax::AstModule::parse(
            "<empty>",
            "".to_string(),
            &starlark::syntax::Dialect::Standard,
        )
        .unwrap(),
    )
}

/// Note that this *consumes* the *ast* argument, which is unusable after
/// being passed to this fucntion.
///
/// :returns: the value returned by the evaluation, after :ref:`object-conversion`.
#[pyfunction]
#[pyo3(
    text_signature = "(module: Module, ast: AstModule, globals: Globals, file_loader: Optional[FileLoader]) -> object"
)]
fn eval(
    module: &mut Module,
    ast: &PyCell<AstModule>,
    globals: &Globals,
    file_loader: Option<&PyCell<FileLoader>>,
) -> PyResult<PyObject> {
    let mut evaluator = starlark::eval::Evaluator::new(&module.0);

    let tail = |evaluator: &mut starlark::eval::Evaluator| {
        // Stupid: eval_module consumes the AST.
        // Python would like it to live on,  but starlark-rust says no.
        value_to_pyobject(convert_err(
            evaluator.eval_module(ast.replace(empty_ast()).0, &globals.0),
        )?)
    };

    match file_loader {
        Some(loader_cell) => {
            let loader_ref = loader_cell.borrow();
            evaluator.set_loader(&*loader_ref);
            tail(&mut evaluator)
        }
        None => tail(&mut evaluator),
    }
}

// }}}

#[pymodule]
#[pyo3(name = "starlark")]
fn starlark_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<ResolvedSpan>()?;
    m.add_class::<ResolvedFileSpan>()?;
    m.add_class::<Lint>()?;
    m.add_class::<DialectTypes>()?;
    m.add_class::<Dialect>()?;
    m.add_class::<AstModule>()?;
    m.add_class::<Globals>()?;
    m.add_class::<Module>()?;
    m.add_class::<FrozenModule>()?;
    m.add_class::<FileLoader>()?;
    m.add_wrapped(wrap_pyfunction!(parse))?;
    m.add_wrapped(wrap_pyfunction!(eval))?;
    m.add("StarlarkError", _py.get_type::<StarlarkError>())?;

    Ok(())
}

// vim: foldmethod=marker
