use std::str::Chars;
use std::fmt;

#[derive(Debug)]
pub enum Token {
    None,
    Eof(Range),
    Lparen(Range),
    Rparen(Range),
    Identifier(Range, String),
    StringLit(Range, String),
    Number(Range, f64),
    Comment(Range, String),
    Unknown(Range, char),
    Error(Range, String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fstr = match self {
            &Token::None => String::from("none"),
            &Token::Eof(_) => String::from("EOF"),
            &Token::Lparen(_) => String::from("left paren"),
            &Token::Rparen(_) => String::from("right paren"),
            &Token::Identifier(_, ref s) => format!("{}", s),
            &Token::StringLit(_, _) => String::from("string literal"),
            &Token::Number(_, _) => String::from("number"),
            &Token::Comment(_, ref s) => String::from("comment"),
            &Token::Unknown(_, ref c) => format!("unknown character `{}'", c),
            &Token::Error(_, ref e) => format!("syntax error: {}", e),
        };
        write!(f, "{}", &fstr)
    }
}

impl Token {
    pub fn is_rparen(&self) -> bool {
        match self {
            &Token::Rparen(_) => true,
            _ => false,
        }
    }

    /*
    pub fn is_lparen(&self) -> bool {
        match self {
            &Token::Lparen(_) => true,
            _ => false,
        }
    }
    */

