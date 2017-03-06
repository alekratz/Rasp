use vm;
use errors::*;

use libc::{
    // libc functions
    open, close, read, write,

    // libc flags
    O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_APPEND, O_TRUNC,

    // libc types
    c_int, c_void,
};

use std::collections::HashMap;
use std::ffi::CString;

/// Builtin function definition map
lazy_static! {
    pub static ref BUILTIN_FUNCTIONS: HashMap<&'static str, fn(&mut vm::VM) -> Result<()>> = {
        let mut map = HashMap::new();
        map.insert("stdopen", rasp_open as fn(&mut vm::VM) -> Result<()>);
        map.insert("stdclose", rasp_close as fn(&mut vm::VM) -> Result<()>);
        map.insert("stdwrite", rasp_write as fn(&mut vm::VM) -> Result<()>);
        map.insert("stdread", rasp_read as fn(&mut vm::VM) -> Result<()>);

        map.insert("+", plus as fn(&mut vm::VM) -> Result<()>);
        map.insert("-", minus as fn(&mut vm::VM) -> Result<()>);
        map.insert("*", times as fn(&mut vm::VM) -> Result<()>);
        map.insert("/", divide as fn(&mut vm::VM) -> Result<()>);

        map.insert("car", car as fn(&mut vm::VM) -> Result<()>);
        map.insert("cdr", cdr as fn(&mut vm::VM) -> Result<()>);
        map.insert("nil?", is_nil as fn(&mut vm::VM) -> Result<()>);
        map.insert("list", list as fn(&mut vm::VM) -> Result<()>);
        map.insert("append", append as fn(&mut vm::VM) -> Result<()>);
        map.insert("string", string as fn(&mut vm::VM) -> Result<()>);
        
        map.insert("=", equals as fn(&mut vm::VM) -> Result<()>);
        map
    };
}

/*
/// Builtin list function
/// The list function takes n parameters and makes a list out of those parameters.
pub fn list(v: &mut vm::VM) -> Result<()> {
    Ok(())
}
*/

/// Builtin string function
/// Converts the top item to a string
pub fn string(v: &mut vm::VM) -> Result<()> {
    let item = v.pop_value();
    v.push(vm::Value::String(value_to_string(item)));
    Ok(())
}

/// Auxiliary function that turns a list into a string.
fn value_to_string(val: vm::Value) -> String {
    match val {
        vm::Value::String(s) => s,
        vm::Value::Number(n) => n.to_string(),
        vm::Value::Identifier(s) => s,
        vm::Value::Boolean(b) => b.to_string(),
        vm::Value::List(l) => {
            let mut constructed = String::new();
            for i in l {
                constructed += &value_to_string(i);
            }
            constructed
        },
        _ => unreachable!(),
    }
}

/// Builtin append function
/// Puts the top two items on the stack together.
pub fn append(v: &mut vm::VM) -> Result<()> {
    let first = v.pop_value();
    let second = v.pop_value();
    if !first.is_listy() || !second.is_listy() {
        Err("append takes only listy items".into())
    }
    else if first.is_list() != second.is_list() {
        Err("append arguments either must be both Lists or Strings".into())
    }
    else if first.is_list() {
        assert!(second.is_list());
        let mut list_start = second.into_list();
        let mut list_end = first.into_list();
        list_start.append(&mut list_end);
        v.push(vm::Value::List(list_start));
        Ok(())
    }
    else {
        assert!(second.is_string() && first.is_string());
        v.push(vm::Value::String(second.string().to_string() + first.string()));
        Ok(())
    }
}

/// Builtin = function
/// Gets whether two items are equal to one another
pub fn equals(v: &mut vm::VM) -> Result<()> {
    let first = v.pop_value();
    let second = v.pop_value();
    v.push(vm::Value::Boolean(first == second));
    Ok(())
}

/// Builtin list function
/// Gets whether a given listy item is empty.
pub fn list(v: &mut vm::VM) -> Result<()> {
    let mut arg_count = v.pop_value()
        .start_args();
    let mut result_list = Vec::new();
    while arg_count >= 0 {
        if v.peek_value().is_some() {
            let value = v.pop_value();
            if value.is_end_args() {
                break;
            }
            else {
                result_list.push(value);
            }
        }
        else {
            return Err("VM error: unexpected end of value stack when popping var args".into());
        }
        arg_count -= 1;
    }
    v.push(vm::Value::List(result_list));
    Ok(())
}

