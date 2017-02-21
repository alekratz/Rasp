use ast::AST;
use internal::*;
use gatherer::*;
use errors::*;

pub struct Compiler<'a> {
    source_file: &'a str,
    ast: Vec<AST>,
    fun_table: FunTable,
    type_table: TypeTable,
}

impl<'a> Compiler<'a> {
    pub fn new(source_file: &str, ast: Vec<AST>) -> Compiler {
        Compiler {
            source_file: source_file,
            ast: ast,
            fun_table: FunTable::new(Vec::new()),
            type_table: TypeTable::new(Vec::new()),
        }
    }

    pub fn compile(&mut self) -> Result<()>{
        // get includes
        debug!("Gathering includes");
        {
            let include_gatherer = IncludeGatherer;
            let include_result = include_gatherer.gather(&self.ast);
            if include_result.is_err() {
                include_result.chain_err(|| format!("in {}", self.source_file))?;
            }
            else {
                let asts = include_result.unwrap();
                for mut include in asts {
                    self.ast.append(&mut include);
                }
            }
        }
        // get types
        debug!("Gathering types");
        {
            let type_gatherer = TypeGatherer;
            let type_result = type_gatherer.gather_and_link(&self.ast);
            if let Err(e) = type_result {
                return Err(e);
            }
            let types = type_result.unwrap();
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
            let fun_result = fun_gatherer.gather(&self.ast);
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

        // go through the AST and prune any items that are not builtin
        self.ast 
            .retain(|expr| {
                if expr.is_expr() && expr.exprs().len() > 0 {
                    let ref first = expr.exprs()[0];
                    !(first.is_identifier() && is_builtin(first.identifier()))
                }
                else {
                    true
                }
            });
        Ok(())
    }
}
