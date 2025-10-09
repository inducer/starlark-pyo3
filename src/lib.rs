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

extern crate allocative;
extern crate anyhow;
extern crate dupe;
extern crate gazebo;
extern crate pyo3;
extern crate serde_json;
extern crate starlark;
extern crate starlark_derive;
extern crate thiserror;

use std::collections::HashMap;
use std::fmt::{self, Display};
use std::sync::Mutex;

use crate::pyo3::create_exception;
use crate::pyo3::exceptions::PyException;
use crate::pyo3::prelude::*;

use gazebo::prelude::*;

use crate::starlark::collections::SmallMap;
use crate::starlark::typing::AstModuleTypecheck;
use allocative::Allocative;
use dupe::Dupe;
use pyo3::sync::MutexExt;
use pyo3::types::{PyDict, PyTuple};
use starlark::analysis::AstModuleLint;
use starlark::eval::Arguments;
use starlark::starlark_simple_value;
use starlark::values::dict::Dict;
use starlark::values::list::AllocList;
use starlark::values::{FreezeResult, Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value};
use starlark_derive::starlark_value;
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

fn serde_to_starlark(x: serde_json::Value, heap: &Heap) -> anyhow::Result<Value<'_>> {
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

fn value_to_pyobject(value: Value) -> PyResult<Py<PyAny>> {
    let json_val = convert_anyhow_err(value.to_json())?;
    Python::attach(|py| {
        let json = py.import("json")?;
        json.getattr("loads")?.call1((json_val,))?.extract()
    })
}

fn pyobject_to_value<'v>(obj: Py<PyAny>, heap: &'v Heap) -> PyResult<Value<'v>> {
    Python::attach(|py| -> PyResult<Value<'v>> {
        let json = py.import("json")?;
        let json_str: String = json.getattr("dumps")?.call1((obj,))?.extract()?;
        convert_anyhow_err(serde_to_starlark(
            convert_serde_err(serde_json::from_str(&json_str))?,
            heap,
        ))
    })
}

// }}}

// {{{ result conversions

fn convert_anyhow_err<T>(err: Result<T, anyhow::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

fn convert_starlark_err<T>(err: starlark::Result<T>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(e.to_string())),
    }
}

fn convert_freeze_err<T>(err: FreezeResult<T>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        // FIXME: Could also format the additional contexts provided.
        Err(e) => Err(StarlarkError::new_err(e.err_msg)),
    }
}

fn convert_serde_err<T>(err: Result<T, serde_json::Error>) -> Result<T, PyErr> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(StarlarkError::new_err(format!("{}", e))),
    }
}

fn convert_to_starlark_err<T>(err: Result<T, PyErr>) -> Result<T, starlark::Error> {
    match err {
        Ok(t) => Ok(t),
        Err(e) => Err(starlark::Error::new_other(e)),
    }
}

// }}}

// {{{ ResolvedPos

/// .. autoattribute:: line
///
///     A :class:`int`.
/// .. autoattribute:: column
///
///     A :class:`int`.
#[pyclass]
struct ResolvedPos(starlark::codemap::ResolvedPos);

#[pymethods]
impl ResolvedPos {
    #[getter]
    fn line(&self) -> usize {
        self.0.line
    }
    #[getter]
    fn column(&self) -> usize {
        self.0.column
    }
}

// }}}

// {{{ ResolvedSpan

/// .. autoattribute:: begin
///
///     A :class:`ResolvedPos`.
/// .. autoattribute:: end
///
///     A :class:`ResolvedPos`.
#[pyclass]
struct ResolvedSpan(starlark::codemap::ResolvedSpan);

