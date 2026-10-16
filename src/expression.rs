//! Small arithmetic language. No code execution or external services.
#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    Variable(usize),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
    Function(Function, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum Function {
    Ln,
    Sin,
    Cos,
    Tan,
}

impl Expr {
    fn uses_variable(&self, index: usize) -> bool {
        match self {
            Self::Variable(i) => *i == index,
            Self::Neg(a) | Self::Function(_, a) => a.uses_variable(index),
            Self::Binary(_, a, b) => a.uses_variable(index) || b.uses_variable(index),
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
                }
            }
        };
        value.is_finite().then_some(value)
    }
    fn has_variable(&self) -> bool {
        match self {
            Self::Variable(_) => true,
            Self::Neg(a) | Self::Function(_, a) => a.has_variable(),
            Self::Binary(_, a, b) => a.has_variable() || b.has_variable(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Equation {
    root: Expr,
}

impl Equation {
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
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
                    self.pos += 1;
                }
                let name = std::str::from_utf8(&self.text[start..self.pos]).unwrap();
                match name {
                    "x" => Expr::Variable(0),
                    "y" => Expr::Variable(1),
                    "z" => Expr::Variable(2),
                    "ln" | "sin" | "cos" | "tan" => {
                        let function = match name {
                            "ln" => Function::Ln,
                            "sin" => Function::Sin,
                            "cos" => Function::Cos,
                            _ => Function::Tan,
                        };
                        self.space();
                        if self.peek() != Some(b'(') {
                            return Err(format!("Use {name}(expression)."));
                        }
                        self.pos += 1;
                        let argument = self.expr(0, depth + 1)?;
                        self.space();
                        if self.peek() != Some(b')') {
                            return Err(format!("Expected ')' after {name} argument."));
                        }
                        self.pos += 1;
                        Expr::Function(function, Box::new(argument))
                    }
                    _ => {
                        return Err(format!(
                            "Unknown name '{name}'. Use x/y/z or ln/sin/cos/tan."
                        ));
                    }
                }
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.pos;
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
                    self.pos += 1;
                }
                if matches!(self.peek(), Some(b'e' | b'E')) {
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
                    .map_err(|_| "Invalid numeric literal.".to_owned())?;
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
            let (op, lb, rb) = match self.peek() {
                Some(b'+') => ('+', 1, 2),
                Some(b'-') => ('-', 1, 2),
                Some(b'*') => ('*', 3, 4),
                Some(b'/') => ('/', 3, 4),
                Some(b'^') => ('^', 7, 6),
                _ => break,
            };
            if lb < min {
                break;
            }
            self.pos += 1;
            left = Expr::Binary(op, Box::new(left), Box::new(self.expr(rb, depth + 1)?));
        }
        Ok(left)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
            "sin x", "sin()", "sin(x,y)", "sinh(x)", "sin(x", "sin(x))", "sinx", "SIN(x)", "ln(1)",
            "2sin(x)",
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
    fn rejects_unknown_code_and_malformed_input() {
        for s in [
            "", "x=0=1", "2x", "sqrt(x)", "w+1", "x;exit()", "(x", "x+", "1e999*x", "0=0",
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
