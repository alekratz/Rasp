use ast::AST;
use vm::Value;
use internal::*;
use errors::*;
use builtins::BUILTIN_FUNCTIONS;

#[derive(Clone, Debug)]
pub enum Bytecode {
    //Nop,
    /// Calls a function with the given parameters.
    Call(String, usize),
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
    /// Special VM bytecode for creating a new variable stack
    NewVarStack,
    /// Special VM bytecode for forcing popping off a variable stack
    PopVarStack,
    /// Special VM bytecode for skipping N instructions unconditionally
    Skip(usize),
    /// Special VM bytecode that pops a value off the stack and skips N instructions if the value is falsy
    SkipFalse(usize),
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
    pub fn to_bytecode(&self, ast: &Vec<AST>) -> Result<Vec<Bytecode>> {
        let mut code = Vec::new();
        for expr in ast {
            match expr {
                &AST::Expr(ref r, _) => {
                    match self.expr_to_bytecode(expr) {
                        Ok(mut codez) => code.append(&mut codez),
                        e => { // braces necessary because of some rust weirdness
                            e.chain_err(|| format!("{}", r))?;
                        },
                    }
                },
                &AST::StringLit(_, ref s) => code.push(Bytecode::Push(Value::String(s.to_string()))),
                &AST::Identifier(_, ref s) => code.push(Bytecode::Load(s.to_string())),
                &AST::Number(_, n) => code.push(Bytecode::Push(Value::Number(n))),
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
            codez.push(Bytecode::Push(Value::List(Vec::new())));
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
                    if name == "let" {
                        match self.let_builtin(expr) {
                            Ok(mut inner) => codez.append(&mut inner),
                            e => {
                                e.chain_err(|| format!("{}", r))?;
                            }
                        }
                    }
                    else if name == "list" {
                        match self.list_builtin(expr) {
                            Ok(mut inner) => codez.append(&mut inner),
                            e => {
                                e.chain_err(|| format!("{}", r))?;
                            }
                        }
                    }
                    else if name == "if" {
                        match self.if_builtin(expr) {
                            Ok(mut inner) => codez.append(&mut inner),
                            e => {
                                e.chain_err(|| format!("{}", r))?;
                            }
                        }
                    }
                    else if !self.fun_table.has_fun(name) && !BUILTIN_FUNCTIONS.contains_key(name.as_str()) {
                        return Err(format!("attempt to call non-existent function `{}'", name).into());
                    }
                    else {
                        let mut count = 0;
                        let args = exprs.into_iter()
                            .skip(1)
                            .collect::<Vec<&AST>>();
                        let arg_count = args.len();
                        if BUILTIN_FUNCTIONS.contains_key(name.as_str()) {
                            // TODO(alek): Check args for builtin functions
                            for arg in args {
                                count += 1;
                                if arg.is_expr() {
                                    match self.expr_to_bytecode(arg) {
                                        Ok(mut inner_codez) => codez.append(&mut inner_codez),
                                        e => return e.chain_err(|| format!("{}", r)),
                                    }
                                }
                                else if arg.is_identifier() {
                                    codez.push(Bytecode::Load(arg.identifier().to_string()));
                                }
                                else {
                                    codez.push(Bytecode::Push(arg.to_value()));
                                }
                            }
                        }
                        else {
                            let fun = self.fun_table
                                .get_fun(name)
                                .unwrap();
                            let min_args = self.min_function_args(&fun);
                            let max_args = self.max_function_args(&fun);
                            if arg_count > max_args || arg_count < min_args {
                                return if max_args == min_args {
                                    Err(format!("no variant of function {} takes {} arguments (takes exactly {} arguments)", 
                                                fun.name, arg_count, min_args).into())
                                }
                                else {
                                    Err(format!("no variant of function {} takes {} arguments (takes {} to {} arguments)", 
                                                fun.name, arg_count, min_args, max_args).into())
                                }
                            }

                            let mut arg_index = 0;
                            loop {
                                if arg_index == arg_count { break; }

                                let ref param = fun.params[arg_index];
                                let ref arg = args[arg_index];
                                if arg.is_expr() {
                                    match self.expr_to_bytecode(arg) {
                                        Ok(mut inner_codez) => codez.append(&mut inner_codez),
                                        e => return e.chain_err(|| format!("{}", r)),
                                    }
                                }
                                else if arg.is_identifier() {
                                    codez.push(Bytecode::Load(arg.identifier().to_string()));
                                }
                                else {
                                    codez.push(Bytecode::Push(arg.to_value()));
                                }
                                arg_index += 1;
                            }
                        }
                        codez.push(Bytecode::Call(name.to_string(), arg_count));
                    }
                },
                // if it's a number, throw an error;
                &AST::Number(_, _) =>
                    return Err("attempt to call number literal as a function".into()),
            }
        }
        Ok(codez)
    }

    fn min_function_args(&self, fun: &Function) -> usize {
        let mut count = 0;
        for param in &fun.params {
            if param.optional {
                break;
            }
            else {
                count += 1;
            }
        }
        count
    }

    fn max_function_args(&self, fun: &Function) -> usize {
        let mut count = 0;
        for param in &fun.params {
            if param.optional {
                count += 1;
            }
            else {
                count += 1;
            }
        }
        count
    }

    fn let_builtin(&self, ast: &AST) -> Result<Vec<Bytecode>> {
        assert!(ast.is_expr());
        let exprs = ast.exprs();
        let ref first = exprs[0];
        let ref setz = exprs[1];
        let the_rest = exprs
            .iter()
            .skip(2)
            .map(|x| x.clone())
            .collect::<Vec<AST>>();
        if !first.is_identifier() {
            Err("let function must be called as an identifier".into())
        }
        else if !setz.is_expr() {
            Err("second argument of let function must be a list".into())
        }
        else {
            let mut codez = Vec::new();
            assert!(first.identifier() == "let");
            codez.push(Bytecode::NewVarStack);
            for set in setz.exprs() {
                if !set.is_expr() || set.exprs().len() != 2 {
                    return Err("assignments must be a list of two items".into())
                }
                let assign = set.exprs();
                if assign.len() != 2 {
                    return Err("assignments must be exactly two items long".into())
                }
                else if !assign[0].is_identifier() {
                    return Err(format!("assignments name must be an identifier, instead got {}", assign[0]).into());
                }
                // handles function calls
                if assign[1].is_expr() {
                    match self.expr_to_bytecode(&assign[1]) {
                        Ok(mut v) => codez.append(&mut v),
                        e => return e.chain_err(|| "invalid function call"),
                    }
                    codez.push(Bytecode::Pop(assign[0].identifier().to_string()));
                }
                else {
                    codez.push(Bytecode::Store(assign[0].identifier().to_string(), assign[1].to_value()));
                }
            }
            match self.to_bytecode(&the_rest) {
                Ok(mut inner_codez) => codez.append(&mut inner_codez),
                e => return e,
            }
            codez.push(Bytecode::PopVarStack);
            Ok(codez)
        }
    }

    fn list_builtin(&self, ast: &AST) -> Result<Vec<Bytecode>> {
        assert!(ast.is_expr());
        let exprs = ast.exprs();
        let ref first = exprs[0];
        if !first.is_identifier() {
            Err("list function must be called as an identifier".into())
        }
        else {
            assert!(first.identifier() == "list");
            let the_rest = exprs
                .iter()
                .skip(1)
                .map(|x| x.clone())
                .rev()
                .collect::<Vec<AST>>();
            let mut codez = Vec::new();
            codez.push(Bytecode::Push(Value::EndArgs));
            match self.to_bytecode(&the_rest) {
                Ok(mut l) => codez.append(&mut l),
                e => return e.chain_err(|| "list function call"),
            }
            let size = (codez.len() - 1) as i64;
            codez.push(Bytecode::Push(Value::StartArgs(size)));
            codez.push(Bytecode::Call("list".to_string(), 0));
            Ok(codez)
        }
    }


    fn if_builtin(&self, ast: &AST) -> Result<Vec<Bytecode>> {
        assert!(ast.is_expr());
        let exprs = ast.exprs();
        let ref first = exprs[0];
        if !first.is_identifier() {
            Err("if function must be called as an identifier".into())
        }
        else {
            assert!(first.identifier() == "if");
            let the_rest = exprs
                .iter()
                .skip(1)
                .map(|x| x.clone())
                .collect::<Vec<AST>>();
            if the_rest.len() != 3 {
                Err(format!("if function requires exactly 3 arguments, got {} instead", the_rest.len()).into())
            }
            else {
                let first = exprs[1].clone();
                let second = exprs[2].clone();
                let third = exprs[3].clone();

                let mut codez = Vec::new();
                let mut first_codez = match self.to_bytecode(&vec![first]) {
                    Ok(l) => l,
                    e => return e.chain_err(|| "condition of if function call"),
                };
                let mut second_codez = match self.to_bytecode(&vec![second]) {
                    Ok(l) => l,
                    e => return e.chain_err(|| "first expression of if function call"),
                };
                let mut third_codez = match self.to_bytecode(&vec![third]) {
                    Ok(l) => l,
                    e => return e.chain_err(|| "first expression of if function call"),
                };

                codez.append(&mut first_codez);
                codez.push(Bytecode::SkipFalse(second_codez.len() + 1));
                codez.append(&mut second_codez);
                codez.push(Bytecode::Skip(third_codez.len() + 1));
                codez.append(&mut third_codez);
                Ok(codez)
            }
        }
    }
}