#[pymethods]
impl ResolvedSpan {
    #[getter]
    fn begin(&self) -> ResolvedPos {
        ResolvedPos(self.0.begin)
    }
    #[getter]
    fn end(&self) -> ResolvedPos {
        ResolvedPos(self.0.end)
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
/// .. automethod:: __str__
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
    fn __str__(&self) -> String {
        format!("{}", self.0)
    }
}

// }}}

// {{{ EvalSeverity

/// .. attribute:: Error
/// .. attribute:: Warning
/// .. attribute:: Advice
/// .. attribute:: Disabled
#[pyclass]
#[derive(Clone)]
struct EvalSeverity(starlark::analysis::EvalSeverity);

#[pymethods]
impl EvalSeverity {
    fn __repr__(&self) -> String {
        match self.0 {
            starlark::analysis::EvalSeverity::Error => "Error".to_string(),
            starlark::analysis::EvalSeverity::Warning => "Warning".to_string(),
            starlark::analysis::EvalSeverity::Advice => "Advice".to_string(),
            starlark::analysis::EvalSeverity::Disabled => "Disabled".to_string(),
        }
    }
    fn __str__(&self) -> String {
        self.__repr__()
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Error() -> Self {
        EvalSeverity(starlark::analysis::EvalSeverity::Error)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Warning() -> Self {
        EvalSeverity(starlark::analysis::EvalSeverity::Warning)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Advice() -> Self {
        EvalSeverity(starlark::analysis::EvalSeverity::Advice)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Disabled() -> Self {
        EvalSeverity(starlark::analysis::EvalSeverity::Disabled)
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
/// .. autoattribute:: severity
///
///     A :class:`EvalSeverity`.
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
    pub severity: starlark::analysis::EvalSeverity,
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
    fn severity(&self) -> EvalSeverity {
        EvalSeverity(self.severity)
    }
    #[getter]
    fn resolved_location(&self) -> ResolvedFileSpan {
        ResolvedFileSpan(self.location.resolve())
    }
    fn __str__(&self) -> String {
        self.to_string()
    }
}

// }}}

// {{{ Error

/// .. attribute:: span: ResolvedFileSpan | None
/// .. automethod:: __str__
#[pyclass]
struct Error(starlark::Error);

#[pymethods]
impl Error {
    #[getter]
    fn span(&self) -> Option<ResolvedFileSpan> {
        match self.0.span() {
            Some(span) => Some(ResolvedFileSpan(span.resolve())),
            None => None,
        }
    }
    fn __str__(&self) -> String {
        self.0.to_string()
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
/// .. autoattribute:: enable_load
///
///     A :class:`bool`.
/// .. autoattribute:: enable_keyword_only_arguments
///
///     A :class:`bool`.
/// .. autoattribute:: enable_types
///
///     A value of type :class:`DialectTypes`.
/// .. autoattribute:: enable_load_reexport
///
///     A :class:`bool`.
/// .. autoattribute:: enable_top_level_stmt
///
///     A :class:`bool`.
/// .. autoattribute:: enable_f_strings
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
    fn enable_load_reexport(&mut self, value: bool) {
        self.0.enable_load_reexport = value;
    }
    #[setter]
    fn enable_top_level_stmt(&mut self, value: bool) {
        self.0.enable_top_level_stmt = value;
    }
    #[setter]
    fn enable_f_strings(&mut self, value: bool) {
        self.0.enable_f_strings = value;
    }
}

// }}}

// {{{ Interface

/// Opaque for now.
#[pyclass(frozen)]
struct Interface(starlark::typing::Interface);

#[pymethods]
impl Interface {}

// }}}

// {{{ AstLoad

/// .. attribute:: module_id
/// .. attribute:: symbols

#[pyclass]
struct AstLoad {
    #[pyo3(get)]
    pub module_id: String,
    #[pyo3(get)]
    pub symbols: HashMap<String, String>,
}

// }}}

// {{{ AstModule

/// See :func:`parse` to create objects of this type,
/// and :func:`eval` to evaluate them.
///
/// .. automethod:: lint
/// .. automethod:: loads
/// .. automethod:: typecheck
#[pyclass]
struct AstModule(starlark::syntax::AstModule);

/// Parse Starlark source code as a string and return an AST.
#[pyfunction]
#[pyo3(
    signature = (filename, content, dialect=None),
    text_signature = "(filename: str, content: str, dialect: Dialect | None = None) -> AstModule"
)]
fn parse(filename: &str, content: &str, dialect: Option<Dialect>) -> PyResult<AstModule> {
    let dialect = match dialect {
        Some(dialect) => dialect.0,
        None => starlark::syntax::Dialect::Standard,
    };
    Ok(AstModule(convert_starlark_err(
        starlark::syntax::AstModule::parse(filename, content.to_string(), &dialect),
    )?))
}

/// .. automethod:: lint
#[pymethods]
impl AstModule {
    #[pyo3(text_signature = "() -> list[Lint]")]
    fn lint(&self) -> Vec<Lint> {
        self.0.lint(None).map(|lint| Lint {
            location: lint.location.dupe(),
            short_name: lint.short_name.clone(),
            severity: lint.severity,
            problem: lint.problem.clone(),
            original: lint.original.clone(),
        })
    }

    #[pyo3(text_signature = "() -> list[AstLoad]")]
    fn loads(&self) -> Vec<AstLoad> {
        self.0
            .loads()
            .iter()
            .map(|ld| -> AstLoad {
                AstLoad {
                    module_id: ld.module_id.to_string(),
                    symbols: ld
                        .symbols
                        .iter()
                        .map(|(imp_as, name)| (imp_as.to_string(), name.to_string()))
                        .collect(),
                }
            })
            .collect()
    }

    #[pyo3(
        text_signature = "(globals: Globals, loads: dict[str, Interface]) -> tuple[list[Error], None, None]"
    )]
    fn typecheck<'py>(
        slf: Bound<'py, AstModule>,
        globals: &Globals,
        py_loads: HashMap<String, Bound<'py, Interface>>,
    ) -> (Vec<Error>, Interface, ()) {
        // FIXME: Can we get by without cloning all the interfaces?
        let loads: HashMap<String, starlark::typing::Interface> = py_loads
            .iter()
            .map(|(name, iface)| (name.clone(), iface.get().0.clone()))
            .collect();
        // FIXME: Can we make do without cloning the module?
        let (mut errors, _typemap, iface, _approximations) =
            slf.borrow().0.clone().typecheck(&globals.0, &loads);
        (
            errors.drain(..).map(|err| Error(err)).collect(),
            Interface(iface),
            (),
        )
    }
}

// }}}

// {{{ LibraryExtension

/// .. attribute:: StructType
/// .. attribute:: RecordType
/// .. attribute:: EnumType
/// .. attribute:: Map
/// .. attribute:: Filter
/// .. attribute:: Partial
/// .. attribute:: ExperimentalRegex
/// .. attribute:: Debug
/// .. attribute:: Print
/// .. attribute:: Pprint
/// .. attribute:: Breakpoint
/// .. attribute:: Json
/// .. attribute:: Typing
/// .. attribute:: Internal
/// .. attribute:: CallStack
#[pyclass]
#[derive(Clone)]
struct LibraryExtension(starlark::environment::LibraryExtension);

#[pymethods]
impl LibraryExtension {
    #[classattr]
    #[allow(non_snake_case)]
    fn StructType() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::StructType)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn RecordType() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::RecordType)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn EnumType() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::EnumType)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Map() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Map)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Filter() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Filter)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Partial() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Partial)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Debug() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Debug)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Print() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Print)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Pprint() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Pprint)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Breakpoint() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Breakpoint)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Json() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Json)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Typing() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Typing)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn Internal() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::Internal)
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn CallStack() -> Self {
        LibraryExtension(starlark::environment::LibraryExtension::CallStack)
    }
}

