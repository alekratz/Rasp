use ast::AST;
use errors::*;

const INT_TYPE: &'static str = ":int";
const STRING_TYPE: &'static str = ":string";
const LISTY_TYPE: &'static str = ":listy";
const ANY_TYPE: &'static str = ":any";

#[derive(Debug)]
pub struct Param  {
    pub name: String,
    pub param_type: Type,
    pub optional: bool,
    pub varargs: bool,
}

impl Param {
    pub fn new(name: String, param_type: Type, optional: bool, varargs: bool) -> Param {
        Param {
            name: name,
            param_type: param_type,
            optional: optional,
            varargs: varargs,
        }
    }
    
    pub fn from_type(name: String, param_type: Type, optional: bool) -> Param {
        Param::new(name, param_type, optional, false)
    }

    pub fn any(name: String, optional: bool) -> Param {
        Param::new(name, Type::Any, optional, false)
    }
}

/// Defines an internal type.
#[derive(Clone, Debug)]
pub enum Type {
    /// A number
    Number,
    /// A list-like structure
    Listy,
    /// A string
    Str,
    /// An "any" type - this is a catchall
    Any,
    // A user-defined type
    // UserDefined(String),
    /// A typedef
    TypeDef(String, String), /* TODO(alek) : This should be able to point at a type reference.
       for some reason, the lifetimes for it weren't working. With the string layout, we have
       potential for cycles, which are not good eats.
    */
}

impl Type {
    pub fn is_typedef(&self) -> bool {
        match self {
            &Type::TypeDef(_, _) => true,
            _ => false,
        }
    }

    /*
    pub fn is_primitive(&self) -> bool {
        match self {
            &Type::Number | &Type::Listy | &Type::Str => true,
            _ => false,
        }
    }
    */

    pub fn name(&self) -> &str {
        match self {
            &Type::Number => INT_TYPE,
            &Type::Listy => LISTY_TYPE,
            &Type::Str => STRING_TYPE,
            &Type::TypeDef(ref name, _) => name,
            &Type::Any => ANY_TYPE,
        }
    }

    pub fn alias(&self) -> &str {
        assert!(self.is_typedef(), "Attempted to get the aliased type of a non-typedef");
        if let &Type::TypeDef(_, ref other) = self {
            other
        }
        else {
            unreachable!()
        }
    }
}

pub struct TypeTable {
    types: Vec<Type>,
}

impl TypeTable {
    pub fn new(types: Vec<Type>) -> TypeTable {
        TypeTable {
            types: types,
        }
    }

    pub fn get_type(&self, type_name: &str) -> Option<&Type> {
        for t in &self.types {
            if t.name() == type_name {
                if let &Type::TypeDef(_, ref points_to) = t {
                    return self.get_type(points_to);
                }
                else {
                    return Some(t);
                }
            }
        }
        None
    }

    /*
    pub fn add_type(&mut self, target: Type) {
        assert!(!self.has_type(target.name()), "Type aready exists in type table");
        self.types
            .push(target);
    }
    */

    /// Merges two type tables, consuming the other typetable.
    /// This will result in an error if there are any mismatched types.
    pub fn merge(&mut self, other: TypeTable) -> Result<()> {
        for t in &other.types {
            if let Some(ref other_type) = self.get_type(t.name()) {
                if t.name() == other_type.name() && t.is_typedef() && other_type.is_typedef()
                && t.alias() != other_type.alias() {
                    return Err(format!("type {} was originally set to alias {}, and is later set to alias {}",
                                       t.name(), t.alias(), other_type.alias()).into());
                }
            }
        }
        let mut filtered_other: Vec<Type> = other.types
            .iter()
            .cloned()  // TODO(alek) : remove this cloned call and remove #[derive(Clone)] from the type enum
            .filter(|x| !self.has_type(x.name()))
            .collect();
        self.types
            //.append(&mut other.types);
            .append(&mut filtered_other);
        Ok(())
    }

    pub fn add_typedef(&mut self, type_name: &str, target: &str) {
        assert!(!self.has_type(type_name), "Defined type aready exists in type table");

        let other_type = self.get_type(target)
                             .expect("Target type does not exist in the type table")
                             .name()
                             .to_string();
        self.types
            .push(Type::TypeDef(
                    String::from(type_name),
                    other_type));
    }

    pub fn has_type(&self, type_name: &str) -> bool {
        let type_result = self.get_type(type_name);
        type_result.is_some()
    }

    pub fn dump_debug(&self) {
        for t in &self.types {
            debug!("- TYPE -------------------------------------------------------------------------");
            debug!("name: {}", t.name());
            match t {
                &Type::Number => debug!("type: number"),
                &Type::Str => debug!("type: string"),
                &Type::Listy => debug!("type: listy"),
                &Type::Any => debug!("type: any"),
                &Type::TypeDef(ref from, _) => {
                    let to = self.get_type(from)
                        .unwrap()
                        .name();
                    debug!("type: typedef");
                    debug!("underlying type: {}", to);
                }
            }
        }
        debug!("--------------------------------------------------------------------------------");
    }
}

/// A function table.
pub struct FunTable {
    funs: Vec<Function>,
}

impl FunTable {
    /// Creates a new table with a vector.
    pub fn new(funs: Vec<Function>) -> FunTable {
        FunTable {
            funs: funs,
        }
    }
    
    /// Appends an entire vector of functions to the table.
    pub fn append(&mut self, mut funs: Vec<Function>) {
        self.funs
            .append(&mut funs);
    }

    pub fn merge(&mut self, mut other: FunTable) {
        self.funs
            .append(&mut other.funs);
    }

    /// Does a linear search for if a function exists in the table.
    pub fn has_fun(&self, name: &str) -> bool {
        for f in &self.funs {
            if name == f.name {
                return true;
            }
        }
        false
    }

    pub fn get_fun(&self, name: &str) -> Option<&Function> {
        if !self.has_fun(name) {
            None
        }
        else {
            for f in &self.funs {
                if name == f.name {
                    return Some(f);
                }
            }
            unreachable!()
        }
    }

    /// Dumps debug information about all functions in the table.
    pub fn dump_debug(&self) {
        for fun in &self.funs {
            debug!("- FUNCTION ---------------------------------------------------------------------");
            debug!("name: {}", fun.name);
            debug!("params: {:?}", fun.params);
            debug!("docstring: {}", fun.docstring);
        }
        debug!("--------------------------------------------------------------------------------");
    }

    /*
    pub fn push(&mut self, fun: Function) {
        self.funs.push(fun);
    }
    */
}

/// Describes a function that has been defined in a program.
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub docstring: String,
    pub body: Vec<AST>,
    pub source_file: String,
}

impl Function {
    /// Creates a new function, with a name, its parameters, its docstring, and the body.
    pub fn new(name: String, params: Vec<Param>, docstring: String, body: Vec<AST>, source_file: &str) -> Function {
        Function {
            name: name,
            params: params,
            docstring: docstring,
            body: body,
            source_file: source_file.to_string(),
        }
    }
}
