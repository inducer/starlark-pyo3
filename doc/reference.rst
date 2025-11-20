Reference
=========

.. module:: starlark

.. _object-conversion:

Object conversion
-----------------

To convert values between starlark and Python, JSON is currently being used
as an intermediate format, which defines the scope of what is convertible.
This, however, is subject to change.

References to Source Locations
------------------------------

.. autoclass:: ResolvedFileSpan
.. autoclass:: ResolvedPos
.. autoclass:: ResolvedSpan

Diagnostics
-----------

.. autoexception:: StarlarkError
.. autoclass:: EvalSeverity
.. autoclass:: Lint
.. autoclass:: Error

Dialect
-------

.. autoclass:: DialectTypes
.. autoclass:: Dialect

Type checking
-------------

.. autoclass:: Interface

AST
---

.. autoclass:: AstLoad
.. autoclass:: AstModule


Values
------

.. autoclass:: OpaquePythonObject
.. autoclass:: ToRecord

Decimal
^^^^^^^

This package preserves Python :class:`~decimal.Decimal` values without precision loss.
Decimals passed from Python stay as precise decimal values in Starlark and
round-trip back to Python as ``Decimal`` objects.

.. doctest::

    >>> import decimal
    >>> import starlark as sl

    >>> glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    >>> mod = sl.Module()

    >>> # Pass Python decimals to Starlark
    >>> mod["amount"] = decimal.Decimal("100.25")

    >>> program = """
    ... # Create decimals in Starlark with RustDecimal()
    ... result = amount * 2 + RustDecimal('0.75')
    ... # Control precision with scale() and round_dp()
    ... pi = RustDecimal("3.14159")
    ... pi.scale()        # Returns 5 (number of decimal places)
    ... pi.round_dp(2)    # Returns RustDecimal("3.14")
    ... result
    ... """

    >>> ast = sl.parse("prog.star", program)
    >>> val = sl.eval(mod, ast, glb)
    >>> assert val == decimal.Decimal("201.25")

Implementation notes:

- Starlark ``RustDecimal`` operations use `rust_decimal <rust_decimal>`__
  semantics (28 decimal places maximum, Banker's rounding)
- Python :class:`~decimal.Decimal` operations use Python semantics (configurable via context)
- Conversion preserves exact values without precision loss
- Python's ``decimal.getcontext()`` is not consulted during conversion
- Use ``round_dp(n)`` to explicitly control decimal places before or after conversion

Globals
-------

.. autoclass:: LibraryExtension
.. autoclass:: Globals

Modules
-------

.. autoclass:: Module
.. autoclass:: FrozenModule


Loaders
-------

.. autoclass:: FileLoader

Parsing and Evaluation
----------------------

.. autofunction:: parse
.. autofunction:: eval
