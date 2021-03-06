use lexer::Range;
use vm::Value;
use std::fmt;

#[derive(Debug)]
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

    pub fn to_value(&self) -> Value {
        match self {
            &AST::Expr(_, ref exprs) => Value::List(exprs.iter()
                                                    .map(|x| x.to_value())
                                                    .collect()),
            &AST::StringLit(_, ref s) => Value::String(s.to_string()),
            &AST::Identifier(_, ref i) => Value::Identifier(i.to_string()),
            &AST::Number(_, n) => Value::Number(n),
        }
    }

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

    /*
    pub fn string(&self) -> &str {
        match self {
            &AST::StringLit(_, ref s) => s,
            _ => panic!("Attempted to grab string field from non-string literal"),
        }
    }    
    */

    pub fn exprs(&self) -> &Vec<AST> {
        match self {
            &AST::Expr(_, ref v) => v,
            _ => panic!("Attempted to grab expressions field from non-expression"),
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

impl Clone for AST {
    /// Deep-clones an AST object
    fn clone(&self) -> AST {
        match self {
            &AST::Expr(ref r, ref v) => AST::Expr(*r, v.clone()),
            &AST::StringLit(ref r, ref s) => AST::StringLit(*r, s.clone()),
            &AST::Identifier(ref r, ref s) => AST::Identifier(*r, s.clone()),
            &AST::Number(ref r, n) => AST::Number(*r, n),
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
