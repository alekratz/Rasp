use internal::{FunTable, TypeTable, Function};
use bytecode::{ToBytecode, Bytecode};
use errors::*;
use builtins::BUILTIN_FUNCTIONS;

use std::collections::HashMap;

/// Represents a run-time value
#[derive(PartialEq, Clone, Debug)]
pub enum Value {
    /// A string value.
    String(String),
    /// A numeric value.
    Number(f64),
    /// An identifier. This may be treated as a reference in the future.
    Identifier(String),
    /// A list.
    List(Vec<Value>),
    Boolean(bool),
    /// A special VM value that delimits the start of a varargs value to a function call.
    /// The value contains the number of instructions before the EndArgs.
    StartArgs(i64),
    /// A special VM value that delimits the end of a varargs value to a function call.
    EndArgs,
}

impl Value {
    pub fn type_str(&self) -> &'static str {
        match self {
            &Value::String(_) => "string",
            &Value::List(_) => "list",
            &Value::Number(_) => "number",
            &Value::Identifier(_) => "identifier",
            &Value::Boolean(_) => "boolean",
            &Value::StartArgs(_) => "startargs",
            &Value::EndArgs => "endargs",
        }
    }

    pub fn is_listy(&self) -> bool {
        match self {
            &Value::String(_) | &Value::List(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            &Value::List(_) => true,
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
            &Value::Number(_) => true,
            _ => false,
        }
    }

    /*
    pub fn list(&self) -> &Vec<Value> {
        match self {
            &Value::List(ref v) => v,
            _ => panic!("called list() on non-List vm::Value"),
        }
    }
    */

    pub fn into_list(self) -> Vec<Value> {
        match self {
            Value::List(v) => v,
            _ => panic!("called into_list() on non-List vm::Value"),
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
    
    pub fn start_args(&self) -> i64 {
        match self {
            &Value::StartArgs(n) => n,
            _ => panic!("called start_args() on non-StartArgs vm::Value"),
        }
    }

    /*
    pub fn is_start_args(&self) -> bool {
        match self {
            &Value::StartArgs(_) => true,
            _ => false,
        }
    }
    */
    
    pub fn is_end_args(&self) -> bool {
        match self {
            &Value::EndArgs => true,
            _ => false,
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
    fun_stack: Vec<String>,
}

impl VM {
    pub fn new(fun_table: FunTable, type_table: TypeTable) -> VM {
        VM {
            var_stack: Vec::new(),
            value_stack: ValueStack::new(),
            fun_table: fun_table,
            type_table: type_table,
            fun_bytecode: HashMap::new(),
            fun_stack: Vec::new(),
        }
    }

    pub fn run(&mut self, bytecode: &Vec<Bytecode>) -> Result<()>{
        let mut skip = 0usize;
        self.var_stack
            .push(VarTable::new());
        for b in bytecode {
            if skip > 0 {
                skip -= 1;
                trace!("skipping {:?}", b);
                continue;
            }
            trace!("executing {:?}", b);
            trace!("value stack: {:?}", self.value_stack);
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
                        self.fun_stack.push(fname.to_string());
                        // TODO: extra error message
                        self.run(&bytecode)?;
                        self.fun_stack.pop();
                    }
                    else if BUILTIN_FUNCTIONS.contains_key(fname.as_str()) {
                        self.fun_stack.push(fname.to_string());
                        let builtin = BUILTIN_FUNCTIONS.get(fname.as_str())
                            .unwrap();
                        builtin(self)?;
                        self.fun_stack.pop();
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
                    let value = self.value_stack
                        .pop()
                        .expect("attempted to pop a value off of an empty stack");
                    self.set_var(name, &value);
                },
                &Bytecode::Load(ref name) => {
                    let value = match self.get_var(name) {
                        Some(value) => value,
                        None => return Err(format!("unknown variable or function name: {}", name).into()),
                    }.clone();
                    self.value_stack.push(value);
                },
                &Bytecode::Store(ref name, ref value) => self.set_var(name, value),
                &Bytecode::NewVarStack => self.var_stack.push(VarTable::new()),
                &Bytecode::PopVarStack => { 
                    self.var_stack.pop()
                        .expect("tried to pop variable table stack but there was nothing on the stack");
                },
                &Bytecode::Skip(n) => skip = n,
                &Bytecode::SkipFalse(n) => match self.pop_value() {
                    Value::Number(num) => if num == 0.0 {
                        skip = n;
                    },
                    Value::String(s) => if s.len() == 0 {
                        skip = n;
                    },
                    Value::List(l) => if l.len() == 0 {
                        skip = n;
                    },
                    Value::Boolean(t) => if !t {
                        skip = n;
                    },
                    e => return Err(format!("VM error: invalid boolean value reached (got {:?})", e).into()),
                },
            }
        }
        self.var_stack
            .pop()
            .unwrap();
        Ok(())
    }

    pub fn fun_stack(&self) -> &Vec<String> {
        &self.fun_stack
    }

    pub fn fun_table(&self) -> &FunTable {
        &self.fun_table
    }

    pub fn push(&mut self, value: Value) {
        self.value_stack
            .push(value);
    }

    pub fn pop_value(&mut self) -> Value {
        if self.value_stack.len() == 0 {
            // we know a crash is going to happen
            self.dump_debug();
        }
        self.value_stack
            .pop()
            .expect("attempted to pop a value off of an empty value stack")
    }

    pub fn peek_value(&self) -> Option<&Value> {
        if self.value_stack.len() == 0 {
            None
        }
        else {
            Some(&self.value_stack[self.value_stack.len() - 1])
        }
    }

    pub fn dump_debug(&self) {
        let mut count = self.value_stack
            .len();
        debug!("--------------------------------------------------------------------------------");
        debug!("Value stack");
        for value in &self.value_stack {
            debug!("    {:02}. {:?}", count, value);
            count -= 1;
        }
        debug!("--------------------------------------------------------------------------------");
        count = self.var_stack
            .len();
        for table in &self.var_stack {
            let mut table_count = table.len();
            debug!("{:02}. Var table", count);
            for (key, value) in table {
                debug!("   {:02}. {} -> {:?}", table_count, key, value); 
                table_count -= 1;
            }
            count -= 1;
            debug!("--------------------------------------------------------------------------------");
        }
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
        let mut prelude = Vec::new();
        for ref param in &fun.params {
            prelude.push(Bytecode::Pop(param.name.clone()));
        }
        let mut bytecode = {
            let generator = ToBytecode::new(&self.fun_table, &self.type_table);
            match generator.to_bytecode(&fun.body) {
                Ok(b) => b,
                e => { 
                    e.chain_err(|| format!("failure to compile function `{}'", fun.name))?;
                    unreachable!()
                },
            }
        };
        prelude.append(&mut bytecode);
        debug!("--------------------------------------------------------------------------------");
        debug!("Compiled code for {}", fun.name);
        for ref p in &prelude {
            debug!("{:?}", p);
        }
        debug!("--------------------------------------------------------------------------------");
        Ok(prelude)
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

