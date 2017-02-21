use ast::AST;
use internal::*;
use gatherer::*;

pub struct Compiler {
    ast: Vec<AST>,
    includes: Vec<AST>,
    fun_table: FunTable,
    type_table: TypeTable,
}

impl Compiler {
    pub fn new(ast: Vec<AST>) -> Compiler {
        Compiler {
            ast: ast,
            includes: Vec::new(),
            fun_table: FunTable::new(Vec::new()),
            type_table: TypeTable::new(Vec::new()),
        }
    }

    pub fn compile(&mut self) -> Result<(), String>{
        // get includes
        debug!("Gathering includes");
        {
            let include_gatherer = IncludeGatherer;
            let include_result = include_gatherer.gather(&mut self.ast);
            if let Err(e) = include_result {
                return Err(e);
            }
            let mut asts = include_result.unwrap();
            for mut include in asts {
                self.ast.append(&mut include);
            }
        }
        // get types
        debug!("Gathering types");
        {
            let type_gatherer = TypeGatherer;
            let type_result = type_gatherer.gather_and_link(&mut self.ast);
            if let Err(e) = type_result {
                return Err(e);
            }
            let mut types = type_result.unwrap();
            let merge_result = self.type_table
                                   .merge(types);
            if let Err(e) = merge_result {
                return Err(e);
            }
        }
        self.type_table.dump_debug();
        // get functions
        debug!("Gathering functions");
        {
            let fun_gatherer = FunGatherer;
            let fun_result = fun_gatherer.gather(&mut self.ast);
            if let Err(e) = fun_result {
                return Err(e);
            }
            let funs = fun_result.unwrap();
            self.fun_table
                .append(funs);
        }
        // get externs
        debug!("Gathering extern functions");
        {
            let extern_gatherer = ExternGatherer;
            let fun_result = extern_gatherer.gather(&mut self.ast);

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
