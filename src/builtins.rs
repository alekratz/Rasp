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
        map
    };
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
        v.push(vm::Value::Number(fd as f64));
        Ok(())
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
            let result = unsafe {
                let mut buffer_cstr = CString::new(buffer_vec)
                    .unwrap();
                read(fd, buffer_cstr.as_ptr() as *mut c_void, count)
            };
            Ok(())
        }
    }
}