    pub fn range(&self) -> Range {
        match self {
            &Token::Lparen(r) => r,
            &Token::Rparen(r) => r,
            &Token::Identifier(r, _) => r,
            &Token::StringLit(r, _) => r,
            &Token::Number(r, _) => r,
            &Token::Comment(r, _) => r,
            &Token::Unknown(r, _) => r,
            &Token::Error(r, _) => r,
            &Token::Eof(r) => r,
            _ => Range::new(Pos::start(), Pos::start()),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Pos {
    src_index: i64,
    line_index: i64,
    col_index: i64,
}

impl Pos {

    /// Creates a new `Pos` object.
    pub fn new(src_index: i64, line_index: i64, col_index: i64) -> Pos {
        Pos {
            src_index: src_index,
            line_index: line_index,
            col_index: col_index,
        }
    }

    /// Creates a new `Pos` object that is at the start of a file.
    pub fn start() -> Pos {
        Pos::new(-1, 0, -1)
    }

    /// Advances the position by one character.
    /// Increments the src_index by 1 and col_index by 1.
    pub fn advance(&mut self) {
        self.src_index += 1;
        self.col_index += 1;
    }

    /// Advances the position by a line.
    /// Sets the col_index to -1
    /// Increments the line index by 1
    pub fn line(&mut self) {
        self.col_index = -1;
        self.line_index += 1;
    }
}

impl fmt::Display for Pos {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.line_index + 1, self.col_index + 1)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Range {
    pub start: Pos,
    pub end: Pos,
}

impl Range {
    pub fn new(start: Pos, end: Pos) -> Range {
        Range { start: start, end: end }
    }

    pub fn end_advance(&mut self) {
        self.end.advance();
    }

    pub fn end_line(&mut self) {
        self.end.line();
    }

    pub fn catchup(&mut self) {
        self.start = self.end;
    }
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.start == self.end {
            write!(f, "{}", self.end)
        }
        else {
            write!(f, "{}-{}", self.start, self.end)
        }
    }
}

pub struct Lexer<'a> {
    pub range: Range,
    //source_text: &'a str,
    source_iter: Chars<'a>,
    curr: Option<char>,
    peek: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(source_text: &'a str) -> Lexer{
        let mut lexer = Lexer {
            range: Range::new(Pos::start(), Pos::start()),
            //source_text: source_text,
            source_iter: source_text.chars(),
            curr: None,
            peek: None,
        };
        /*
        lexer.range
            .start
            .advance();
        */
        lexer.peek = lexer.source_iter.next();
        lexer
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        self.next();
        if let Some(c) = self.curr {
            let tok = match c {
                ';' => Token::Comment(self.range, self.eat_comment()),
                '(' => { 
                    self.range.catchup();
                    Token::Lparen(self.range)
                },
                ')' => Token::Rparen(self.range),
                    /* this range includes all printable characters minus lparen, rparen, dquote, and decimals */
                '*' ... '/' | ':' ... '~' | '!' | '#' ... '\'' =>
                    Token::Identifier(self.range, self.eat_identifier()),
                '"' => match self.eat_string() {
                        Ok(s) => Token::StringLit(self.range, s),
                        Err(e) => Token::Error(self.range, e),
                    },
                '0' ... '9' => match self.eat_number() {
                    Ok(n) => Token::Number(self.range, n),
                    Err(e) => Token::Error(self.range, e),
                },
                u => Token::Unknown(self.range, u),
            };
            self.range.catchup();
            tok
        }
        else {
            self.range.catchup();
            Token::Eof(self.range)
        }
    }

    fn eat_comment(&mut self) -> String {
        let mut comment_food = String::new();
        loop {
            self.next();
            match self.curr {
                Some('\n') => break,
                None => break,
                Some(c) => comment_food.push(c),
            }
        }
        comment_food
    }

    fn eat_identifier(&mut self) -> String {
        trace!("Eating identifier");
        let mut identifier = String::new();
        loop {
            identifier.push(self.curr
                                .expect("self.curr was EOF when it was detected not to be"));
            if let Some(p) = self.peek {
                match p {
                    '*' ... '~' | '!' | '#' ... '\'' => self.next(),
                    _ => break,
                }

            }
            else {
                // EOF reached
                break;
            }
        }
        trace!("Got {}", identifier);
        identifier
    }

    fn eat_string(&mut self) -> Result<String, String> {
        let mut string_lit = String::new();
        //let mut escape = false;
        loop {
            self.next();
            match self.curr {
                Some('"') => break,
                Some('\\') => {
                    self.next();
                    match self.curr {
                        Some('r') => string_lit.push('\r'),
                        Some('n') => string_lit.push('\n'),
                        Some('t') => string_lit.push('\t'),
                        Some(c) => return Err(format!("unknown escape sequence: \\{}", c)),
                        None => return Err(String::from("reached EOF before end of string")),
                    }
                },
                Some(c) => string_lit.push(c),
                None => return Err(String::from("reached EOF before end of string")),
            }
        }
        Ok(string_lit)
    }

    fn eat_number(&mut self) -> Result<f64, String> {
        trace!("eating number");
        let mut num_str = String::new();
        let mut decimal = false;
        loop {
            num_str.push(self.curr
                             .expect("self.curr was EOF when it was detected not to be"));
            if let Some(c) = self.curr {
                match c {
                    '0' ... '9' => if let Some(p) = self.peek {
                        match p {
                            '0' ... '9' | '.' => { },
                            ' ' | '\t' | '\r' | '\n' | '(' | ')' => break,
                            u => return Err(format!("unexpected character while parsing number: {}", u)),
                        }
                    },
                    '.' => {
                        if decimal {
                            return Err(String::from("decimal specified twice in number"));
                        }
                        else if let Some(p) = self.peek {
                            match p {
                                '0' ... '9' => decimal = true,
                                u => return Err(format!("unexpected character while parsing number: {}", u)),
                            }
                        }
                        else {
                            return Err(String::from("EOF reached before end of number"));
                        }
                    },
                    // suffix chars
                    //'a' ... 'z' | 'A' ... 'Z' | '_' => break,
                    _ => break,
                }
            }
            else {
                // EOF
                break;
            }
            self.next();
        }

        /*
        if let Some(c) = self.curr {
            match c {
                'a' ... 'z' | 'A' ... 'Z' | '_' => {
                    // suffix
                },
                ' ' | '\n' | '\r' | '\t' => { }, // no-op
                _ => return Err("Invalid number suffix specified; may only be _ or alpha characters".to_string()),
            }
        }
        */
        Ok(num_str.parse().unwrap())
    }

    pub fn skip_whitespace(&mut self) {
        loop {
            if let Some(c) = self.peek {
                match c {
                    ' ' | '\t' | '\r' | '\n' => self.next(),
                    _ => { 
                        break;
                    },
                }
            }
            else {
                break;
            }
        }
        self.range.catchup();
    }

    fn next(&mut self) {
        self.range.end_advance();
        self.curr = self.peek;
        self.peek = self.source_iter.next();
        match self.curr {
            Some('\n') => self.range.end_line(),
            _ => { }
        }
    }
}
