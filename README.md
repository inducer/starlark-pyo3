# Starlark-PyO3: Python bindings for starlark-rust

This isn't even alpha software! There is almost nothing here.

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
