# Starlark-PyO3: Python bindings for starlark-rust

This exposes the
[starlark-rust](https://github.com/facebookexperimental/starlark-rust/)
interpreter for the
[Starlark](https://github.com/bazelbuild/starlark/blob/master/spec.md)
Python-like language to Python via [PyO3](https://pyo3.rs).

To use this, make sure you have nightly rust available:
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
```

Then, to install into the current Python virtual environment:
```
pip install maturin
maturin develop
```

- [Documentation](https://documen.tician.de/starlark-pyo3/)

*Status:* This is reasonably complete and usable.
