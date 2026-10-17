//! Small arithmetic language. No code execution or external services.
#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    Variable(usize),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
    Function(Function, Box<Expr>),
    Function2(Function2, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum Function {
    Ln,
    Sin,
    Cos,
    Tan,
    LogBase(f64),
    Sec,
    Csc,
    Cot,
    Asin,
    Acos,
    Atan,
    Sinh,
    Cosh,
    Tanh,
    Asinh,
    Acosh,
    Atanh,
    Sqrt,
    Cbrt,
    Abs,
    Sign,
}

#[derive(Clone, Copy, Debug)]
enum Function2 {
    Log,
    Atan2,
    Min,
    Max,
}

impl Function {
    fn named(name: &str) -> Result<Option<Self>, String> {
        Ok(Some(match name {
            "ln" => Self::Ln,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "log" => Self::LogBase(10.0),
            "sec" => Self::Sec,
            "csc" => Self::Csc,
            "cot" => Self::Cot,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "sinh" => Self::Sinh,
            "cosh" => Self::Cosh,
            "tanh" => Self::Tanh,
            "asinh" => Self::Asinh,
            "acosh" => Self::Acosh,
            "atanh" => Self::Atanh,
            "sqrt" => Self::Sqrt,
            "cbrt" => Self::Cbrt,
            "abs" => Self::Abs,
            "sign" => Self::Sign,
            _ => {
                if let Some(base) = name
                    .strip_prefix("log")
                    .filter(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
                {
                    let value = base.parse::<f64>().unwrap_or(f64::INFINITY);
                    if base.starts_with('0') || value < 2.0 || !value.is_finite() {
                        return Err("A logN base must be a finite decimal integer at least 2, without leading zeroes.".into());
                    }
                    Self::LogBase(value)
                } else {
                    return Ok(None);
                }
            }
        }))
    }
}

impl Expr {
    fn uses_variable(&self, index: usize) -> bool {
        match self {
            Self::Variable(i) => *i == index,
            Self::Neg(a) | Self::Function(_, a) => a.uses_variable(index),
            Self::Binary(_, a, b) | Self::Function2(_, a, b) => {
                a.uses_variable(index) || b.uses_variable(index)
            }
            _ => false,
        }
    }
    fn eval(&self, p: [f64; 3]) -> Option<f64> {
        let value = match self {
            Self::Number(v) => *v,
            Self::Variable(i) => p[*i],
            Self::Neg(a) => -a.eval(p)?,
            Self::Binary(op, a, b) => {
                let (a, b) = (a.eval(p)?, b.eval(p)?);
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    '^' => a.powf(b),
                    _ => unreachable!(),
                }
            }
            Self::Function(function, argument) => {
                let value = argument.eval(p)?;
                match function {
                    Function::Ln if value > 0.0 => value.ln(),
                    Function::Ln => return None,
                    Function::Sin => value.sin(),
                    Function::Cos => value.cos(),
                    // Exclude near-pole samples; grid edges can still span a pole.
                    Function::Tan if value.cos().abs() > 1e-12 => value.tan(),
                    Function::Tan => return None,
                    Function::LogBase(base) if value > 0.0 => match base {
                        2.0 => value.log2(),
                        10.0 => value.log10(),
                        _ => value.log(*base),
                    },
                    Function::LogBase(_) => return None,
                    Function::Sec if value.cos().abs() > 1e-12 => 1.0 / value.cos(),
                    Function::Csc if value.sin().abs() > 1e-12 => 1.0 / value.sin(),
                    Function::Cot if value.sin().abs() > 1e-12 => value.cos() / value.sin(),
                    Function::Sec | Function::Csc | Function::Cot => return None,
                    Function::Asin => value.asin(),
                    Function::Acos => value.acos(),
                    Function::Atan => value.atan(),
                    Function::Sinh => value.sinh(),
                    Function::Cosh => value.cosh(),
                    Function::Tanh => value.tanh(),
                    Function::Asinh => value.asinh(),
                    Function::Acosh => value.acosh(),
                    Function::Atanh => value.atanh(),
                    Function::Sqrt => value.sqrt(),
                    Function::Cbrt => value.cbrt(),
                    Function::Abs => value.abs(),
                    Function::Sign => {
                        if value == 0.0 {
                            0.0
                        } else {
                            value.signum()
                        }
                    }
                }
            }
            Self::Function2(function, a, b) => {
                let (a, b) = (a.eval(p)?, b.eval(p)?);
                match function {
                    Function2::Log if a > 0.0 && b > 0.0 && b != 1.0 => a.log(b),
                    Function2::Log => return None,
                    Function2::Atan2 if a != 0.0 || b != 0.0 => a.atan2(b),
                    Function2::Atan2 => return None,
                    Function2::Min => a.min(b),
                    Function2::Max => a.max(b),
                }
            }
        };
        value.is_finite().then_some(value)
    }
    fn has_variable(&self) -> bool {
        match self {
            Self::Variable(_) => true,
            Self::Neg(a) | Self::Function(_, a) => a.has_variable(),
            Self::Binary(_, a, b) | Self::Function2(_, a, b) => {
                a.has_variable() || b.has_variable()
            }
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Equation {
    root: Expr,
}

impl Equation {
    pub fn references_z(source: &str) -> Result<bool, String> {
        Ok(Self::parse_impl(source, false)?.root.uses_variable(2))
    }
    pub fn planar(source: &str) -> Result<Self, String> {
        let mut equation = Self::parse_impl(source, false)?;
        if equation.root.uses_variable(2) {
            return Err("2D equations may use x and y, but not z.".into());
        }
        if !source.contains('=') && !equation.root.uses_variable(1) {
            equation.root = Expr::Binary('-', Box::new(equation.root), Box::new(Expr::Variable(1)));
        }
        if !equation.root.has_variable() {
            return Err("A planar equation must describe a curve in x/y.".into());
        }
        Ok(equation)
    }
    pub fn parse(source: &str) -> Result<Self, String> {
        Self::parse_impl(source, true)
    }
    /// Explicit Cartesian y=f(x), including bare f(x) shorthand and constants.
    pub fn explicit_y(source: &str) -> Result<Self, String> {
        if source.len() > 256 {
            return Err("Equation is limited to 256 bytes.".into());
        }
        let expression = if let Some((a, b)) = source.split_once('=') {
            if b.contains('=') {
                return Err("Use one equality.".into());
            }
            if a.trim() == "y" {
                b
            } else if b.trim() == "y" {
                a
            } else {
                return Err("2D currently supports y=f(x), f(x)=y, or bare f(x). Use 3D for implicit equations.".into());
            }
        } else {
            source
        };
        let equation = Self::parse_impl(expression, false)?;
        if equation.root.uses_variable(1) || equation.root.uses_variable(2) {
            return Err("A 2D y=f(x) expression may use x only; y and z are not allowed on the function side.".into());
        }
        Ok(equation)
    }
    fn parse_impl(source: &str, require_variable: bool) -> Result<Self, String> {
        if source.len() > 256 {
            return Err("Equation is limited to 256 bytes.".into());
        }
        let mut parser = Parser {
            text: source.as_bytes(),
            pos: 0,
        };
        let left = parser.expr(0, 0)?;
        parser.space();
        let root = if parser.peek() == Some(b'=') {
            parser.pos += 1;
            Expr::Binary('-', Box::new(left), Box::new(parser.expr(0, 0)?))
        } else {
            left
        };
        parser.space();
        if parser.pos != parser.text.len() {
            return Err(format!("Unexpected input at byte {}.", parser.pos + 1));
        }
        if require_variable && !root.has_variable() {
            return Err("Use at least one variable: x, y, or z.".into());
        }
        Ok(Self { root })
    }
    /// Undefined or non-real arithmetic is excluded from sampling.
    pub fn evaluate(&self, point: [f64; 3]) -> Option<f64> {
        if point.iter().any(|v| !v.is_finite()) {
            return None;
        }
        self.root.eval(point)
    }
}

struct Parser<'a> {
    text: &'a [u8],
    pos: usize,
}
impl Parser<'_> {
    fn function(&mut self, name: &str, depth: usize) -> Result<Expr, String> {
        let unary = Function::named(name)?;
        let binary = match name {
            "log" => Some(Function2::Log),
            "atan2" => Some(Function2::Atan2),
            "min" => Some(Function2::Min),
            "max" => Some(Function2::Max),
            _ => None,
        };
        if unary.is_none() && binary.is_none() {
            return Err(format!(
                "Unknown name '{name}'. Use x/y/z, pi/π/e or a supported lowercase function."
            ));
        }
        self.space();
        if self.peek() != Some(b'(') {
            return Err(format!("Use {name}(...) with parenthesized arguments."));
        }
        self.pos += 1;
        let first = self.expr(0, depth + 1)?;
        self.space();
        let second = if self.peek() == Some(b',') {
            self.pos += 1;
            Some(self.expr(0, depth + 1)?)
        } else {
            None
        };
        self.space();
        if self.peek() != Some(b')') {
            return Err(format!(
                "Expected ')' after {name} arguments; check the argument count."
            ));
        }
        self.pos += 1;
        match (second, unary, binary) {
            (None, Some(f), _) => Ok(Expr::Function(f, Box::new(first))),
            (Some(second), _, Some(f)) => Ok(Expr::Function2(f, Box::new(first), Box::new(second))),
            (None, None, _) => Err(format!("{name} requires exactly two arguments.")),
            _ => Err(format!("{name} requires exactly one argument.")),
        }
    }
    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }
    fn space(&mut self) {
        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
            self.pos += 1;
        }
    }
    fn expr(&mut self, min: u8, depth: usize) -> Result<Expr, String> {
        if depth > 64 {
            return Err("Equation nesting is limited to 64 levels.".into());
        }
        self.space();
        let mut left = match self.peek() {
            Some(b'+' | b'-') => {
                let negative = self.peek() == Some(b'-');
                self.pos += 1;
                let value = self.expr(5, depth + 1)?;
                if negative {
                    Expr::Neg(Box::new(value))
                } else {
                    value
                }
            }
            Some(b'(') => {
                self.pos += 1;
                let value = self.expr(0, depth + 1)?;
                self.space();
                if self.peek() != Some(b')') {
                    return Err("Expected closing parenthesis.".into());
                }
                self.pos += 1;
                value
            }
            Some(c)
                if c.is_ascii_alphabetic() || self.text[self.pos..].starts_with("π".as_bytes()) =>
            {
                let start = self.pos;
                loop {
                    if self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                        self.pos += 1;
                    } else if self.text[self.pos..].starts_with("π".as_bytes()) {
                        self.pos += "π".len();
                    } else {
                        break;
                    }
                }
                let name = std::str::from_utf8(&self.text[start..self.pos]).unwrap();
                match name {
                    "pi" | "π" => Expr::Number(std::f64::consts::PI),
                    "x" => Expr::Variable(0),
                    "y" => Expr::Variable(1),
                    "z" => Expr::Variable(2),
                    "e" => Expr::Number(std::f64::consts::E),
                    _ => self.function(name, depth)?,
                }
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
                    self.pos += 1;
                }
                // A longer name (e.g. exp) is a separate factor, but a lone
                // e/E touching a number belongs to scientific notation.
                if matches!(self.peek(), Some(b'e' | b'E'))
                    && !self
                        .text
                        .get(self.pos + 1)
                        .is_some_and(u8::is_ascii_alphabetic)
                {
                    self.pos += 1;
                    if matches!(self.peek(), Some(b'+' | b'-')) {
                        self.pos += 1;
                    }
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.pos += 1;
                    }
                }
                let token = std::str::from_utf8(&self.text[start..self.pos]).unwrap();
                let value: f64 = token
                    .parse()
                    .map_err(|_| format!("Invalid numeric literal at byte {}. Complete scientific exponents; use '*' or a space before Euler's e.", start + 1))?;
                if !value.is_finite() {
                    return Err("Numeric constants must be finite.".into());
                }
                Expr::Number(value)
            }
            _ => {
                return Err(format!(
                    "Expected number, x/y/z, or parenthesis at byte {}.",
                    self.pos + 1
                ));
            }
        };
        loop {
            self.space();
            let (op, lb, rb, explicit) = match self.peek() {
                Some(b'+') => ('+', 1, 2, true),
                Some(b'-') => ('-', 1, 2, true),
                Some(b'*') => ('*', 3, 4, true),
                Some(b'/') => ('/', 3, 4, true),
                Some(b'^') => ('^', 7, 6, true),
                Some(c)
                    if c == b'('
                        || c.is_ascii_alphabetic()
                        || self.text[self.pos..].starts_with("π".as_bytes()) =>
                {
                    ('*', 3, 4, false)
                }
                Some(c) if c.is_ascii_digit() || c == b'.' => {
                    return Err(format!(
                        "Unexpected numeric factor at byte {}. Insert '*' or put the coefficient first.",
                        self.pos + 1
                    ));
                }
                _ => break,
            };
            if lb < min {
                break;
            }
            if explicit {
                self.pos += 1;
            }
            left = Expr::Binary(op, Box::new(left), Box::new(self.expr(rb, depth + 1)?));
        }
        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implicit_products_match_explicit_products_at_multiple_points() {
        for (implicit, explicit) in [
            ("4x", "4*x"),
            ("2pi", "2*pi"),
            ("2π", "2*π"),
            ("2(x+y)", "2*(x+y)"),
            ("(x+y)(x-y)", "(x+y)*(x-y)"),
            ("x(y+1)", "x*(y+1)"),
            ("(x+1)y", "(x+1)*y"),
            ("2sin(x)", "2*sin(x)"),
            ("2abs(x)", "2*abs(x)"),
            ("abs(x)sin(x)", "abs(x)*sin(x)"),
            ("x sin (x)", "x*sin(x)"),
            ("sin(x)cos(x)", "sin(x)*cos(x)"),
            ("sin(x)(x+1)", "sin(x)*(x+1)"),
            ("pi(x+1)", "pi*(x+1)"),
            ("x y", "x*y"),
            ("x π", "x*π"),
            ("2x(y+1)", "2*x*(y+1)"),
            ("2 x\t(y+1)", "2*x*(y+1)"),
            ("2log2(x)", "2*log2(x)"),
            ("log(2x,2)", "log(2*x,2)"),
            ("atan2(2y,3x)", "atan2(2*y,3*x)"),
            ("2 e(x)", "2*e*x"),
            ("2e3x", "2000*x"),
            ("2E+3x", "2000*x"),
            ("2e-3x", "0.002*x"),
            ("2E-3x", "0.002*x"),
            (".5x", "0.5*x"),
            ("2(log10(x)+sin(2x))", "2*(log10(x)+sin(2*x))"),
            ("x^2+2y^2=1", "x^2+2*y^2=1"),
        ] {
            let a = Equation::parse_impl(implicit, false).unwrap();
            let b = Equation::parse_impl(explicit, false).unwrap();
            for point in [[0.5, -1.0, 2.0], [3.0, 2.0, -1.0], [0.0; 3]] {
                assert_eq!(
                    a.evaluate(point),
                    b.evaluate(point),
                    "{implicit} at {point:?}"
                );
            }
        }
    }

    #[test]
    fn implicit_products_share_left_associative_multiplicative_precedence() {
        for (source, expected) in [
            ("1/2x", 1.5),
            ("1/(2x)", 1.0 / 6.0),
            ("6/2(x+1)", 12.0),
            ("2x^2", 18.0),
            ("2^3x", 24.0),
            ("-x^2", -9.0),
            ("-2x^2", -18.0),
            ("x^2^3", 6561.0),
            ("2-x", -1.0),
            ("2(-x)", -6.0),
            ("2x/3x", 6.0),
        ] {
            assert_eq!(
                Equation::explicit_y(source)
                    .unwrap()
                    .evaluate([3.0, 0.0, 0.0]),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn adjacency_rejects_unknown_names_numeric_factors_and_incomplete_exponents() {
        for source in [
            "xy",
            "xyz",
            "xpi",
            "pix",
            "xsin(x)",
            "sinx",
            "log2x",
            "x2",
            "x/yz",
            "sin x",
            "unknown(x)",
            "2exp(x)",
            "2e",
            "2e+",
            "2e-",
            "2e(x)",
            "2..3",
            "2 3",
            "(x+1)2",
            "sin(x)2",
            "x 2",
            "sin(x,y)",
            "log(2x,)",
            "x=2=3",
            "πx",
            "ππ",
            "xπ",
            "2efoo(x)",
        ] {
            assert!(Equation::parse_impl(source, false).is_err(), "{source}");
        }
        assert!(Equation::explicit_y("π 2").unwrap_err().contains("byte 4"));
        assert!(
            Equation::explicit_y("x 2")
                .unwrap_err()
                .contains("Insert '*'")
        );
        assert!(
            Equation::explicit_y("2e")
                .unwrap_err()
                .contains("scientific")
        );
    }

    #[test]
    fn implicit_products_retain_resource_and_arithmetic_limits() {
        let chain = format!("x{}", "(x)".repeat(85));
        assert_eq!(chain.len(), 256);
        assert_eq!(
            Equation::explicit_y(&chain)
                .unwrap()
                .evaluate([1.0, 0.0, 0.0]),
            Some(1.0)
        );
        assert!(Equation::explicit_y(&(chain + " ")).is_err());
        assert!(Equation::explicit_y(&format!("{}x{}", "2(".repeat(65), ")".repeat(65))).is_err());
        assert_eq!(
            Equation::explicit_y("0(1/x)").unwrap().evaluate([0.0; 3]),
            None
        );
        assert_eq!(
            Equation::explicit_y("0(1e308 x)")
                .unwrap()
                .evaluate([2.0, 0.0, 0.0]),
            None
        );
    }
    fn value(source: &str) -> Option<f64> {
        Equation::explicit_y(source).unwrap().evaluate([0.0; 3])
    }

    #[test]
    fn extended_function_values_and_principal_branches() {
        for (source, expected) in [
            ("log(100)", 2.0),
            ("log10(100)", 2.0),
            ("log2(8)", 3.0),
            ("log3(27)", 3.0),
            ("log16(256)", 2.0),
            ("log(8,2)", 3.0),
            ("log(8,1+1)", 3.0),
            ("log(4,0.5)", -2.0),
            ("sqrt(4)", 2.0),
            ("sqrt(0)", 0.0),
            ("cbrt(-8)", -2.0),
            ("abs(-3)", 3.0),
            ("sec(pi/3)", 2.0),
            ("csc(pi/2)", 1.0),
            ("cot(pi/4)", 1.0),
            ("asin(1)", std::f64::consts::FRAC_PI_2),
            ("asin(-1)", -std::f64::consts::FRAC_PI_2),
            ("acos(-1)", std::f64::consts::PI),
            ("acos(1)", 0.0),
            ("atan(1)", std::f64::consts::FRAC_PI_4),
            ("sinh(0)", 0.0),
            ("cosh(0)", 1.0),
            ("tanh(1000)", 1.0),
            ("tanh(-1000)", -1.0),
            ("asinh(sinh(0.5))", 0.5),
            ("acosh(cosh(0.5))", 0.5),
            ("acosh(1)", 0.0),
            ("atanh(tanh(0.5))", 0.5),
            ("cosh(2)^2-sinh(2)^2", 1.0),
            ("min(2,3)", 2.0),
            ("max(-2,-3)", -2.0),
            ("sign(-3)", -1.0),
            ("sign(0)", 0.0),
            ("sign(-0)", 0.0),
            ("sign(3)", 1.0),
            ("-sqrt(4)^2", -4.0),
            ("cbrt(-2^3)", -2.0),
            ("log(abs(-7)+1,2)", 3.0),
            ("atan2(sin(pi/4),cos(pi/4))", std::f64::consts::FRAC_PI_4),
        ] {
            let actual = value(source).unwrap();
            assert!(
                (actual - expected).abs() < 1e-12 * (1.0 + expected.abs()),
                "{source}: {actual}"
            );
        }
        for (y, x, expected) in [
            (1.0, 1.0, 0.25),
            (1.0, -1.0, 0.75),
            (-1.0, -1.0, -0.75),
            (-1.0, 1.0, -0.25),
            (0.0, -1.0, 1.0),
            (-0.0, -1.0, -1.0),
        ] {
            let actual = value(&format!("atan2({y:?},{x:?})")).unwrap();
            assert!((actual - expected * std::f64::consts::PI).abs() < 1e-12);
        }
    }

    #[test]
    fn extended_domains_poles_overflow_and_invalid_arguments_propagate() {
        for source in [
            "log(0)",
            "log2(-1)",
            "log3(0)",
            "log(-1,2)",
            "log(8,0)",
            "log(8,-2)",
            "log(8,1)",
            "sqrt(-1)",
            "asin(1.0000000001)",
            "acos(-1.0000000001)",
            "acosh(0.9999999999)",
            "atanh(1)",
            "atanh(-1)",
            "atan2(0,-0)",
            "sec(pi/2)",
            "csc(0)",
            "cot(pi)",
            "csc(1e-14)",
            "sinh(1000)",
            "cosh(1000)",
            "sqrt(-1)^0",
            "max(1,sqrt(-1))",
            "min(ln(0),1)",
            "sign(ln(0))",
            "atan2(0,ln(0))",
        ] {
            assert_eq!(value(source), None, "{source}");
        }
        assert!(value("sinh(-1000)").is_none());
        assert!(value("atanh(0.999999)").unwrap().is_finite());
        assert!(value("asinh(-1e300)").unwrap().is_finite());
        assert!(value("sqrt(1e-300)").unwrap() > 0.0);
        for name in ["csc", "cot"] {
            assert!(value(&format!("{name}(-1e-5)")).unwrap() < 0.0);
            assert!(value(&format!("{name}(1e-5)")).unwrap() > 0.0);
        }
        assert!(value("sec(pi/2-1e-5)").unwrap() > 0.0);
        assert!(value("sec(pi/2+1e-5)").unwrap() < 0.0);
    }

    #[test]
    fn extended_names_arities_and_limits_are_strict() {
        for name in [
            "log", "log10", "log2", "log3", "log16", "sec", "csc", "cot", "asin", "acos", "atan",
            "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "sqrt", "cbrt", "abs", "sign",
        ] {
            assert!(Equation::parse(&format!("{name}(x)")).is_ok(), "{name}");
            assert!(Equation::parse(&format!("{name}()")).is_err());
            assert!(Equation::parse(&format!("{name}(x,x,x)")).is_err());
            if name != "log" {
                assert!(Equation::parse(&format!("{name}(x,x)")).is_err());
            }
        }
        for name in ["atan2", "min", "max", "log"] {
            assert!(
                Equation::parse(&format!("{name}(x,z)"))
                    .unwrap()
                    .root
                    .uses_variable(2)
            );
        }
        for source in [
            "atan2(x)",
            "min(x)",
            "max(x)",
            "log0(x)",
            "log1(x)",
            "log02(x)",
            "log2x(x)",
            "atan2x(x)",
            "log(,2)",
            "log(x,)",
            "log(x,,2)",
            "min(x,2,)",
            "sqrt x",
            "SQRT(x)",
            "sqrtx",
            "sin^-1(x)",
            "√(x)",
            "|x|",
            "exp(x)",
            "floor(x)",
        ] {
            assert!(Equation::parse(source).is_err(), "{source}");
        }
        assert_eq!(value("log2(2e3)"), Some(2000.0_f64.log2()));
        assert_eq!(value("sqrt(2^3^2)"), Some(512.0_f64.sqrt()));
        assert!(Equation::explicit_y("max(1,z)").is_err());
        assert!(Equation::parse(&format!("{}x{}", "min(1,".repeat(35), ")".repeat(35))).is_ok());
        assert!(Equation::parse(&format!("{}x{}", "abs(".repeat(65), ")".repeat(65))).is_err());
        let deep = format!(
            "{}{}x{}",
            "min(1,".repeat(20),
            "(".repeat(50),
            ")".repeat(70)
        );
        assert!(deep.len() < 256 && Equation::parse(&deep).is_err());
    }
    #[test]
    fn pi_aliases_work_in_functions_arithmetic_and_equations() {
        for alias in ["pi", "π"] {
            let eval = |s: String| {
                Equation::explicit_y(&s)
                    .unwrap()
                    .evaluate([0.0; 3])
                    .unwrap()
            };
            assert_eq!(eval(alias.into()), std::f64::consts::PI);
            assert!((eval(format!("sin({alias}/2)")) - 1.0).abs() < 1e-12);
            assert!((eval(format!("cos({alias})")) + 1.0).abs() < 1e-12);
            assert_eq!(eval(format!("-{alias}^2")), -std::f64::consts::PI.powi(2));
            assert_eq!(
                Equation::parse(&format!("x={alias}")).unwrap().evaluate([
                    std::f64::consts::PI,
                    0.0,
                    0.0
                ]),
                Some(0.0)
            );
            assert!(Equation::parse(alias).is_err());
            assert_eq!(eval(format!("2{alias}")), 2.0 * std::f64::consts::PI);
            assert_eq!(eval(format!("{alias}(x)")), 0.0);
        }
        for source in ["piano", "PI", "πx", "ππ", "Π", "π+💥"] {
            assert!(Equation::explicit_y(source).is_err(), "{source}");
        }
        assert!(Equation::explicit_y(&"π+".repeat(86)).is_err());
        assert!(Equation::explicit_y(&format!("{}π{}", "(".repeat(65), ")".repeat(65))).is_err());
    }
    #[test]
    fn functions_radians_nested_calls_and_precedence() {
        let eval = |s, x| Equation::parse(s).unwrap().evaluate([x, 0.0, 0.0]).unwrap();
        assert!((eval("ln(x)", std::f64::consts::E) - 1.0).abs() < 1e-12);
        assert!((eval("sin(x)", std::f64::consts::FRAC_PI_2) - 1.0).abs() < 1e-12);
        assert!((eval("cos(x)", std::f64::consts::PI) + 1.0).abs() < 1e-12);
        assert!((eval("tan(x)", std::f64::consts::FRAC_PI_4) - 1.0).abs() < 1e-12);
        assert_eq!(eval("ln(cos(x)+1)", 0.0), 2.0_f64.ln());
        assert_eq!(eval("-sin(x)^2", 0.5), -(0.5_f64.sin().powi(2)));
        assert!((eval("sin(-x^2)", 0.5) - (-0.25_f64).sin()).abs() < 1e-12);
        assert!((eval("sin (x+1)^2^3", 0.5) - 1.5_f64.sin().powi(8)).abs() < 1e-12);
    }
    #[test]
    fn function_syntax_and_resource_limits_remain_strict() {
        for s in [
            "sin x", "sin()", "sin(x,y)", "sinhh(x)", "sin(x", "sin(x))", "sinx", "SIN(x)",
            "ln(1)", "2sinx",
        ] {
            assert!(Equation::parse(s).is_err(), "{s}");
        }
        let deep = format!("{}{}x{}", "sin(".repeat(20), "(".repeat(50), ")".repeat(70));
        assert!(deep.len() <= 256);
        assert!(Equation::parse(&deep).is_err());
    }
    #[test]
    fn invalid_domains_and_intermediates_cannot_be_masked() {
        for s in ["ln(x)", "ln(x)^0", "cos(ln(x))", "(1/x)^0"] {
            assert_eq!(Equation::parse(s).unwrap().evaluate([0.0; 3]), None, "{s}");
        }
        assert_eq!(
            Equation::parse("ln(x)").unwrap().evaluate([-1.0, 0.0, 0.0]),
            None
        );
        assert!(
            Equation::parse("ln(x)")
                .unwrap()
                .evaluate([1e-300, 0.0, 0.0])
                .unwrap()
                .is_finite()
        );
        let tan = Equation::parse("tan(x)").unwrap();
        assert_eq!(tan.evaluate([std::f64::consts::FRAC_PI_2, 0.0, 0.0]), None);
        assert!(
            tan.evaluate([std::f64::consts::FRAC_PI_2 - 1e-5, 0.0, 0.0])
                .unwrap()
                > 0.0
        );
        assert!(
            tan.evaluate([std::f64::consts::FRAC_PI_2 + 1e-5, 0.0, 0.0])
                .unwrap()
                < 0.0
        );
        assert_eq!(
            Equation::parse("sin(x)")
                .unwrap()
                .evaluate([f64::INFINITY, 0.0, 0.0]),
            None
        );
        assert_eq!(
            Equation::parse("sin(x*x)")
                .unwrap()
                .evaluate([1e300, 0.0, 0.0]),
            None
        );
    }
    #[test]
    fn precedence_equations_and_general_evaluation() {
        let eval = |s, p| Equation::parse(s).unwrap().evaluate(p).unwrap();
        assert_eq!(eval("-x^2 + 2^3^2", [3.0, 0.0, 0.0]), 503.0);
        assert_eq!(eval("(x+y)*z/2 = 6", [1.0, 2.0, 4.0]), 0.0);
        assert_eq!(eval("x^2-y^2-z^2=0", [5.0, 3.0, 4.0]), 0.0);
        assert_eq!(eval("x + 1e-2", [1.0, 0.0, 0.0]), 1.01);
    }
    #[test]
    fn euler_constant_preserves_scientific_notation_and_token_boundaries() {
        let eval = |s| {
            Equation::explicit_y(s)
                .unwrap()
                .evaluate([2.0, 0.0, 0.0])
                .unwrap()
        };
        assert_eq!(eval("e"), std::f64::consts::E);
        assert!((eval("ln(e)") - 1.0).abs() < 1e-12);
        assert_eq!(eval("e^x"), std::f64::consts::E.powi(2));
        assert_eq!(eval("-e^2"), -std::f64::consts::E.powi(2));
        assert_eq!(eval("1e-2+1E+2+e"), 100.01 + std::f64::consts::E);
        assert_eq!(
            Equation::parse("x=e")
                .unwrap()
                .evaluate([std::f64::consts::E, 0.0, 0.0]),
            Some(0.0)
        );
        assert!(Equation::parse("e").is_err());
        assert_eq!(eval("e(x)"), 2.0 * std::f64::consts::E);
        for source in ["E", "ee", "ex", "2e", "1e-", "1e999"] {
            assert!(Equation::explicit_y(source).is_err(), "{source}");
        }
    }
    #[test]
    fn rejects_unknown_code_and_malformed_input() {
        for s in [
            "", "x=0=1", "xy", "sqrtx(x)", "w+1", "x;exit()", "(x", "x+", "1e999*x", "0=0",
        ] {
            assert!(Equation::parse(s).is_err(), "{s}");
        }
        assert!(Equation::parse(&format!("{}x{}", "(".repeat(70), ")".repeat(70))).is_err());
        assert!(Equation::parse(&"x".repeat(257)).is_err());
    }
    #[test]
    fn excludes_nonreal_and_nonfinite_values() {
        assert_eq!(Equation::parse("1/x").unwrap().evaluate([0.0; 3]), None);
        assert_eq!(
            Equation::parse("x^0.5").unwrap().evaluate([-1.0, 0.0, 0.0]),
            None
        );
    }
}