/// Builtin nil? function
/// Gets whether a given listy item is empty.
pub fn is_nil(v: &mut vm::VM) -> Result<()> {
    let first = v.pop_value();
    if first.is_listy() {
        match first {
            vm::Value::String(ref s) => v.push(vm::Value::Boolean(s.len() == 0)),
            vm::Value::List(ref l) => v.push(vm::Value::Boolean(l.len() == 0)),
            _ => unreachable!(),
        }
        Ok(())
    }
    else {
        debug!("{:?}", first);
        Err(format!("argument to `nil?' function must be listy (instead got {})", first.type_str()).into())
    }
}

/// Builtin cdr function
/// Gets a list, minus the first item.
pub fn cdr(v: &mut vm::VM) -> Result<()> {
    let first = v.pop_value();
    if first.is_listy() {
        match first {
            vm::Value::String(s) => if s.len() > 0 {
                    v.push(vm::Value::String(s.chars().skip(1).collect()));
                }
                else {
                    v.push(vm::Value::String(String::new()));
                },
            vm::Value::List(l) => if l.len() > 0 {
                    let e = l.into_iter()
                        .skip(1)
                        .collect();
                    v.push(vm::Value::List(e));
                }
                else {
                    v.push(vm::Value::List(Vec::new()));
                },
            _ => unreachable!(),
        }
        Ok(())
    }
    else {
        Err(format!("argument to `cdr' function must be listy").into())
    }
}

/// Builtin car function
/// Gets the first element of a list.
pub fn car(v: &mut vm::VM) -> Result<()> {
    let first = v.pop_value();
    if first.is_listy() {
        match first {
            vm::Value::String(s) => if let Some(c) = s.chars().nth(0) {
                    let mut c_str = String::new();
                    c_str.push(c);
                    v.push(vm::Value::String(c.to_string()));
                }
                else {
                    v.push(vm::Value::String(String::new()));
                },
            vm::Value::List(l) => if l.len() > 0 {
                    let e = l.into_iter()
                        .nth(0)
                        .unwrap();
                    v.push(e);
                }
                else {
                    v.push(vm::Value::List(Vec::new()));
                },
            _ => unreachable!(),
        }
        Ok(())
    }
    else {
        Err(format!("argument to `car' function must be listy").into())
    }
}

/// Builtin + function
/// The plus function takes two numbers.
pub fn plus(v: &mut vm::VM) -> Result<()> {
    let right_val = v.pop_value();
    let left_val = v.pop_value();
    if !left_val.is_number() || !right_val.is_number() {
        Err("+ function may only be used on numbers".into())
    }
    else {
        Ok(v.push(vm::Value::Number(left_val.number() + right_val.number())))
    }
}

/// Builtin - function
/// The minus function takes two numbers.
pub fn minus(v: &mut vm::VM) -> Result<()> {
    // TODO : allow using this function to make single expressions negative?
    let right_val = v.pop_value();
    let left_val = v.pop_value();
    if !left_val.is_number() || !right_val.is_number() {
        Err("- function may only be used on numbers".into())
    }
    else {
        Ok(v.push(vm::Value::Number(left_val.number() - right_val.number())))
    }
}

/// Builtin * function
/// The times function takes two numbers.
pub fn times(v: &mut vm::VM) -> Result<()> {
    let right_val = v.pop_value();
    let left_val = v.pop_value();
    if !left_val.is_number() || !right_val.is_number() {
        Err("* function may only be used on numbers".into())
    }
    else {
        Ok(v.push(vm::Value::Number(left_val.number() * right_val.number())))
    }
}

/// Builtin / function
/// The divide function takes two numbers.
pub fn divide(v: &mut vm::VM) -> Result<()> {
    let right_val = v.pop_value();
    let left_val = v.pop_value();
    if !left_val.is_number() || !right_val.is_number() {
        Err("/ function may only be used on numbers".into())
    }
    else {
        Ok(v.push(vm::Value::Number(left_val.number() / right_val.number())))
    }
}

