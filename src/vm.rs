use ast::AST;

/// Represents a run-time value
#[derive(Clone)]
pub enum Value {
    String(String),
    Number(f64),
}

/// Represents a RASP virtual machine that runs bytecode.
pub struct VM {
}

