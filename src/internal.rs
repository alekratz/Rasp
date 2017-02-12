use ast::AST;

pub struct FunTable<'a> {
    funs: Vec<Function<'a>>,
}

impl<'a> FunTable<'a> {
    pub fn new(funs: Vec<Function<'a>>) -> FunTable<'a> {
        FunTable {
            funs: funs,
        }
    }
    
    pub fn append(&mut self, mut funs: Vec<Function<'a>>) {
        self.funs
            .append(&mut funs);
    }

    /// Does a linear search for if a function exists in the table
    pub fn has_fun(&self, name: &str) -> bool {
        for f in &self.funs {
            if name == f.name {
                return true;
            }
        }
        false
    }

    pub fn dump_debug(&self) {
        for fun in &self.funs {
            debug!("--------------------------------------------------------------------------------");
            debug!("name: {}", fun.name);
            debug!("params: {:?}", fun.params);
            debug!("docstring: {}", fun.docstring);
            debug!("foreign: {}", fun.foreign);
            if !fun.foreign {
                debug!("body: {} items", fun.body.len());
            }
        }
    }
}

/// Defines a function that can be called.
pub struct Function<'a> {
    pub name: String,
    pub params: Vec<String>,
    pub docstring: String,
    pub body: Vec<&'a AST>,
    pub foreign: bool,
}

impl<'a> Function<'a> {
    /// Creates a new function, with a name, its parameters, its docstring, the body, and whether it's foreign or not.
    /// Note: for now, a foreign function must not contain a body.
    pub fn new(name: String, params: Vec<String>, docstring: String, body: Vec<&'a AST>, foreign: bool) -> Function<'a> {
        assert!(foreign == (foreign && body.len() == 0), "AST body was filled out for a foreign function");
        Function {
            name: name,
            params: params,
            docstring: docstring,
            body: body,
            foreign: foreign,
        }
    }

    pub fn define(name: String, params: Vec<String>, docstring: String, body: Vec<&'a AST>) -> Function<'a> {
        Function::new(name, params, docstring, body, false)
    }

    pub fn external(name: String, params: Vec<String>, docstring: String) -> Function<'a> {
        Function::new(name, params, docstring, Vec::new(), true)
    }
}