/// Builtin function for opening files.
/// The open function takes a path string and a mode string.
/// Leaves the new file descriptor on the stack.
pub fn rasp_open(v: &mut vm::VM) -> Result<()> {
    let mode_val = v.pop_value();
    let path_val = v.pop_value();
    if !mode_val.is_string() {
        Err("file mode must be a string".into())
    }
    else if !path_val.is_string() {
        Err("file path must be a string".into())
    }
    else {
        // discover file mode
        let mode = mode_val.string();
        let path = path_val.string();
        let open_flags = match mode {
            "r" | "rb" => O_RDONLY,
            "w" | "wb" => O_CREAT | O_TRUNC | O_WRONLY,
            "a" | "ab" => O_CREAT | O_APPEND | O_WRONLY,
            "r+" | "rb+" | "r+b" => O_APPEND | O_RDWR,
            "w+" | "wb+" | "w+b" => O_CREAT | O_TRUNC | O_RDWR,
            "a+" | "ab+" | "a+b" => O_CREAT | O_APPEND | O_RDWR,
            _ => unreachable!(),
        };
        let fd = unsafe {
            open(CString::new(path).unwrap().as_ptr(), open_flags, 0o644)
        };
        Ok(v.push(vm::Value::Number(fd as f64)))
    }
}

/// Builtin function for closing files.
/// The close function takes a file descriptor int.
/// Leaves the close result on the stack.
pub fn rasp_close(v: &mut vm::VM) -> Result<()> {
    let fd_val = v.pop_value();
    if !fd_val.is_number() {
        Err("file descriptor must be a number".into())
    }
    else {
        let fd_num = fd_val.number();
        if fd_num.floor() != fd_num {
            Err("file descriptor must be an integer".into())
        }
        else {
            let fd = fd_num as c_int;
            let result = unsafe {
                close(fd)
            };
            v.push(vm::Value::Number(result as f64));
            Ok(())
        }
    }
}

/// Builtin function for writing to files.
/// The write function takes a file descriptor and a buffer to write.
/// Leaves the write result on the stack.
pub fn rasp_write(v: &mut vm::VM) -> Result<()> {
    let buffer_val = v.pop_value();
    let fd_val = v.pop_value();
    if !buffer_val.is_string() {
        Err("buffer must be a string".into())
    }
    else if !fd_val.is_number() {
        Err("file descriptor must be a number".into())
    }
    else {
        let fd_num = fd_val.number();
        if fd_num.floor() != fd_num {
            Err("file descriptor must be an integer".into())
        }
        else if fd_num.is_sign_negative() {
            Err("file descriptor must be positive".into())
        }
        else {
            let fd = fd_num as c_int;
            let buffer = buffer_val.string();
            let result = unsafe {
                let buffer_cstr = CString::new(buffer)
                    .unwrap();
                write(fd, buffer_cstr.as_ptr() as *const c_void, buffer.len() + 1)
            };
            v.push(vm::Value::Number(result as f64));
            Ok(())
        }
    }
}

/// Builtin function for reading from files.
/// The read function takes a file descriptor and the number of characters to read.
/// Leaves a list of the result and the contents on the stack.
pub fn rasp_read(v: &mut vm::VM) -> Result<()> {
    let count_val = v.pop_value();
    let fd_val = v.pop_value();
    if !count_val.is_number() {
        Err("count must be a number ".into())
    }
    else if !fd_val.is_number() {
        Err("file descriptor must be a number".into())
    }
    else {
        let fd_num = fd_val.number();
        let count_num = count_val.number();
        if fd_num.floor() != fd_num {
            Err("file descriptor must be an integer".into())
        }
        else if fd_num.is_sign_negative() {
            Err("file descriptor must be positive".into())
        }
        else if count_num.floor() != count_num {
            Err("count must be an integer".into())
        }
        else if count_num.is_sign_negative() {
            Err("count must be positive".into())
        }
        else {
            let fd = fd_num as c_int;
            let count = count_num as usize;
            let mut buffer_vec = Vec::new();
            buffer_vec.resize(count, 0 as u8);
            let buffer_cstr = CString::new(buffer_vec)
                .unwrap();
            let result = unsafe {
                read(fd, buffer_cstr.as_ptr() as *mut c_void, count)
            };
            let result_vec = buffer_cstr.into_bytes()
                .into_iter()
                .map(|x| vm::Value::Number(x as f64))
                .collect();
            v.push(vm::Value::List(vec![
                                   vm::Value::Number(result as f64),
                                   vm::Value::List(result_vec)]));
            Ok(())
        }
    }
}
