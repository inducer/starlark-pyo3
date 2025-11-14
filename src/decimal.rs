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

//! Decimal type support for Starlark
//!
//! Provides first-class Decimal support via the rust_decimal crate,
//! enabling precise decimal arithmetic without float precision loss.
//! Exposes a `RustDecimal()` constructor to Starlark for creating decimal values.

use std::cmp::Ordering;
use std::fmt::{self, Display};
use std::hash::Hash;
use std::str::FromStr;

use allocative::Allocative;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rust_decimal::Decimal;
use starlark::collections::StarlarkHasher;
use starlark::environment::GlobalsBuilder;
use starlark::starlark_simple_value;
use starlark::values::{
    Heap, NoSerialize, ProvidesStaticType, StarlarkValue, Value, ValueError, ValueLike,
};
use starlark_derive::{starlark_module, starlark_value};

#[derive(Debug, Clone, ProvidesStaticType, NoSerialize, Allocative)]
pub struct DecimalValue {
    #[allocative(skip)]
    pub(crate) value: Decimal,
}
starlark_simple_value!(DecimalValue);

impl Display for DecimalValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

fn decimal_operation_error(op: &str, other_type: &str) -> starlark::Error {
    ValueError::OperationNotSupportedBinary {
        op: op.to_owned(),
        left: "rust_decimal".to_owned(),
        right: other_type.to_owned(),
    }
    .into()
}

fn decimal_constructor_error(arg_type: &str) -> starlark::Error {
    ValueError::OperationNotSupported {
        op: "RustDecimal()".to_owned(),
        typ: arg_type.to_owned(),
    }
    .into()
}

// Helper to convert Starlark values to Decimal, with custom error handling
fn try_decimal_from_value<'v, F>(
    value: Value<'v>,
    make_error: F,
) -> starlark::Result<Decimal>
where
    F: Fn(&str) -> starlark::Error,
{
    if let Some(existing) = value.downcast_ref::<DecimalValue>() {
        return Ok(existing.value);
    }

    let ty = value.get_type();
    if ty == "int" {
        // Use JSON rendering to avoid repr quirks and get an exact, base-10 string.
        let json_repr = value.to_json().map_err(|_| make_error(ty))?;
        Decimal::from_str(&json_repr).map_err(|_| make_error(ty))
    } else {
        // Intentionally do not coerce floats to Decimal to avoid silently embedding
        // binary floating-point artifacts; require explicit RustDecimal("...") instead.
        Err(make_error(ty))
    }
}

fn decimal_from_value<'v>(value: Value<'v>, op: &str) -> starlark::Result<Decimal> {
    try_decimal_from_value(value, |ty| decimal_operation_error(op, ty))
}

fn decimal_from_constructor<'v>(value: Value<'v>) -> starlark::Result<Decimal> {
    // Strings are only valid in the constructor, not in operations,
    // so handle them separately before calling the shared helper
    if let Some(as_str) = value.unpack_str() {
        return Decimal::from_str(as_str).map_err(|_| decimal_constructor_error("string"));
    }
    // Use common helper for DecimalValue and int handling
    try_decimal_from_value(value, decimal_constructor_error)
}

pub fn alloc_decimal<'v>(heap: &'v Heap, decimal: Decimal) -> Value<'v> {
    heap.alloc(DecimalValue { value: decimal })
}

#[starlark_module]
pub fn decimal_module(builder: &mut GlobalsBuilder) {
    /// Construct a rust_decimal value from a string, int, or existing RustDecimal.
    fn RustDecimal<'v>(#[starlark(require = pos)] value: Value<'v>, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        let decimal = decimal_from_constructor(value)?;
        Ok(alloc_decimal(heap, decimal))
    }
}

#[starlark_value(type = "rust_decimal")]
impl<'v> StarlarkValue<'v> for DecimalValue {
    fn equals(&self, other: Value<'v>) -> starlark::Result<bool> {
        match decimal_from_value(other, "==") {
            Ok(rhs) => Ok(self.value == rhs),
            Err(_) => Ok(false),
        }
    }

