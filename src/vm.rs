use ast::AST;
use internal::{FunTable, TypeTable, Function};
use bytecode::{ToBytecode, Bytecode};
use errors::*;
use builtins::BUILTIN_FUNCTIONS;

use std::collections::HashMap;

/// Represents a run-time value
#[derive(Clone, Debug)]
pub enum Value {
    String(String),
    Number(f64),
    Identifier(String),
    List(Vec<Box<Value>>),
}

impl Value {
    pub fn is_listy(&self) -> bool {
        match self {
            &Value::String(_) | &Value::List(_) => true,
            _ => false,
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            &Value::String(_) => true,
            _ => false,
        }
    }

    pub fn is_number(&self) -> bool {
        match self {
            &Value::Number(n) => true,
            _ => false,
        }
    }

    pub fn string(&self) -> &str {
        match self {
            &Value::String(ref s) => s.as_str(),
            _ => panic!("called string() on non-String vm::Value"),
        }
    }

    pub fn number(&self) -> f64 {
        match self {
            &Value::Number(n) => n,
            _ => panic!("called number() on non-Number vm::Value"),
        }
    }
}

type ValueStack = Vec<Value>;
type VarTable = HashMap<String, Value>;

/// Represents a RASP virtual machine that runs bytecode.
pub struct VM {
    var_stack: Vec<VarTable>,
    value_stack: ValueStack,
    fun_table: FunTable,
    type_table: TypeTable,
    /// Cache of functions' compiled Bytecode
    fun_bytecode: HashMap<String, Vec<Bytecode>>,
}

impl VM {
    pub fn new(fun_table: FunTable, type_table: TypeTable) -> VM {
        VM {
            var_stack: Vec::new(),
            value_stack: ValueStack::new(),
            fun_table: fun_table,
            type_table: type_table,
            fun_bytecode: HashMap::new(),
        }
    }

    pub fn run(&mut self, bytecode: &Vec<Bytecode>) -> Result<()>{
        self.var_stack
            .push(VarTable::new());
        for b in bytecode {
            match b {
                &Bytecode::Call(ref fname) => {
                    if self.has_function(fname) {
                        if !self.has_compiled_function(fname) {
                            let fun = self.fun_table
                                .get_fun(fname)
                                .unwrap();
                            let bytecode_result = self.compile_function(fun);
                            if let Ok(bytecode) = bytecode_result {
                                self.fun_bytecode
                                    .insert(fname.to_string(), bytecode);
                            }
                            else {
                                bytecode_result.chain_err(|| "failure to compile function")?;
                            }
                        }
                        let bytecode = self.fun_bytecode
                            .get(fname)
                            .unwrap()
                            .clone();
                        self.run(&bytecode)?;
                    }
                    else if BUILTIN_FUNCTIONS.contains_key(fname.as_str()) {
                        let builtin = BUILTIN_FUNCTIONS.get(fname.as_str())
                            .unwrap();
                        builtin(self);
                    }
                    else {
                        return Err(format!("unknown function {}", fname).into());
                    }
                },
                &Bytecode::Push(ref value) => match value {
                    // TODO(alek): references
                    &Value::Identifier(ref name) => {
                        let value = { 
                            match self.get_var(name) {
                                Some(v) => v.clone(),
                                None => return Err(format!("unknown identifier {}", name).into()),
                            }
                        };
                        self.value_stack
                            .push(value);
                    },
                    v => self.value_stack
                            .push(v.clone()),
                },
                &Bytecode::Pop(ref name) => {
                    self.value_stack
                        .pop()
                        .expect("attempted to pop a value off of an empty stack");
                },
                &Bytecode::Load(ref name) => {
                    let value = match self.get_var(name) {
                        Some(value) => value,
                        None => return Err(format!("unknown variable or function name: {}", name).into()),
                    }.clone();
                    self.push(value);
                },
                &Bytecode::Store(ref name, ref value) => self.set_var(name, value),
            }
        }
        self.var_stack
            .pop()
            .unwrap();
        Ok(())
    }

    pub fn push(&mut self, value: Value) {
        self.value_stack
            .push(value);
    }

    pub fn pop_value(&mut self) -> Value {
        self.value_stack
            .pop()
            .expect("attempted to pop a value off of an empty value stack")
    }

    fn get_var(&self, name: &str) -> Option<&Value> {
        for vartable in self.var_stack.iter().rev() {
            if vartable.contains_key(name) {
                return vartable.get(name);
            }
        }
        None
    }

    fn set_var(&mut self, name: &str, value: &Value) {
        self.var_stack
            .last_mut()
            .unwrap()
            .insert(name.to_string(), value.clone());
    }

    fn compile_function(&self, fun: &Function) -> Result<Vec<Bytecode>>{ 
        let bytecode = {
            let mut generator = ToBytecode::new(&self.fun_table, &self.type_table);
            match generator.to_bytecode(&fun.body) {
                Ok(b) => b,
                e => { 
                    e.chain_err(|| format!("failure to compile function `{}'", fun.name))?;
                    unreachable!()
                },
            }
        };
        Ok(bytecode)
    }

    /// Gets if we have a defined function either defined in the fun_table or in bytecode.
    fn has_function(&self, name: &str) -> bool {
        self.fun_table.has_fun(name)
            || self.has_compiled_function(name)
    }

    /// Gets if we have the bytecode for a function compiled
    fn has_compiled_function(&self, name: &str) -> bool {
        self.fun_bytecode
            .contains_key(name)
    }
}

