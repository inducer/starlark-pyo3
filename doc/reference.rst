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
.. autoclass:: ResolvedSpan

Diagnostics
-----------

.. autoexception:: StarlarkError
.. autoclass:: Lint

Dialect
-------

.. autoclass:: DialectTypes
.. autoclass:: Dialect

AST
---

.. autoclass:: AstModule

Globals
-------

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