// }}}

// {{{ Globals

/// .. automethod:: standard
/// .. automethod:: extended_by
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
    #[pyo3(text_signature = "(extensions: list[LibraryExtension]) -> Globals")]
    fn extended_by(extensions: Vec<LibraryExtension>) -> PyResult<Globals> {
        let exts: Vec<starlark::environment::LibraryExtension> =
            extensions.iter().map(|ext| ext.0).collect();

        Ok(Globals(starlark::environment::Globals::extended_by(&exts)))
    }
}

// }}}

// {{{ PythonCallableValue

#[derive(Debug, ProvidesStaticType, NoSerialize, Allocative)]
struct PythonCallableValue {
    #[allocative(skip)]
    callable: Py<PyAny>,
}
starlark_simple_value!(PythonCallableValue);

impl Display for PythonCallableValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<python callable>")
    }
}

#[starlark_value(type = "python_callable_value")]
impl<'v> StarlarkValue<'v> for PythonCallableValue {
    fn invoke(
        &self,
        _me: Value<'v>,
        args: &Arguments<'v, '_>,
        eval: &mut starlark::eval::Evaluator<'v, '_, '_>,
    ) -> starlark::Result<Value<'v>> {
        Python::attach(|py| -> starlark::Result<Value<'v>> {
            // Handle positional arguments
            let py_args: Vec<Py<PyAny>> = convert_to_starlark_err(
                (args
                    .positions(eval.heap())?
                    .map(|v| -> PyResult<Py<PyAny>> { value_to_pyobject(v) }))
                .collect::<PyResult<Vec<Py<PyAny>>>>(),
            )?;

            // Handle named arguments.
            let py_kwargs = PyDict::new(py);
            for name in args.names_map()?.iter() {
                let key = name.0.as_str();
                let val = convert_to_starlark_err(value_to_pyobject(*name.1))?;
                convert_to_starlark_err(py_kwargs.set_item(key, val))?;
            }

            convert_to_starlark_err(pyobject_to_value(
                convert_to_starlark_err(self.callable.call(
                    py,
                    convert_to_starlark_err(PyTuple::new(py, py_args))?,
                    Some(&py_kwargs),
                ))?,
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
struct Module(Mutex<starlark::environment::Module>);

// Rust infers that Module is not Send because Module contains 'extra_value',
// which is a Value, and this change prevents Values from ever being Send:
// https://github.com/facebook/starlark-rust/commit/6af7319980c5f4227b5642a49c4c520e96b5c06a
// In our case, extra_value isn't used, so here we go pretending to know better.
// It will probably be useful to disable this on every update to starlark to
// see if there are other reasons for Module to not be Send.
unsafe impl Send for Module {}
unsafe impl Sync for Module {}

#[pymethods]
impl Module {
    #[new]
    #[pyo3(text_signature = "() -> None")]
    fn py_new() -> PyResult<Module> {
        Ok(Module(Mutex::new(starlark::environment::Module::new())))
    }

    fn __getitem__(slf: &Bound<Self>, name: &str) -> PyResult<Py<PyAny>> {
        Python::attach(|py| match slf.borrow().0.lock().unwrap().get(name) {
            Some(val) => Ok(value_to_pyobject(val)?),
            None => Ok(py.None()),
        })
    }

    fn __setitem__(slf: &Bound<Self>, name: &str, obj: Py<PyAny>) -> PyResult<()> {
        let self_ref = slf.borrow();
        let self_locked = self_ref.0.lock().unwrap();
        self_locked.set(name, pyobject_to_value(obj, self_locked.heap())?);
        Ok(())
    }

    #[pyo3(text_signature = "(name: str, callable: Callable) -> None")]
    fn add_callable(slf: &Bound<Self>, name: &str, callable: Py<PyAny>) {
        let self_ref = slf.borrow();
        let self_locked = self_ref.0.lock().unwrap();
        let b = self_locked.heap().alloc(PythonCallableValue { callable });
        self_locked.set(name, b);
    }

    #[pyo3(text_signature = "() -> FrozenModule")]
    fn freeze(slf: &Bound<Self>) -> PyResult<FrozenModule> {
        let self_ref = slf.borrow_mut();
        let mut self_locked = self_ref.0.lock().unwrap();
        let module = std::mem::replace(&mut *self_locked, starlark::environment::Module::new());
        Ok(FrozenModule(convert_freeze_err(module.freeze())?))
    }
}

// }}}

// {{{ FrozenModule

#[pyclass]
struct FrozenModule(starlark::environment::FrozenModule);

// }}}

// {{{ FileLoader

#[pyclass]
struct FileLoader {
    callable: Py<PyAny>,
}

#[pymethods]
impl FileLoader {
    #[new]
    #[pyo3(text_signature = "(load_func: Callable[[str], FrozenModule]) -> None")]
    fn py_new(callable: Py<PyAny>) -> FileLoader {
        FileLoader { callable }
    }
}

impl starlark::eval::FileLoader for FileLoader {
    fn load(&self, path: &str) -> starlark::Result<starlark::environment::FrozenModule> {
        Python::attach(
            |py| -> starlark::Result<starlark::environment::FrozenModule> {
                let fmod: Py<FrozenModule> = convert_to_starlark_err(
                    convert_to_starlark_err(self.callable.call1(py, (path.to_string(),)))?
                        .extract(py),
                )?;
                // FIXME: Can this be done without cloning the module?
                let fmod_clone = fmod.borrow(py).0.clone();
                Ok(fmod_clone)
            },
        )
    }
}

// }}}

// {{{ eval

/// :returns: the value returned by the evaluation, after :ref:`object-conversion`.
#[pyfunction]
#[pyo3(
    signature = (module, ast, globals, file_loader=None),
    text_signature = "(module: Module, ast: AstModule, globals: Globals, file_loader: FileLoader | None = None) -> object"
)]
fn eval(
    module: &mut Module,
    ast: &Bound<AstModule>,
    globals: &Globals,
    file_loader: Option<&Bound<FileLoader>>,
) -> PyResult<Py<PyAny>> {
    let tail = |evaluator: &mut starlark::eval::Evaluator| {
        // Stupid: eval_module consumes the AST. Clone it.
        value_to_pyobject(convert_starlark_err(
            evaluator.eval_module(ast.borrow().0.clone(), &globals.0),
        )?)
    };

    let mod_locked = module.0.lock_py_attached(ast.py()).unwrap();
    match file_loader {
        Some(loader_cell) => {
            let loader_ref = loader_cell.borrow();
            let mut evaluator = starlark::eval::Evaluator::new(&mod_locked);
            evaluator.set_loader(&*loader_ref);
            tail(&mut evaluator)
        }
        None => {
            let mut evaluator = starlark::eval::Evaluator::new(&mod_locked);
            tail(&mut evaluator)
        }
    }
}

// }}}

#[pymodule]
#[pyo3(name = "starlark")]
fn starlark_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ResolvedPos>()?;
    m.add_class::<ResolvedSpan>()?;
    m.add_class::<ResolvedFileSpan>()?;
    m.add_class::<EvalSeverity>()?;
    m.add_class::<Lint>()?;
    m.add_class::<Error>()?;
    m.add_class::<DialectTypes>()?;
    m.add_class::<Dialect>()?;
    m.add_class::<Interface>()?;
    m.add_class::<AstLoad>()?;
    m.add_class::<AstModule>()?;
    m.add_class::<LibraryExtension>()?;
    m.add_class::<Globals>()?;
    m.add_class::<Module>()?;
    m.add_class::<FrozenModule>()?;
    m.add_class::<FileLoader>()?;
    m.add_wrapped(wrap_pyfunction!(parse))?;
    m.add_wrapped(wrap_pyfunction!(eval))?;
    m.add("StarlarkError", m.py().get_type::<StarlarkError>())?;

    Ok(())
}

// vim: foldmethod=marker
