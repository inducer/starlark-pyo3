Starlark-PyO3: Python bindings for starlark-rust
================================================

.. image:: https://github.com/inducer/starlark-pyo3/workflows/CI/badge.svg?branch=main&event=push
    :alt: Github Build Status
    :target: https://github.com/inducer/starlark-pyo3/actions?query=branch%3Amain+workflow%3ACI+event%3Apush
.. image:: https://badge.fury.io/py/starlark-pyo3.png
    :alt: Python Package Index Release Page
    :target: https://pypi.org/project/starlark-pyo3/

This exposes the
`starlark-rust <https://github.com/facebookexperimental/starlark-rust/>`__
interpreter for the
`Starlark <https://github.com/bazelbuild/starlark/blob/master/spec.md>`__
Python-like language to Python via `PyO3 <https://pyo3.rs>`__.

*Status:* This is reasonably complete and usable.

Links
-----

-  `Documentation <https://documen.tician.de/starlark-pyo3/>`__
-  `Github <https://github.com/inducer/starlark-pyo3>`__ (issues etc.)
-  `Package index <https://pypi.org/project/starlark-pyo3>`__

Installation 
------------
To install, say::

    pip install starlark-pyo3

Binary wheels are available for all major platforms.  The module is importable
as ``starlark``.

Installation for Development
----------------------------

To use this, make sure you have nightly rust available::

    curl –proto ‘=https’ –tlsv1.2 -sSf https://sh.rustup.rs \| sh rustup
    default nightly

Then, to install into the current Python virtual environment::

    pip install maturin
    maturin develop
