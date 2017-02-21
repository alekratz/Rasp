use lexer::{Lexer, Token, Range};
use ast::AST;
use errors::*;

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_tok: Token,
}

fn parse_error(pos: &Range, message: &str) -> String {
    format!("{}: {}", pos, message)
}

impl<'a> Parser<'a> {
    pub fn new(lexer: Lexer<'a>) -> Parser<'a> {
        Parser {
            lexer: lexer,
            current_tok: Token::None,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<AST>> {
        self.next();
        let mut ast = Vec::new();
        loop {
            match self.current_tok {
                Token::Identifier(r, _) | Token::StringLit(r, _) | Token::Lparen(r) | Token::Number(r, _) => {
                    let expr_result = self.expr();
                    if expr_result.is_err() {
                        let start = r.start;
                        let end = self.range().end;
                        if start == end {
                            expr_result.chain_err(|| format!("expression at {}", Range::new(start, end)))?;
                        }
                        else {
                            expr_result.chain_err(|| format!("expression spanning {}", Range::new(start, end)))?;
                        }
                    }
                    else {
                        ast.push(expr_result.unwrap());
                    }
                },
                Token::Comment(_, _) => self.next(),
                Token::Eof(_) => break,
                Token::Unknown(r, _) => return Err(parse_error(&r,
                    &self.unexpected_token("left paren, identifier, string literal, or comment")).into()),
                Token::Error(r, ref s) =>
                    return Err(parse_error(&r, &format!("lexer error: {}", s)).into()),
                Token::None => unreachable!(),
                ref t => return Err(parse_error(&t.range(),
                    &self.unexpected_token("left paren, identifier, string literal, or comment")).into()),
            }
        }
        Ok(ast)
    }

    fn expr(&mut self) -> Result<AST> {
        if !self.is_expr_start() {
            return Err(parse_error(&self.current_tok.range(),
                &self.unexpected_token("left paren, identifier, number, or string literal")).into())
        }

        let start = self.lexer
                        .range
                        .start;
        let expr = match self.current_tok {
            Token::Identifier(r, ref id) => AST::Identifier(r, id.clone()),
            Token::StringLit(r, ref s_lit) => AST::StringLit(r, s_lit.clone()),
            Token::Number(r, ref num) => AST::Number(r, *num),
            Token::Lparen(_) => {
                let mut exprs = Vec::new();
                self.next();
                // the next token may not be an expression start; it may just be an rparen
                while self.is_expr_start() {
                    let expr_result = self.expr();
                    if expr_result.is_err() {
                        expr_result.chain_err(|| "invalid expression")?;
                    }
                    else {
                        exprs.push(expr_result.unwrap());
                    }
                }

                if let Token::Error(r, ref s) = self.current_tok {
                    return Err(s.as_str()
                                .into());
                }
                else if let Token::Unknown(r, c) = self.current_tok {
                    return Err(parse_error(&r, &format!("syntax error: unexpected character {}", c)).into())
                }
                else if !self.current_tok.is_rparen() {
                    return Err(self.unexpected_token(
                            "left paren, identifier, string literal, number, or right paren").into());
                }

                let end = self.lexer
                    .range
                    .end;
                
                let range = Range::new(start, end);
                AST::Expr(range, exprs)
            },
            _ => unreachable!(),
        };
        self.next();
        Ok(expr)
    }

    /// Gets whether the current character is an expression start
    fn is_expr_start(&self) -> bool {
        match self.current_tok {
            Token::Lparen(_) | Token::Identifier(_,_) | Token::StringLit(_, _) | Token::Number(_, _) => true,
            _ => false,
        }
    }

    fn range(&self) -> &Range {
        &self.lexer
            .range
    }

    fn unexpected_token(&self, expected: &'static str) -> String {
        format!("unexpected {} at {}: expected {}", self.current_tok, self.current_tok.range(), expected)
    }

    fn next(&mut self) {
        self.current_tok = self.lexer.next_token();
    }
}
