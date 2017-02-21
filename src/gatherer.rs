use ast::AST;
use internal;
use internal::Type;
use lexer;
use parser;
use util;

use std::path::Path;

const DEFINE_KEYWORD: &'static str = "&define";
const EXTERN_KEYWORD: &'static str = "&extern";
const TYPE_KEYWORD: &'static str = "&type";
const INCLUDE_KEYWORD: &'static str = "&include";

pub trait Gatherer<T> {

    fn gather(&self, ast: &mut Vec<AST>) -> Result<Vec<T>, String> {
        let mut items = Vec::new();
        for ast_item in ast {
            if let &mut AST::Expr(ref range, ref mut exprs) = ast_item {
                match self.visit_exprs(exprs) {
                    Ok(i) => if i.is_some() {
                        items.push(i.unwrap())
                    },
                    Err(s) => return Err(format!("{}: {}", range, s)),
                }
            }
        }
        Ok(items)
    }

    fn visit_exprs(&self, exprs: &Vec<AST>) -> Result<Option<T>, String> {
        // Get the first expression, if any
        if exprs.len() == 0 {
            Ok(None)
        }
        else if let AST::Identifier(_, ref ident) = exprs[0] {
            if ident == self.keyword() {
                match self.visit_expr(exprs) {
                    Ok(fun) => Ok(Some(fun)),
                    Err(s) => Err(format!("{}: {}", self.keyword(), s)),
                }
            }
            else {
                Ok(None)
            }
        }
        else {
            Ok(None)
        }
    }

    fn visit_expr(&self, exprs: &Vec<AST>) -> Result<T, String>;

    fn keyword(&self) -> &'static str;
}

/*******************************
 * INCLUDEGATHERER
 */

/// Gathers include directives
pub struct IncludeGatherer;
impl Gatherer<Vec<AST>> for IncludeGatherer {
    fn keyword(&self) -> &'static str {
        INCLUDE_KEYWORD
    }

    fn visit_expr(&self, exprs: &Vec<AST>) -> Result<Vec<AST>, String> {
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
                    return Err(format!("included file {} does not exist", path.display()));
                }
                // NOTE : This will print illegal index types AND paths in the same loop; makes handling multiple errors a little weird
                // TODO : error_chain
                paths.push(path);
            }
            else {
                return Err(format!("item at index {} must be a string literal (got {} instead)", index, path_expr));
            }

            index += 1;
        }

        // attempt to compile all paths collected thus far
        let mut asts = Vec::new();
        for path in paths {
            match self.compile_path(path) {
                Ok(mut a) => asts.append(&mut a),
                Err(s) => return Err(format!("error from included file {}: {}", path.display(), s)),
            }
        }
        Ok(asts)
    }
}

impl IncludeGatherer {
    /// Utility function that attempts to turn a path into an AST
    fn compile_path(&self, path: &Path) -> Result<Vec<AST>, String> {
        // I implore you to find a messier method
        let file_contents = util::read_file(path.to_str().expect("Got a weird filename"))
            .expect("Failed to load the file (permissions issues probably)");
        let mut parser = parser::Parser
            ::new(lexer::Lexer::new(&file_contents));
        parser.parse()
    }
}

/*******************************
 * FUNGATHERER
 */


/// Gathers function definitions
pub struct FunGatherer;

impl Gatherer<internal::Function> for FunGatherer {

    fn keyword(&self) -> &'static str {
        DEFINE_KEYWORD
    }

    fn visit_expr(&self, exprs: &Vec<AST>) -> Result<internal::Function, String> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == DEFINE_KEYWORD);
        if exprs.len() < 3 {
            return Err(format!("{kw} must be at least 3 items long: I found {} items ({kw} NAME (PARAMS) ... )", exprs.len(), kw=DEFINE_KEYWORD));
        }
        let name = exprs[1].identifier();
        let mut params = Vec::new();
        match &exprs[2] {
            &AST::Expr(_, ref expr_list) => {
                for e in expr_list {
                    match e {
                        &AST::Identifier(_, ref s) => params.push(s.to_string()),
                        ref t => return Err(format!("expected identifier in params list, but instead got a {} item", t)),
                    }
                }
            },
            ref t => return Err(format!("expected params list, but instead got a {} item", t)),
        }
        if exprs.len() == 3 {
            Ok(internal::Function::define(name.to_string(), params, String::new(), Vec::new()))
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
            Ok(internal::Function::define(name.to_string(), params, docstring, body))
        }
    }
}

/*******************************
 * EXTERNGATHERER
 */

pub struct ExternGatherer;

impl Gatherer<internal::Function> for ExternGatherer {

