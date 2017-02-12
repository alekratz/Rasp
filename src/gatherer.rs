use ast::AST;
use internal;

/// Gathers function definitions
pub struct FunGatherer { }

impl<'a> FunGatherer {
    /// Visits a list of AST items.
    fn visit_exprs(&self, exprs: &'a Vec<AST>) -> Result<Option<internal::Function<'a>>, String> {
        // Get the first expression, if any
        if exprs.len() == 0 {
            Ok(None)
        }
        else if let AST::Identifier(_, ref ident) = exprs[0] {
            if ident == "&define" {
                match self.visit_define(exprs) {
                    Ok(fun) => Ok(Some(fun)),
                    Err(s) => Err(s),
                }
            }
            else {
                Ok(None)
            }
        }
        else {
            Ok(None)
        }
    }

    fn visit_define(&self, exprs: &'a Vec<AST>) -> Result<internal::Function<'a>, String> {
        assert!(exprs[0].is_identifier());
        if exprs.len() < 3 {
            return Err(String::from("&define must be at least 3 items long (&define NAME (PARAMS) ... )"));
        }
        let name = exprs[1].identifier();
        let mut params = Vec::new();
        match &exprs[2] {
            &AST::Expr(_, ref expr_list) => {
                for e in expr_list {
                    match e {
                        &AST::Identifier(_, ref s) => params.push(s.to_string()),
                        ref t => return Err(format!("expected identifier in params list, but instead got a {} item", t)),
                    }
                }
            },
            ref t => return Err(format!("expected params list, but instead got a {} item", t)),
        }
        if exprs.len() == 3 {
            Ok(internal::Function::new(name.to_string(), params, String::new(), Vec::new(), false))
        }
        else {
            assert!(exprs.len() >= 4);
            // get whether this is the docstring, or if it's the start of the body
            let mut start = 3;
            let docstring = if let AST::StringLit(_, ref s) = exprs[start] {
                start += 1;
                s.to_string()
            }
            else {
                String::new()
            };

            let mut body = Vec::new();
            for expr in exprs.iter().skip(start) {
                body.push(expr);
            }
            Ok(internal::Function::new(name.to_string(), params, docstring, body, false))
        }
    }

    pub fn gather(&self, ast: &'a Vec<AST>) -> Result<Vec<internal::Function<'a>>, String> {
        let mut functions = Vec::new();
        for ast_item in ast {
            if let &AST::Expr(_, ref exprs) = ast_item {
                match self.visit_exprs(exprs) {
                    Ok(fun) => if fun.is_some() {
                        functions.push(fun.unwrap())
                    },
                    Err(s) => return Err(s),
                }
            }
        }
        Ok(functions)
    }
}
