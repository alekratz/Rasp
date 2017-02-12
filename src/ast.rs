use lexer::Range;
use std::fmt;

pub enum AST {
    Expr(Range, Vec<AST>),
    StringLit(Range, String),
    Identifier(Range, String),
    Number(Range, f64),
}

impl AST {
    /*
    /// Adds an expression to an AST::Expr item.
    pub fn add_expr(&mut self, expr: AST) {
        if let &mut AST::Expr(_, ref mut exprs) = self {
            exprs.push(expr);
        }
        else {
            panic!("Tried to add an expression to a non-AST::Expr item");
        }
    }
    */

    pub fn range(&self) -> &Range {
        match self {
            &AST::Expr(ref r, _) => r,
            &AST::StringLit(ref r, _) => r,
            &AST::Identifier(ref r, _) => r,
            &AST::Number(ref r, _) => r,
        }
    }

    pub fn identifier(&self) -> &str {
        match self {
            &AST::Identifier(_, ref s) => s,
            _ => panic!("Attempted to grab identifier field from non-identifier"),
        }
    }

    pub fn is_expr(&self) -> bool {
        match self {
            &AST::Expr(_, _) => true,
            _ => false,
        }
    }

    /*
    pub fn is_stringlit(&self) -> bool {
        match self {
            &AST::StringLit(_, _) => true,
            _ => false,
        }
    }
    */

    pub fn is_identifier(&self) -> bool {
        match self {
            &AST::Identifier(_, _) => true,
            _ => false,
        }
    }

    pub fn display_recursive(&self, f: &mut fmt::Formatter, level: i32) -> fmt::Result {
        match self {
            &AST::Expr(_, ref v) => {
                print_spaces(level * 4, f);
                write!(f, "(\n").unwrap();
                for e in v {
                    e.display_recursive(f, level + 1).unwrap();
                    write!(f, "\n").unwrap();
                }
                print_spaces(level * 4, f);
                write!(f, ")")
            },
            &AST::StringLit(_, ref s) => {
                print_spaces(level * 4, f);
                write!(f, "\"{}\"", s)
            },
            &AST::Identifier(_, ref s) => {
                print_spaces(level * 4, f);
                write!(f, "{}", s)
            },
            &AST::Number(_, n) => {
                print_spaces(level * 4, f);
                write!(f, "{}", n)
            },
        }
    }
}

fn print_spaces(count: i32, f: &mut fmt::Formatter) {
    if count > 0 {
        for _ in 0 .. count { write!(f, " ").unwrap(); }
    }
}

impl fmt::Display for AST {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        /*
        match self {
            &AST::Expr(_, _) => write!(f, "expression"),
            &AST::Identifier(_, ref ident) => write!(f, "identifier {}", ident),
            &AST::StringLit(_, _) => write!(f, "string literal"),
            &AST::Number(_, _) => write!(f, "number literal"),
        }
        */
        self.display_recursive(f, 0)
    }
}
