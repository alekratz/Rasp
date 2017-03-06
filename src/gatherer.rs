use ast::AST;
use internal::*;
use lexer;
use parser;
use preprocessor::Preprocessor;
use util;
use errors::*;

use std::path::Path;

const DEFINE_KEYWORD: &'static str = "&define";
const EXTERN_KEYWORD: &'static str = "&extern";
const TYPE_KEYWORD: &'static str = "&type";
const INCLUDE_KEYWORD: &'static str = "&include";
const OPTIONAL_TOKEN: &'static str = "?";

pub fn is_builtin(keyword: &str) -> bool {
    keyword == DEFINE_KEYWORD   ||
    keyword == EXTERN_KEYWORD   ||
    keyword == TYPE_KEYWORD     ||
    keyword == INCLUDE_KEYWORD
}

pub trait Gatherer<T> {
    fn gather(&mut self, ast: &Vec<AST>) -> Result<Vec<T>> {
        let mut items = Vec::new();

        for ast_item in ast {
            if let &AST::Expr(ref range, ref exprs) = ast_item {
                let visit_result = self.visit_exprs(exprs);
                if visit_result.is_err() {
                    visit_result.chain_err(|| format!("builtin expression at {}", range))?;
                }
                else if let Ok(i) = visit_result {
                    if i.is_some() {
                        items.push(i.unwrap());
                    }
                }
            }
        }
        Ok(items)
    }

    fn visit_exprs(&mut self, exprs: &Vec<AST>) -> Result<Option<T>> {
        // Get the first expression, if any
        if exprs.len() == 0 {
            return Ok(None);
        }
        else if let AST::Identifier(_, ref ident) = exprs[0] {
            if ident == self.keyword() {
                let visit_result = self.visit_expr(exprs);
                if visit_result.is_err() {
                    visit_result.chain_err(|| self.keyword())?;
                    unreachable!();
                }
                else {
                    return Ok(Some(visit_result.unwrap()))
                }
            }
        }
        Ok(None)
    }

    fn visit_expr(&mut self, exprs: &Vec<AST>) -> Result<T>;

    fn keyword(&self) -> &'static str;
}

/*******************************
 * INCLUDEGATHERER
 */

/// Gathers include directives
pub struct IncludeGatherer<'a> {
    fun_table: &'a mut FunTable,
    type_table: &'a mut TypeTable,
}

impl<'a> Gatherer<Vec<AST>> for IncludeGatherer<'a> {
    fn keyword(&self) -> &'static str {
        INCLUDE_KEYWORD
    }

    fn visit_expr(&mut self, exprs: &Vec<AST>) -> Result<Vec<AST>> {
        if exprs.len() == 1 {
            return Ok(Vec::new());
        }

        let mut paths = Vec::new();
        // ensure all paths are strings
        let mut index = 1;
        for path_expr in exprs.iter().skip(1) {
            if let &AST::StringLit(_, ref p) = path_expr {
                // add it to the paths list
                let path = Path::new(p);
                // ensure all paths exist
                if !path.exists() {
                    return Err(format!("included file {} does not exist", path.display()).into());
                }
                // NOTE : This will print illegal index types AND paths in the same loop; makes handling multiple errors a little weird
                paths.push(path);
            }
            else {
                return Err(format!("item at index {} must be a string literal (got {} instead)", 
                                   index, path_expr).into());
            }

            index += 1;
        }

        // attempt to compile all paths collected thus far
        let mut asts = Vec::new();
        for path in paths {
            let mut fun_table = FunTable::new(Vec::new());
            let mut type_table = TypeTable::new(Vec::new());
            let compile_result = self.compile_path(path, &mut fun_table, &mut type_table);
            if compile_result.is_err() {
                compile_result.chain_err(|| format!("included file {}", path.display()))?;
            }
            else if let Ok(mut a) = compile_result {
                asts.append(&mut a);
            }
            self.fun_table.merge(fun_table);
            self.type_table.merge(type_table)?;
        }
        Ok(asts)
    }
}

impl<'a> IncludeGatherer<'a> {

    /// Creates a new IncludeGatherer.
    /// `fun_table` is a mutable reference to a `FunTable`.
    /// `type_table` is a mutable reference to a `TypeTable`.
    pub fn new(fun_table: &'a mut FunTable, type_table: &'a mut TypeTable) -> IncludeGatherer<'a> {
        IncludeGatherer {
            fun_table: fun_table,
            type_table: type_table,
        }
    }

    /// Utility function that attempts to turn a path into an AST
    /// `funtbl` is a mutable reference to a `FunTable`.
    /// `typetbl` is a mutable reference to a `TypeTable`.
    fn compile_path(&mut self, path: &Path, mut funtbl: &mut FunTable, mut typetbl: &mut TypeTable) -> Result<Vec<AST>> {
        // I implore you to find a messier method
        let file_contents = util::read_file(path.to_str().expect("Got a weird filename"))
            .expect("Failed to load the file (permissions issues probably)");
        let mut parser = parser::Parser
            ::new(lexer::Lexer::new(&file_contents));
        let parse_result = parser.parse();
        if parse_result.is_err() {
            return parse_result;
        }
        let mut ast = parse_result.unwrap();
        // preprocess *this* AST
        {
            let mut preprocessor = Preprocessor::new(path.to_str().unwrap(), &mut ast, &mut funtbl, &mut typetbl);
            let preproc_result = preprocessor.preprocess();
            if let Err(e) = preproc_result {
                return Err(e);
            }
        }
        Ok(ast)
    }
}