    fn keyword(&self) -> &'static str {
        EXTERN_KEYWORD
    }

    fn visit_expr(&self, exprs: &Vec<AST>) -> Result<internal::Function, String> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == EXTERN_KEYWORD );
        if exprs.len() < 3 || exprs.len() > 4 {
            return Err(format!("{kw} must be at least 3 and at most 4 items long: I found {} items ({kw} NAME (PARAMS) ... )", exprs.len(), kw=EXTERN_KEYWORD));
        }
        let name = exprs[1].identifier();
        let mut params = Vec::new();
        match &exprs[2] {
            &AST::Expr(_, ref expr_list) => {
                for e in expr_list {
                    match e {
                        &AST::Identifier(_, ref s) => params.push(s.to_string()),
                        ref t => return Err(format!("expected identifier in params list, but instead got a {} item", t)),
                    }
                }
            },
            ref t => return Err(format!("expected params list, but instead got a {} item", t)),
        }
        if exprs.len() == 3 {
            Ok(internal::Function::external(name.to_string(), params, String::new()))
        }
        else if exprs.len() == 4 {
            let docstring = if let AST::StringLit(_, ref s) = exprs[3] {
                s.to_string()
            }
            else {
                return Err(format!("expected string literal for {kw} DOCSTRING, but instead got {}", exprs[3], kw=EXTERN_KEYWORD));
            };
            Ok(internal::Function::external(name.to_string(), params, docstring))
        }
        else {
            assert!(exprs.len() > 4);
            Err(format!("too many arguments: expected at least 3 and at most 4 arguments to {kw}, but got {} arguments instead", exprs.len(), kw=EXTERN_KEYWORD))
        }
    }
}

/*******************************
 * TYPEGATHERER
 */
pub struct TypeGatherer;

impl Gatherer<(String, String)> for TypeGatherer {

    fn keyword(&self) -> &'static str {
        TYPE_KEYWORD
    }

    fn visit_expr(&self, exprs: &Vec<AST>) -> Result<(String, String), String> {
        assert!(exprs[0].is_identifier() && exprs[0].identifier() == TYPE_KEYWORD);
        if exprs.len() != 3 {
            return Err(format!("{kw} must be at exactly 3 items long: I found {} items ({kw} TYPE NEWTYPE)", exprs.len(), kw=TYPE_KEYWORD));
        }
        if !exprs[1].is_identifier() {
            return Err(format!("param 1: expected identifier, but instead got {}", exprs[1]));
        }
        if !exprs[2].is_identifier() {
            return Err(format!("param 2: expected identifier, but instead got {}", exprs[2]));
        }
        let oldtype = exprs[1].identifier();
        let newtype = exprs[2].identifier();
        if oldtype == newtype {
            return Err(format!("illegal type definition: cannot define a type to itself ({} to {})", 
                               oldtype, newtype));
        }
        Ok((oldtype.to_string(), newtype.to_string()))
    }
}

impl<'b> TypeGatherer {
    pub fn gather_and_link(&self, exprs: &mut Vec<AST>) -> Result<internal::TypeTable, String> {
        let mut type_table = internal::TypeTable::new(vec![Type::Number, Type::Str, Type::Listy]);
        match self.gather(exprs) {
            Ok(type_mappings) => {
                let mut proto_types = Vec::new();
                for (old, new) in type_mappings {
                    if type_table.has_type(&new) { // check that the types match before producing an error
                        let pointing_to = type_table.get_type(&new)
                                                    .unwrap();
                        if old != pointing_to.name() {
                            return Err(format!("invalid type mapping from {} to {}: was already set to {}", new, old, pointing_to.name()));
                        }
                    }
                    else if type_table.has_type(&old) {
                        type_table.add_typedef(&new, &old);
                    }
                    else {
                        proto_types.push((old, new));
                    }
                }

                let mut last_size = 0;
                loop {
                    if proto_types.len() == 0 {
                        break;
                    }
                    else if last_size == proto_types.len() {
                        // TODO(alek) platform-agnostic newlines
                        // TODO(alek) tell user what to do if there is *not* a cycle and it's a compiler bug
                        let mut err_msg = String::from("Went one cycle without deducing a type; I am assuming there is a cycle or an invalid type specified. Here are the types I could not deduce:\n");
                        for (old, new) in proto_types {
                            err_msg += &format!("    {} -> {}\n", old, new);
                        }
                        return Err(err_msg);
                    }

                    // add types to table
                    for &(ref old, ref new) in &proto_types {
                        if type_table.has_type(new) { // check that the types match before producing an error
                            let pointing_to = type_table.get_type(new)
                                .unwrap();
                            if old != pointing_to.name() {
                                return Err(format!("invalid type mapping from {} to {}: was already set to {}", new, old, pointing_to.name()));
                            }
                        }
                        else if type_table.has_type(old) {
                            type_table.add_typedef(new, old);
                        }
                    }

                    // remove any types that were added
                    proto_types = proto_types.into_iter()
                                             .filter(|&(_, ref new)| {
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
