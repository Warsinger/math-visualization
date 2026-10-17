# UI basics

These user-supplied screenshots show the request field, selected-object controls, and 3D canvas. They are preserved unchanged. See [README](../README.md) for setup and supported syntax.

## Enter a plot or shape

**Auto** detects 2D versus 3D from the equation (z selects 3D); **Input mode** provides optional 2D/3D overrides. Try `x^2+y^2=1` for a circle. For a flat parabola, enter `x^2` or `y=x^2`; drag to pan, scroll to zoom, and Apply sampling after changing X/Y bounds or sample count. The mode selector affects the next submission, not the currently displayed geometry. In 3D, `plot y=x^2` remains a cylinder. The screenshots below predate the mode selector and illustrate the 3D interface.

Type a request in the field at the top, then press Enter or Apply. Try `create a helix with five turns`, `plot x^2-y^2-z^2=0`, or `plot sin(x)=z`. Feedback below the field reports completion or an error. Helix controls edit radius, height, and turns; implicit-surface controls set the sampling domain and resolution.

![Default five-turn helix with radius, height, turns, and camera controls](images/ui-default-five-turn-helix.png)

**Helix:** `create a helix with five turns` starts with radius 1, total height 4, and five turns. Drag the radius/height/turns sliders or click their numbers for precise entry. Typed edits such as `set radius to 2` update the same object. Pitch is height divided by turns; changing turns holds total height fixed.

## Sphere and function examples

![Wireframe sphere from x squared plus y squared plus z squared minus 5 equals zero](images/ui-sphere-radius-sqrt-five.png)

**Sphere:** enter `plot x^2+y^2+z^2-5`. Omitting an equality means the expression equals zero. This sphere has radius **√5 (about 2.236), not 5**; the ±3 sampling bounds include the full sphere. The screenshot shows its numerical wireframe with no depth occlusion.

![Numerically sampled implicit surface combining natural logarithm and sine](images/ui-log-sine-implicit-surface.png)

**Functions:** enter `plot ln(x^2)+sin(y^2)+z^2-5`. This combines natural log, sine (radians), and arithmetic. The pictured sampling settings are X ±6.5, Y/Z ±6.7, and 39 cells per axis. It is an approximate finite-domain surface, not proof of exact or fully resolved geometry. `ln(x^2)` is undefined at x=0; sampling may miss that singularity or other fine features. Feedback saying zero skipped tetrahedra describes sampled corners only, not the validity of every point between them.

## Adjust sampling

![Implicit-surface controls with X, Y, Z half-extents of 3 and 24 cells per axis](images/implicit-surface-sampling-controls.png)

- **X ±, Y ±, Z ±:** independent half-extents of a domain symmetric about zero. For example, X ±6 samples x from -6 to 6. Drag a slider or click its number to type a precise value.
- **Cells / axis:** grid resolution, currently 8–40 (24 in the screenshot). More cells provide finer sampling but increase work. Increasing domain size with the same cell count makes the grid coarser.
- **Apply sampling:** rebuilds the current equation using the edited settings. Geometry remains cached until an edit is applied.

## Move the camera

Drag the canvas to rotate and scroll over it to zoom. **Orbit**, **Tilt**, and **Zoom** provide direct controls; **Reset view** restores the default camera. The display is an orthographic wireframe without depth occlusion. Rotation and zoom cannot recover geometry outside the sampled domain.

