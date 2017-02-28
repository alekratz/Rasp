use ast::AST;
use vm::Value;
use internal::*;
use errors::*;

#[derive(Clone, Debug)]
pub enum Bytecode {
    //Nop,
    /// Calls a function with the given parameters.
    Call(String),
    /// Pushes a value onto the current stack frame.
    Push(Value),
    /// Pops a value off of the stack into a variable name
    Pop(String),
    //// Pops N values off of the stack into oblivion.
    //PopN(usize),
    /// Loads a given variable value onto the stack
    Load(String),
    /// Stores a given value in a variable value
    Store(String, Value),
}

pub struct ToBytecode<'a> {
    fun_table: &'a FunTable,
    type_table: &'a TypeTable,
}

impl<'a> ToBytecode<'a> {
    /// Creates a new ToBytecode object.
    pub fn new(fun_table: &'a FunTable, type_table: &'a TypeTable) -> ToBytecode<'a> {
        ToBytecode {
            fun_table: fun_table,
            type_table: type_table,
        }
    }

    /// Converts an abstract syntax tree to bytecode.
    pub fn to_bytecode(&mut self, ast: &Vec<AST>) -> Result<Vec<Bytecode>> {
        let mut code = Vec::new();
        for expr in ast {
            match expr {
                &AST::Expr(ref r, ref v) => {
                    match self.expr_to_bytecode(expr) {
                        Ok(mut codez) => code.append(&mut codez),
                        e => { // braces necessary because of some rust weirdness
                            e.chain_err(|| format!("{}", r))?;
                        },
                    }
                },
                &AST::StringLit(_, ref s) => 
                    code.push(Bytecode::Push(Value::String(s.to_string()))),
                &AST::Identifier(_, ref s) => { },
                &AST::Number(_, n) => 
                    code.push(Bytecode::Push(Value::Number(n))),
            }
        }
        Ok(code)
    }

    /// Converts an expression into bytecode
    fn expr_to_bytecode(&self, expr: &AST) -> Result<Vec<Bytecode>> {
        assert!(expr.is_expr());
        let mut codez = Vec::new();
        let exprs = expr.exprs();
        if exprs.len() == 0 {
            // push empty list
        }
        else {
            let ref first = exprs[0];
            match first {
                // if it's an expression, get what that expression is;
                // TODO(alek): add function stack so we can just use "pushfn" and "call" instructions
                &AST::Expr(_, _) =>
                    return Err("attempt to call expression as a function (not yet supported)".into()),
                // honestly, just treat string literals as identifiers in this context
                &AST::StringLit(ref r, ref name) | &AST::Identifier(ref r, ref name) => {
                    // TODO(alek): if we're going to check that a function exists, then we
                    //             want to do this all the way down. Either catch it *all* at
                    //             compile time, or check *none* of it at compile time.
                    /*
                    if !self.fun_table.has_fun(name) {
                        return Err(format!("attempt to call non-existent function `{}'", name).into());
                    }
                    */
                    for arg in exprs.iter().skip(1) {
                        match arg {
                            &AST::Expr(ref r, _) => {
                                match self.expr_to_bytecode(arg) {
                                    Ok(mut inner_codez) => codez.append(&mut inner_codez),
                                    e => return e.chain_err(|| format!("{}", r)),
                                }
                            },
                            &AST::Number(_, n) =>
                                codez.push(Bytecode::Push(Value::Number(n))),
                            &AST::StringLit(_, ref s) =>
                                codez.push(Bytecode::Push(Value::String(s.clone()))),
                            &AST::Identifier(_, ref s) =>
                                codez.push(Bytecode::Load(s.clone())),
                        }
                    }
                    codez.push(Bytecode::Call(name.to_string()));
                },
                // if it's a number, throw an error;
                &AST::Number(_, _) =>
                    return Err("attempt to call number literal as a function".into()),
            }
        }
        Ok(codez)
    }
}

