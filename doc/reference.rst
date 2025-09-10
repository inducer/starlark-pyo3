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
