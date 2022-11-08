Welcome to starlark-pyo3's documentation!
=========================================

This package provides a sandboxed/restricted Python-like environment 
by exposing the
`starlark-rust <https://github.com/facebookexperimental/starlark-rust/>`__
interpreter for the
`Starlark <https://github.com/bazelbuild/starlark/blob/master/spec.md>`__
Python-like language to Python via `PyO3 <https://pyo3.rs>`__.

`Starlark <https://github.com/bazelbuild/starlark>`__ claims the following
*design principles*:

-   **Deterministic evaluation**. Executing the same code twice will give the
    same results.
-   **Hermetic execution**. Execution cannot access the file system, network,
    system clock. It is safe to execute untrusted code.
-   **Parallel evaluation**. Modules can be loaded in parallel. To guarantee a
    thread-safe execution, shared data becomes immutable.
-   **Simplicity**. We try to limit the number of concepts needed to understand
    the code. Users should be able to quickly read and write code, even if they
    are not expert. The language should avoid pitfalls as much as possible.
-   **Focus on tooling**. We recognize that the source code will be read,
    analyzed, modified, by both humans and tools.
-   **Python-like**. Python is a widely used language. Keeping the language
    similar to Python can reduce the learning curve and make the semantics more
    obvious to users.

Here's an example:

.. literalinclude:: ../examples/demo.py

Contents
--------
.. toctree::
    :maxdepth: 2

    reference
    misc

Indices and tables
------------------

* :ref:`genindex`
* :ref:`modindex`
* :ref:`search`
