Installation
============

To use this, make sure you have nightly rust available::

    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    rustup default nightly

After that, this should do the trick::

    pip install starlark-pyo3

For a development install, you may use this incantation::

    pip install maturin
    maturin develop

User-visible Changes
====================

Version 2022.1
--------------

- Initial release

License
=======

.. literalinclude:: ../LICENSE