/*******************************
 * FUNGATHERER
 */


/// Gathers function definitions
pub struct FunGatherer<'a> {
    source_file: &'a str,
    type_table: &'a TypeTable,
}

impl<'a> FunGatherer<'a> {
    pub fn new(source_file: &'a str, type_table: &'a TypeTable) -> FunGatherer<'a> {
        FunGatherer {
            source_file: source_file,
            type_table: type_table,
        }
    }

    fn get_params(&self, expr_list: &Vec<AST>) -> Result<Vec<Param>> {
        let mut params = Vec::new();

        let limit = expr_list.len();
        let mut i = 0;
        let mut optional = false;

        loop {
            if i >= limit {
                break;
            }

            let ref name_expr = expr_list[i];
            if !name_expr.is_identifier() {
                return Err(format!("expected identifier in params list, but instead got a {} token",
                                   name_expr).into())
            }
            let name = name_expr.identifier();
            // check special names
            if name == OPTIONAL_TOKEN {
                if optional {
                    return Err(format!("only one `{}' token is allowed in parameter declarations", OPTIONAL_TOKEN).into());
                }
                else {
                    optional = true;
                }
            }
            else {
                let param = if i + 1 == limit {
                    // last item
                    Param::any(name.to_string(), optional)
                }
                else {
                    i += 1;
                    let ref next_expr = expr_list[i];
                    if !next_expr.is_identifier() {
                        return Err(format!("expected identifier in params list, but instead got a {} token",
                                           next_expr).into())
                    }

                    if let Some(typ) = self.type_table.get_type(next_expr.identifier()) {
                        // defined type
                        Param::new(name.to_string(), typ.clone(), optional)
                    }
                    else {
                        i -= 1;
                        Param::any(name.to_string(), optional)
                    }
                };
                params.push(param);
            }
            i += 1;
        }
        Ok(params)
    }
}

impl<'a> Gatherer<Function> for FunGatherer<'a> {

    fn keyword(&self) -> &'static str {
        DEFINE_KEYWORD
    }

    fn visit_expr(&mut self, exprs: &Vec<AST>) -> Result<Function> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == DEFINE_KEYWORD);
        if exprs.len() < 3 {
            return Err(format!("{kw} must be at least 3 items long: I found {} items ({kw} NAME (PARAMS) ... )", exprs.len(), kw=DEFINE_KEYWORD)
                       .into());
        }

        let name = exprs[1].identifier();
        let params = match &exprs[2] {
            &AST::Expr(ref r, ref expr_list) => match self.get_params(expr_list) {
                Ok(params) => params,
                e => e.chain_err(|| format!("{}", r))?,
            },
            ref t => return Err(format!("expected params list, but instead got a {} item", t).into()),
        };

        if exprs.len() == 3 {
            Ok(Function::new(name.to_string(), params, String::new(), Vec::new(), self.source_file))
        }
        else {
            assert!(exprs.len() >= 4);
            // get whether this is the docstring, or if it's the start of the body
            let mut start = 3;
            let docstring = if let AST::StringLit(_, ref s) = exprs[start] {
                start += 1;
                s.to_string()
            }
            else {
                String::new()
            };

            let mut body = Vec::new();
            for expr in exprs.iter().skip(start) {
                 body.push(expr.clone());
            }
            Ok(Function::new(name.to_string(), params, docstring, body, self.source_file))
        }
    }
}

/*
/*******************************
 * EXTERNGATHERER
 */

pub struct ExternGatherer;

impl Gatherer<Function> for ExternGatherer {