    fn compare(&self, other: Value<'v>) -> starlark::Result<Ordering> {
        let rhs = decimal_from_value(other, "compare")?;
        Ok(self.value.cmp(&rhs))
    }

    fn to_bool(&self) -> bool {
        !self.value.is_zero()
    }

    fn plus(&self, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        Ok(alloc_decimal(heap, self.value))
    }

    fn minus(&self, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        Ok(alloc_decimal(heap, -self.value))
    }

    fn add(&self, rhs: Value<'v>, heap: &'v Heap) -> Option<starlark::Result<Value<'v>>> {
        Some(decimal_from_value(rhs, "+").map(|rhs| {
            alloc_decimal(heap, self.value + rhs)
        }))
    }

    fn radd(&self, lhs: Value<'v>, heap: &'v Heap) -> Option<starlark::Result<Value<'v>>> {
        Some(decimal_from_value(lhs, "+").map(|lhs| {
            alloc_decimal(heap, lhs + self.value)
        }))
    }

    fn sub(&self, rhs: Value<'v>, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        let rhs = decimal_from_value(rhs, "-")?;
        Ok(alloc_decimal(heap, self.value - rhs))
    }

    fn mul(&self, rhs: Value<'v>, heap: &'v Heap) -> Option<starlark::Result<Value<'v>>> {
        Some(decimal_from_value(rhs, "*").map(|rhs| {
            alloc_decimal(heap, self.value * rhs)
        }))
    }

    fn rmul(&self, lhs: Value<'v>, heap: &'v Heap) -> Option<starlark::Result<Value<'v>>> {
        Some(decimal_from_value(lhs, "*").map(|lhs| {
            alloc_decimal(heap, lhs * self.value)
        }))
    }

    fn div(&self, rhs: Value<'v>, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        let rhs = decimal_from_value(rhs, "/")?;
        if rhs.is_zero() {
            return Err(ValueError::DivisionByZero.into());
        }
        Ok(alloc_decimal(heap, self.value / rhs))
    }

    fn floor_div(&self, rhs: Value<'v>, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        let rhs = decimal_from_value(rhs, "//")?;
        if rhs.is_zero() {
            return Err(ValueError::DivisionByZero.into());
        }
        let division = self.value / rhs;
        Ok(alloc_decimal(heap, division.floor()))
    }

    fn percent(&self, rhs: Value<'v>, heap: &'v Heap) -> starlark::Result<Value<'v>> {
        let rhs = decimal_from_value(rhs, "%")?;
        if rhs.is_zero() {
            return Err(ValueError::DivisionByZero.into());
        }
        Ok(alloc_decimal(heap, self.value % rhs))
    }

    fn write_hash(&self, hasher: &mut StarlarkHasher) -> starlark::Result<()> {
        self.value.hash(hasher);
        Ok(())
    }
}

/// Convert DecimalValue to Python Decimal object
pub fn decimal_to_python(decimal_value: &DecimalValue) -> PyResult<Py<PyAny>> {
    let decimal_str = decimal_value.value.to_string();
    Python::attach(|py| {
        let decimal_module = py.import("decimal")?;
        let decimal_type = decimal_module.getattr("Decimal")?;
        let py_decimal = decimal_type.call1((decimal_str,))?;
        Ok(py_decimal.into())
    })
}

/// Try to convert Python object to DecimalValue if it's a Python Decimal
pub fn python_to_decimal<'v>(obj: &Bound<PyAny>, heap: &'v Heap) -> PyResult<Option<Value<'v>>> {
    if let Ok(class) = obj.getattr("__class__") {
        let module_name = match class.getattr("__module__") {
            Ok(module) => module.extract::<String>().ok(),
            Err(_) => None,
        };
        let type_name = match class.getattr("__name__") {
            Ok(name) => name.extract::<String>().ok(),
            Err(_) => None,
        };
        if module_name.as_deref() == Some("decimal") && type_name.as_deref() == Some("Decimal") {
            let value_str: String = obj.str()?.extract()?;
            let decimal =
                Decimal::from_str(&value_str).map_err(|e| PyValueError::new_err(e.to_string()))?;
            return Ok(Some(alloc_decimal(heap, decimal)));
        }
    }
    Ok(None)
}
