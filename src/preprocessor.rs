use ast::AST;
use internal::*;
use gatherer::*;
use errors::*;

pub struct Preprocessor<'a, 'b> {
    source_file: &'a str,
    ast: &'b mut Vec<AST>,
    fun_table: &'b mut FunTable,
    type_table: &'b mut TypeTable,
}

impl<'a, 'b> Preprocessor<'a, 'b> {
    pub fn new(source_file: &'a str, ast: &'b mut Vec<AST>, fun_table: &'b mut FunTable, 
                type_table: &'b mut TypeTable) -> Preprocessor<'a, 'b> {
        Preprocessor {
            source_file: source_file,
            ast: ast,
            fun_table: fun_table,
            type_table: type_table,
        }
    }

    /// Manipulates a given AST based on builtin functions and user-defined macros.
    /// It completes the following stages:
    /// * Preprocessing
    /// * TODO : Macro handling
    pub fn preprocess(&mut self) -> Result<()>{
        // preprocess
        let preprocess_result = self.preprocess_builtins();
        if let Err(e) = preprocess_result {
            return Err(e);
        }
        // macro handling
        Ok(())
    }

    /// Does preprocessing actions on the AST. This involves:
    /// * Gathering includes
    /// * Gathering user-defined types
    /// * Gathering function definitions
    /// * Gathering external function definitions
    /// * Removing all AST items that had something gathered from them
    fn preprocess_builtins(&mut self) -> Result<()> {
        // get includes
        debug!("Gathering includes");
        {
            let include_result = {
                let mut include_gatherer = IncludeGatherer::new(self.fun_table, self.type_table);
                include_gatherer.gather(self.ast)
            };
            if include_result.is_err() {
                include_result.chain_err(|| format!("{}", self.source_file))?;
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
            let mut type_gatherer = TypeGatherer;
            let type_result = type_gatherer.gather_and_link(self.ast);
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
            let mut fun_gatherer = FunGatherer;
            let fun_result = fun_gatherer.gather(self.ast);
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
            let mut extern_gatherer = ExternGatherer;
            let fun_result = extern_gatherer.gather(self.ast);

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
