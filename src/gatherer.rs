use ast::AST;
use internal;
use internal::Type;

const DEFINE_KEYWORD: &'static str = "&define";
const EXTERN_KEYWORD: &'static str = "&extern";
const TYPE_KEYWORD: &'static str = "&type";

/*******************************
 * FUNGATHERER
 */

pub trait Gatherer<'a, T: 'a> {

    fn gather(&self, ast: &'a Vec<AST>) -> Result<Vec<T>, String> {
        let mut items = Vec::new();
        for ast_item in ast {
            if let &AST::Expr(ref range, ref exprs) = ast_item {
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

    fn visit_exprs(&self, exprs: &'a Vec<AST>) -> Result<Option<T>, String> {
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

    fn visit_expr(&self, exprs: &'a Vec<AST>) -> Result<T, String>;

    fn keyword(&self) -> &'static str;
}

/// Gathers function definitions
pub struct FunGatherer;

impl<'a> Gatherer<'a, internal::Function<'a>> for FunGatherer {

    fn keyword(&self) -> &'static str {
        DEFINE_KEYWORD
    }

    fn visit_expr(&self, exprs: &'a Vec<AST>) -> Result<internal::Function<'a>, String> {
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
                 body.push(expr);
            }
            Ok(internal::Function::define(name.to_string(), params, docstring, body))
        }
    }
}

/*******************************
 * EXTERNGATHERER
 */

pub struct ExternGatherer;

impl<'a> Gatherer<'a, internal::Function<'a>> for ExternGatherer {

    fn keyword(&self) -> &'static str {
        EXTERN_KEYWORD
    }

    fn visit_expr(&self, exprs: &'a Vec<AST>) -> Result<internal::Function<'a>, String> {
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

impl<'a> Gatherer<'a, (String, String)> for TypeGatherer {

    fn keyword(&self) -> &'static str {
        TYPE_KEYWORD
    }

    fn visit_expr(&self, exprs: &'a Vec<AST>) -> Result<(String, String), String> {
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

impl<'a, 'b> TypeGatherer {
    pub fn gather_and_link(&self, exprs: &'a Vec<AST>) -> Result<internal::TypeTable, String> {
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
