use ast::AST;
use vm::Value;
use internal::*;

#[derive(Clone)]
pub enum Bytecode {
    //Nop,
    /// Calls a function with the given parameters.
    Call(String, Vec<Value>),
    /// Pushes a value onto the current stack frame.
    Push(Value),
}

pub struct ToBytecode<'a> {
    code: Vec<Bytecode>,
    fun_table: &'a FunTable,
    type_table: &'a TypeTable,
}

impl<'a> ToBytecode<'a> {
    /// Creates a new ToBytecode object.
    pub fn new(fun_table: &'a FunTable, type_table: &'a TypeTable) -> ToBytecode<'a> {
        ToBytecode {
            code: Vec::new(),
            fun_table: fun_table,
            type_table: type_table,
        }
    }

    /// Converts an abstract syntax tree to bytecode.
    pub fn to_bytecode(&mut self, ast: &Vec<AST>) -> Vec<Bytecode> {
        for expr in ast {
            match expr {
                &AST::Expr(_, ref v) => self.expr_to_bytecode(expr),
                &AST::StringLit(_, ref s) => 
                    self.add(Bytecode::Push(Value::String(s.to_string()))),
                &AST::Identifier(_, ref s) => { },
                &AST::Number(_, n) => 
                    self.add(Bytecode::Push(Value::Number(n))),
            }
        }
        let it = self.code.clone();
        self.code.clear();
        it
    }
    
    fn add(&mut self, byte: Bytecode) {
        self.code
            .push(byte);
    }

    fn expr_to_bytecode(&self, expr: &AST) {

    }
}

