//! Small arithmetic language. No code execution or external services.
#[derive(Clone, Debug)]
enum Expr {
    Number(f64),
    Variable(usize),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

impl Expr {
    fn eval(&self, p: [f64; 3]) -> f64 {
        match self {
            Self::Number(v) => *v,
            Self::Variable(i) => p[*i],
            Self::Neg(a) => -a.eval(p),
            Self::Binary(op, a, b) => {
                let (a, b) = (a.eval(p), b.eval(p));
                match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    '^' => a.powf(b),
                    _ => unreachable!(),
                }
            }
        }
    }
    fn has_variable(&self) -> bool {
        match self {
            Self::Variable(_) => true,
            Self::Neg(a) => a.has_variable(),
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
        if !root.has_variable() {
            return Err("Use at least one variable: x, y, or z.".into());
        }
        Ok(Self { root })
    }
    /// Undefined or non-real arithmetic is excluded from sampling.
    pub fn evaluate(&self, point: [f64; 3]) -> Option<f64> {
        let value = self.root.eval(point);
        value.is_finite().then_some(value)
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
            Some(b'x' | b'y' | b'z') => {
                let i = (self.peek().unwrap() - b'x') as usize;
                self.pos += 1;
                Expr::Variable(i)
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
            "", "x=0=1", "2x", "sin(x)", "w+1", "x;exit()", "(x", "x+", "1e999*x", "0=0",
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
