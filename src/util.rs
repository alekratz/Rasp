use std;
use std::fs::File;
use std::io::prelude::*;

pub fn read_file(path: &str) -> std::io::Result<String> {
    let mut source_text = String::new();
    {
        let mut file = try!(File::open(path));
        try!(file.read_to_string(&mut source_text));
    }
    Ok(source_text)
}