Numerical sampling can miss thin features or even roots and can show false crossings at discontinuities. See [supported syntax and limitations](../README.md#plot-an-equation) for details.

## Troubleshooting a clipped surface

![Earlier app screenshot showing disconnected sphere patches clipped by sampling bounds](images/ui-sphere-clipped-by-sampling-bounds.png)

**Bounded sampling example:** the request is `plot x^2+y^2+z^2-25=0`, a sphere of radius 5. Each sampling half-extent is only 3, so the grid covers [-3,3] on each axis and captures disconnected patches near the cube's corners. This is not a complete sphere view. Set X, Y, and Z half-extents to 6 and click **Apply sampling** to include the whole sphere. Zoom changes the camera view; it does not expand the sampled domain. This older screenshot's “no functions” hint predates current ln/sin/cos/tan support.

## Inspecting sampled points

Hover within eight logical pixels of a plotted line to see a highlighted point on the canvas and an immediate coordinates/residual readout in the existing bottom plot-information area, evaluated using the same equation evaluator. There is no tooltip delay or floating panel; leaving the line clears the values to an instruction prompt. The point interpolates along the displayed sampled segment. For equalities F is left minus right; bare 2D f(x) uses f(x)-y. Undefined/nonfinite evaluation is labeled explicitly. Residual size is an approximation check, not proof of exactness, and changes with equation scaling. A parametric helix shows coordinates but has no single implicit-equation residual. In 3D the closest screen segment wins, with stable first-segment ties; there is no depth/occlusion picking. Dragging suppresses inspection and retains pan/orbit controls. Hover selection is transient and is not stored in history.

## Numerical tangent exploration

On an explicit 2D `y=f(x)` plot (including bare `x^2`), enable **Explore tangent** and use the existing hover point inspection to move along the curve. The same highlighted point supplies the X coordinate for the numerical derivative and yellow tangent, which update in the same frame. Slope/error information appears in the bottom plot-information area; there is no separate point slider. Leaving the curve, dragging, or disabling exploration clears the tangent. The tangent is clipped to the applied sampling rectangle. Implicit curves such as circles require a different tangent method and show an explanation instead. Domain boundaries, corners and unstable estimates show no tangent.

The estimate compares left/right slopes over 12 halved steps, initially 0.001*max(1,abs(x)), using at most 25 function evaluations. Each side is extrapolated separately to remove leading curvature error. Agreement and stability within 0.0001*(1+abs(slope)) are necessary but not sufficient: disagreement must shrink across refinements or be consistent with a scale-based floating-point noise allowance. Persistent resolved disagreement rejects even small scaled corners; the full bounded refinement sequence is checked before acceptance. The noise allowance includes rounding in the sampled input coordinates as well as function-value cancellation. The most reliable converged estimate is retained if smaller steps deteriorate through roundoff; any resolved persistent corner disagreement still vetoes it. This is finite-difference convergence, not symbolic differentiation or proof of continuity/differentiability. Floating-point cancellation, rapidly varying functions and unsampled discontinuities can mislead it. The tangent toggle and hover selection are transient and not saved in history. Review `x^2` at x=2, `sin(x)` at 0, and `(x^2)^0.5` at 0; the latter should report no reliable slope.

## Numerical integral and area

On an explicit 2D function enable **Explore integral / area** and move **From a** and **To b** within the applied X domain. Gray lines mark the bounds, green shades above the X axis and red below it. **Signed integral** counts below-axis contributions negatively and reverses sign when bounds are reversed. **Total area** integrates the absolute function over the unordered interval and stays nonnegative. Shading clips to the visible Y domain; the reported integral includes the full sampled function values, including off-screen heights.

Composite Simpson quadrature doubles 32 intervals up to 4096. Before accepting successive signed and absolute estimates within 1e-6*(1+abs(estimate)), it cross-checks both on a non-nested n+2 interval grid to detect repeated-phase aliasing. Disagreement continues refinement and unresolved results report an error at the budget limit. The bounded worst case is at most 16,352 function evaluations, including cross-checks. Results cache until equation/bounds change. Undefined or nonfinite samples and failure to converge suppress shading/value with an explanation. This is a numerical convergence heuristic, not a rigorous error bound or continuity proof: unsampled singularities, rapid oscillations and fine features can still be missed. Shade edges interpolate samples and are also approximate. Implicit curves are unsupported; choose explicit y=f(x). Controls are transient, not history state.

Manual review: integrate `x^2` from 0 to 1 (about 1/3), `x` from -1 to 1 (signed 0, total 1), reverse bounds, and try `1/x` across zero (error). Compare bound lines/shading at minimum window size with History open.

## Surface cross-sections

On a 3D implicit surface, enable **Explore cross-section**, choose X/Y/Z (the fixed coordinate) and move **Plane position** within the applied sampling domain. An amber plane and yellow sampled intersection move together. This uses the same bounded marching-triangle contour kernel as planar implicit curves, sampled directly from the equation on the plane, not intersections with existing wireframe edges. It is reusable for future partial-derivative slices. Results cache by equation, applied sampling, axis and position; camera-only changes do not rebuild slices. Settings are transient and not history state.

The slice uses twice the applied surface resolution per free axis (16–80 cells; at most 6561 evaluations). Undefined regions are skipped and counted. An all-zero sampled field reports that the intersection may fill the plane or be unresolved. Empty geometry is not proof of no intersection; tangencies/point intersections, even roots, thin features and discontinuities retain the contour algorithm's limits. The renderer has no depth occlusion, including the plane. Try `x^2+y^2+z^2=1` with Z=0 and Z=0.5, then Z=2 for an empty sampled slice. Native interaction/appearance remains a manual check.

## Extended mathematical functions

See the [function reference](FUNCTIONS.md) for names, argument order, real domains,
principal branches and numerical limitations. In Auto mode try `cbrt(x)`,
`log(abs(x)+1,2)`, `tanh(x)` or `min(abs(x),1)`; for 3D try
`atan2(y,x)=z`. Calls take one or two parenthesized arguments, according to the
function. Inspect points and restore these plots from History as usual. Smooth
explicit functions work with tangent/area exploration; corners, jumps, poles and
branch cuts may report uncertainty or sampling artifacts. Domain exclusions are
sampling outcomes, whereas malformed syntax rejects the request without replacing
the plot. ASCII calls are required; root symbols and absolute-value bars remain
optional follow-ups.
## Multiplication in equation input

Products can omit `*` between complete factors: `4x`, `2pi`, `2π`,
`2(x+y)`, `(x+y)(x-y)`, `x(y+1)`, `(x+1)y`, `2sin(x)`,
`x sin(x)`, `sin(x)cos(x)`, `sin(x)(x+1)`, `pi(x+1)`, `x y`,
`x π`, and `2x(y+1)`. Whitespace separates names; it is not discarded.
Names are whole and case-sensitive: `xy`, `xpi`, `xsin(x)`, `sinx`,
`log2x`, `x2`, `πx` and `ππ` are unknown names. Use `x*y` or `x y`.
Functions still require parentheses (`sin (x)` works; `sin x` does not)
and their documented argument counts. Products work inside arguments:
`log(2x,2)` and `atan2(2y,3x)`. Equality and commas separate expressions.

**Multiplication and division have equal precedence and evaluate left to right.**
`1/2x` means `(1/2)*x`, not `1/(2x)`; at x=3 these give 1.5 and 1/6.
Likewise `6/2(x+1)` means `(6/2)*(x+1)`. Use parentheses for denominator
products. Powers bind first and remain right-associative: `2x^2` is `2*(x^2)`,
`2^3x` is `(2^3)*x`, `-x^2` is `-(x^2)`, and `-2x^2` is `(-2)*(x^2)`.

Numbers cannot follow another factor without `*`: `2 3`, `(x+1)2`,
`sin(x)2`, and `x 2` are errors. Write `2x` or `x*2`.
Scientific notation stays intact: `2e3` / `2E3` mean 2000 and `2e-3`
means 0.002; `2e3x` means `2000*x`. Incomplete `2e`, `2e+`, `2e-`,
and adjacent `2e(x)` are errors. For Euler's constant use `2*e` or
`2 e`; `2 e(x)` means `2*e*x`. Malformed `2..3` remains invalid.
Unsupported names such as `exp` remain unsupported. Root symbols and
absolute-value bars are not added by this syntax.

Try `y=2x`, `x^2+2y^2=1`, and `x^2+y^2+2z^2=1` in Auto mode.
Dimension selection, x-only shorthand, history source text, residuals,
tangent and integral exploration all use the same equation parser.
The existing 256-byte input and 64-level parsing limits still apply.
