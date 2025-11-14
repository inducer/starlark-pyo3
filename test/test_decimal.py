import decimal

import starlark as sl


# {{{ basic decimal operations

def test_decimal_round_trip():
    """Test Python Decimal -> Starlark -> Python round-trip conversion"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()
    mod["data"] = {
        "amount": decimal.Decimal("100.25"),
        "multiplier": 2,
    }

    program = """
result = data["amount"] * data["multiplier"] + RustDecimal("0.75")
result
"""
    ast = sl.parse("decimal.star", program)
    val = sl.eval(mod, ast, glb)

    assert isinstance(val, decimal.Decimal)
    assert val == decimal.Decimal("201.25")


def test_decimal_arithmetic():
    """Test basic arithmetic operations with int coercion"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
a = RustDecimal("100.00") + 25      # addition with int
b = RustDecimal("100.00") - 25      # subtraction with int
c = RustDecimal("10.50") * 4        # multiplication with int
(a, b, c)
"""
    ast = sl.parse("arithmetic.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (
        decimal.Decimal("125.00"),
        decimal.Decimal("75.00"),
        decimal.Decimal("42.00"),
    )


def test_decimal_division_and_modulo():
    """Test division, floor division, and modulo operations"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
a = RustDecimal("10.00") / 4        # division
b = RustDecimal("10.00") // 4       # floor division
c = RustDecimal("10.00") % 4        # modulo
d = RustDecimal("7.50") / RustDecimal("2.5")  # decimal / decimal
(a, b, c, d)
"""
    ast = sl.parse("division.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (
        decimal.Decimal("2.5"),
        decimal.Decimal("2"),
        decimal.Decimal("2.00"),
        decimal.Decimal("3"),
    )


def test_decimal_negation():
    """Test unary negation operator"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
a = -RustDecimal("10.50")
b = -RustDecimal("-5.25")
(a, b)
"""
    ast = sl.parse("negation.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (decimal.Decimal("-10.50"), decimal.Decimal("5.25"))


def test_decimal_reverse_operations():
    """Test reverse operations (int op Decimal)

    Only radd and rmul are tested because Starlark doesn't support
    reverse subtraction (rsub) or reverse division (rdiv) operations.
    """
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()
    mod["value"] = decimal.Decimal("10.50")

    program = """
a = 5 + value           # reverse addition
b = 3 * value           # reverse multiplication
(a, b)
"""
    ast = sl.parse("reverse.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (decimal.Decimal("15.50"), decimal.Decimal("31.50"))


def test_decimal_comparisons_and_hashing():
    """Test comparison operators and use as dict keys (hashing)"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()
    mod["data"] = {
        "a": decimal.Decimal("10.00"),
        "b": decimal.Decimal("10.00"),
        "c": decimal.Decimal("9.99"),
    }

    program = """
eq_ab = data["a"] == data["b"]
gt_ab = data["a"] > data["b"]
lt_ac = data["a"] < data["c"]

# hashing by using as dict keys
d = {data["a"]: 1, data["c"]: 2}
result = eq_ab, gt_ab, lt_ac, d[data["b"]], d[data["c"]]
result
"""
    ast = sl.parse("compare-hash.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (True, False, False, 1, 2)


def test_decimal_truthiness():
    """Test Decimal truthiness: zero is falsy, non-zero is truthy"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
zero_bool = bool(RustDecimal("0"))
nonzero_bool = bool(RustDecimal("0.001"))
negative_bool = bool(RustDecimal("-5"))
(zero_bool, nonzero_bool, negative_bool)
"""
    ast = sl.parse("truthiness.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (False, True, True)

# }}}


# {{{ nested structures

def test_decimal_in_nested_structures():
    """Test Decimal values in nested dicts, lists, and tuples"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()
    mod["data"] = {
        "in_list": [decimal.Decimal("1.5"), decimal.Decimal("2.5")],
        "in_dict": {"nested": decimal.Decimal("3.0")},
        "in_tuple": (decimal.Decimal("4.0"), decimal.Decimal("5.5")),
    }

    program = """
# Extract from list
a = data["in_list"][0]
# Extract from nested dict
b = data["in_dict"]["nested"]
# Extract from tuple (becomes Starlark list)
c = data["in_tuple"][1]
# Simple arithmetic: 1.5 + 3.0 + 5.5 = 10.0
a + b + c
"""
    ast = sl.parse("nested.star", program)
    val = sl.eval(mod, ast, glb)

    assert isinstance(val, decimal.Decimal)
    assert val == decimal.Decimal("10.0")


def test_decimal_dict_mutation():
    """Test that Decimal values can be stored and mutated in dicts"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()
    mod["value"] = decimal.Decimal("10.00")
    mod["state"] = {"total": decimal.Decimal("0.00")}

    program = """
state["total"] = state["total"] + value
state["total"]
"""
    ast = sl.parse("dict-mutation.star", program)
    val = sl.eval(mod, ast, glb)

    assert isinstance(val, decimal.Decimal)
    assert val == decimal.Decimal("10.00")
    assert mod["state"] == {"total": decimal.Decimal("10.00")}

# }}}


# {{{ edge cases and error handling

def test_decimal_precision_vs_float():
    """Decimal preserves precision where float loses it (classic 0.1 + 0.2 example)"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
# Classic floating-point precision issue
dec_sum = RustDecimal("0.1") + RustDecimal("0.2") == RustDecimal("0.3")
flt_sum = 0.1 + 0.2 == 0.3
(dec_sum, flt_sum)
"""
    ast = sl.parse("precision.star", program)
    val = sl.eval(mod, ast, glb)

    assert val == (True, False)  # Decimal is exact, float loses precision


def test_decimal_rejects_float():
    """Mixing Decimal with float should fail to prevent silent precision loss"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = """
_ = RustDecimal("1.0") + 0.5
"""
    ast = sl.parse("reject-float.star", program)
    try:
        _ = sl.eval(mod, ast, glb)
        raise AssertionError("expected Decimal/float mixing to fail")
    except sl.StarlarkError:
        pass


def test_decimal_constructor_and_errors():
    """Test Decimal constructor with valid and invalid inputs"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    # Valid constructors (string and int)
    program = """
(RustDecimal("0.125"), RustDecimal(5))
"""
    ast = sl.parse("constructor.star", program)
    result = sl.eval(mod, ast, glb)
    assert result == (decimal.Decimal("0.125"), decimal.Decimal("5"))

    # Invalid constructors (float and bool should be rejected)
    bad_programs = [
        "RustDecimal(0.1)",       # float rejected to prevent precision loss
        "RustDecimal(True)",      # bool rejected
    ]
    for idx, snippet in enumerate(bad_programs):
        ast = sl.parse(f"invalid-{idx}.star", snippet)
        try:
            sl.eval(mod, ast, glb)
            raise AssertionError(
                f"expected Decimal constructor error for snippet {idx}"
            )
        except sl.StarlarkError:
            pass


def test_decimal_division_by_zero():
    """Test that division by zero raises appropriate errors"""
    glb = sl.Globals.extended_by([sl.LibraryExtension.RustDecimal])
    mod = sl.Module()

    program = "RustDecimal('10') / RustDecimal('0')"
    ast = sl.parse("divzero.star", program)
    try:
        _ = sl.eval(mod, ast, glb)
        raise AssertionError("expected division by zero error")
    except sl.StarlarkError:
        pass

# }}}


if __name__ == "__main__":
    import sys
    if len(sys.argv) > 1:
        exec(sys.argv[1])
    else:
        from pytest import main
        _ = main([__file__])

# vim: foldmethod=marker
