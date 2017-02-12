use ast::AST;
use internal::*;
use gatherer::{FunGatherer, ExternGatherer};

pub struct Compiler<'a> {
    ast: &'a Vec<AST>,
    fun_table: FunTable<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(ast: &'a Vec<AST>) -> Compiler<'a> {
        Compiler {
            ast: ast,
            fun_table: FunTable::new(Vec::new()),
        }
    }

    pub fn compile(&mut self) -> Result<(), String>{

        // get functions
        {
            let fun_gatherer = FunGatherer { };
            let fun_result = fun_gatherer.gather(&self.ast);
            if let Err(e) = fun_result {
                return Err(e);
            }
            let funs = fun_result.unwrap();
            self.fun_table
                .append(funs);
        }
        // get externs
        {
            let extern_gatherer = ExternGatherer { };
            let fun_result = extern_gatherer.gather(&self.ast);

            if let Err(e) = fun_result {
                return Err(e);
            }
            let funs = fun_result.unwrap();
            self.fun_table
                .append(funs);
        }
        self.fun_table
            .dump_debug();

        Ok(())
    }
}