    fn keyword(&self) -> &'static str {
        EXTERN_KEYWORD
    }

    fn visit_expr(&mut self, exprs: &Vec<AST>) -> Result<Function> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == EXTERN_KEYWORD );
        if exprs.len() < 3 || exprs.len() > 4 {
            return Err(format!("{kw} must be at least 3 and at most 4 items long: I found {} items ({kw} NAME (PARAMS) ... )", exprs.len(), kw=EXTERN_KEYWORD).into());
        }
        let name = exprs[1].identifier();
        let mut params = Vec::new();
        match &exprs[2] {
            &AST::Expr(_, ref expr_list) => {
                for e in expr_list {
                    match e {
                        &AST::Identifier(_, ref s) => params.push(s.to_string()),
                        ref t => return Err(format!("expected identifier in params list, but instead got a {} item", t).into()),
                    }
                }
            },
            ref t => return Err(format!("expected params list, but instead got a {} item", t).into()),
        }
        if exprs.len() == 3 {
            Ok(Function::external(name.to_string(), params, String::new()))
        }
        else if exprs.len() == 4 {
            let docstring = if let AST::StringLit(_, ref s) = exprs[3] {
                s.to_string()
            }
            else {
                return Err(format!("expected string literal for {kw} DOCSTRING, but instead got {}", exprs[3], kw=EXTERN_KEYWORD)
                           .into());
            };
            Ok(Function::external(name.to_string(), params, docstring))
        }
        else {
            assert!(exprs.len() > 4);
            Err(format!("too many arguments: expected at least 3 and at most 4 arguments to {kw}, but got {} arguments instead", exprs.len(), kw=EXTERN_KEYWORD)
                .into())
        }
    }
}
*/

/*******************************
 * TYPEGATHERER
 */
pub struct TypeGatherer;

impl Gatherer<(String, String, lexer::Range)> for TypeGatherer {

    fn keyword(&self) -> &'static str {
        TYPE_KEYWORD
    }

    fn visit_expr(&mut self, exprs: &Vec<AST>) -> Result<(String, String, lexer::Range)> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == TYPE_KEYWORD);
        let start = exprs[0].range()
            .start;
        if exprs.len() != 3 {
            return Err(format!("{kw} must be at exactly 3 items long: I found {} items ({kw} TYPE NEWTYPE)", exprs.len(), kw=TYPE_KEYWORD)
                       .into());
        }
        if !exprs[1].is_identifier() {
            return Err(format!("param 1: expected identifier, but instead got {}", exprs[1]).into());
        }
        if !exprs[2].is_identifier() {
            return Err(format!("param 2: expected identifier, but instead got {}", exprs[2]).into());
        }
        let oldtype = exprs[1].identifier();
        let newtype = exprs[2].identifier();
        if oldtype == newtype {
            return Err(format!("illegal type definition: cannot define a type to itself ({} to {})", 
                               oldtype, newtype).into());
        }
        let end = exprs[2].range()
            .end;
        Ok((oldtype.to_string(), newtype.to_string(), lexer::Range::new(start, end)))
    }
}

impl<'b> TypeGatherer {
    pub fn gather_and_link(&mut self, exprs: &Vec<AST>) -> Result<TypeTable> {
        let mut type_table = TypeTable::new(vec![Type::Number, Type::Str, Type::Listy]);
        match self.gather(exprs) {
            Ok(type_mappings) => {
                let mut proto_types = Vec::new();
                for (old, new, range) in type_mappings {
                    if type_table.has_type(&new) { // check that the types match before producing an error
                        let pointing_to = type_table.get_type(&new)
                                                    .unwrap();
                        if old != pointing_to.name() {
                            return Err(format!("invalid type mapping from {} to {}: was already set to {} at {}",
                                               new, old, pointing_to.name(), range)
                                       .into());
                        }
                    }
                    else if type_table.has_type(&old) {
                        type_table.add_typedef(&new, &old);
                    }
                    else {
                        proto_types.push((old, new, range));
                    }
                }

                let mut last_size = 0;
                loop {
                    if proto_types.len() == 0 {
                        break;
                    }
                    else if last_size == proto_types.len() {
                        // TODO(alek) better error message for this type deduction
                        // TODO(alek) tell user what to do if there is *not* a cycle and it's a compiler bug
                        let mut err_msg = String::from("Went one cycle without deducing a type; I am assuming there is a cycle or an invalid type specified. Here are the types I could not deduce:\n");
                        for (old, new, range) in proto_types {
                            err_msg += &format!("    {} -> {} (defined at {})\n", old, new, range);
                        }
                        return Err(err_msg.into());
                    }

                    // add types to table
                    for &(ref old, ref new, ref range) in &proto_types {
                        if type_table.has_type(new) { // check that the types match before producing an error
                            let pointing_to = type_table.get_type(new)
                                .unwrap();
                            if old != pointing_to.name() {
                                return Err(format!("invalid type mapping from {} to {} at {}: was already set to {}",
                                                   new, old, range, pointing_to.name()).into());
                            }
                        }
                        else if type_table.has_type(old) {
                            type_table.add_typedef(new, old);
                        }
                    }

                    // remove any types that were added
                    proto_types = proto_types.into_iter()
                                             .filter(|&(_, ref new, _)| {
                                                 !type_table.has_type(new)
                                             })
                                             .collect::<Vec<_>>();
                    last_size = proto_types.len();
                }

                Ok(type_table)
            }
            Err(s) => {
                Err(s)
            }
        }
    }
}
