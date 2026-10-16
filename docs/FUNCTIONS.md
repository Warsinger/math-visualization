# Real-valued equation functions

Names are lowercase and case-sensitive. Calls require parentheses and the number
of comma-separated arguments listed below. Arguments may be nested expressions.
Multiplication remains explicit. `pi`/`π`, `e`, scientific numeric notation,
right-associative powers and unary-sign precedence keep their existing meanings.
`sin(x)^-1` is an arithmetic reciprocal; use `asin(x)` for inverse sine.

| Syntax | Meaning and real domain |
| --- | --- |
| `ln(x)` | Natural logarithm, x > 0. |
| `log(x)`, `log10(x)` | Base-10 logarithm, x > 0. |
| `log2(x)`, `logN(x)` | Literal integer base N >= 2, no leading zeroes, finite floating-point base. For example `log3(27)` = 3 and `log16(256)` = 2. |
| `log(x,base)` | Value first, base second; x > 0, base > 0 and base != 1. Fractional or expression bases use this form. |
| `sin(x)`, `cos(x)`, `tan(x)` | Trigonometric functions; arguments in radians. |
| `sec(x)`, `csc(x)`, `cot(x)` | 1/cos(x), 1/sin(x), cos(x)/sin(x). |
| `asin(x)` | Principal inverse sine, -1 <= x <= 1, result in [-π/2, π/2]. |
| `acos(x)` | Principal inverse cosine, -1 <= x <= 1, result in [0, π]. |
| `atan(x)` | Principal inverse tangent, finite x; mathematical range (-π/2, π/2), with rounded endpoints possible. |
| `atan2(y,x)` | Y first; quadrant-aware angle in [-π, π]. (0,0) is undefined. |
| `sinh(x)`, `cosh(x)`, `tanh(x)` | Hyperbolic functions, subject to finite-result handling. Stable tanh remains finite at large inputs, without computing sinh/cosh. |
| `asinh(x)` | Inverse hyperbolic sine, finite x. |
| `acosh(x)` | Principal inverse hyperbolic cosine, x >= 1. |
| `atanh(x)` | Inverse hyperbolic tangent, -1 < x < 1. |
| `sqrt(x)` | Nonnegative square root, x >= 0. Negative inputs are excluded, not clamped. |
| `cbrt(x)` | Real cube root, including negative x: `cbrt(-8)` = -2. |
| `abs(x)` | Absolute value; defined at zero, with a corner there. |
| `min(a,b)`, `max(a,b)` | Exactly two expressions; nest for more arguments. Both must be valid and finite. |
| `sign(x)` | -1 for negative x, 0 for either signed zero, +1 for positive x; jumps at zero. |

Inverse trigonometric results also use radians. `atan2` has a branch cut on the
negative X axis: `atan2(0,-1)` returns +π and `atan2(-0,-1)` returns -π.
Negative-zero spelling is preserved by unary minus; any pair of zero arguments is
excluded. Floating-point arithmetic may affect signs of computed zeros.

For tan/sec, samples with abs(cos(argument)) <= 1e-12 are excluded; csc/cot use
abs(sin(argument)) <= 1e-12. This near-pole guard cannot detect every interval
crossing a pole. Large trig inputs lose argument precision. Branch cuts, sign
jumps and min/max switches can also produce misleading sampled crossings.

Undefined/non-real arguments, invalid intermediates, poles and nonfinite results
exclude the sample. They are not replaced with zero or hidden by `min`, `max`,
`sign` or powers (for example `max(1,sqrt(-1))` stays undefined). Finite underflow
to zero is retained. Domain checks do not clamp near-boundary asin/acos inputs.
Results are floating-point approximations, not complex values or symbolic results.

Parser errors (unknown names, arity, malformed commas, byte/depth limits) reject
the request and retain the previous plot. Valid syntax with an invalid sampled
domain can produce a partially sampled or empty plot. Empty output does not prove
there are no solutions. Expressions remain limited to 256 UTF-8 bytes and 64
parser levels, including every nested argument.

Point residuals and history restoration use the same evaluator. Numerical tangent
and integral tools remain heuristic: abs/min/max corners, sign jumps, poles and
atan2 branch cuts need care. Tested unreliable cases report uncertainty; finite
sampling cannot prove continuity, differentiability, completeness or exact area.

Try `log(abs(x)+1,2)`, `cbrt(x)`, `sqrt(x)`, `tanh(x)`, `min(abs(x),1)`, and
`atan2(sin(x),cos(x))` in Auto mode; `log(abs(x)+1,2)+y=z` and `atan2(y,x)=z`
exercise 3D plotting. Rotate/zoom, inspect residuals, restore from History, and
compare `abs(x)` at zero with a smooth tangent example `sqrt(x)` at x=4.

Optional `exp`, `√(...)`, and absolute-value bars are not implemented here.
Floor/ceil/round, conditionals, general piecewise syntax and symbolic/complex
algebra are outside this feature. Function names do not enable implicit multiplication.
