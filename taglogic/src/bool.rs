use std::collections::VecDeque;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum BinaryOp {
    And,
    Or,
}

impl BinaryOp {
    pub fn as_char(self) -> char {
        match self {
            Self::And => '&',
            Self::Or => '|',
        }
    }
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '&' => Some(Self::And),
            '|' => Some(Self::Or),
            _ => None,
        }
    }
    pub fn from_text(text: &str) -> Option<Self> {
        match text {
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    OpenBracket,
    CloseBracket,
    Invert,
    Name { text: String },
    BinaryOp(BinaryOp),
}

fn lex(s: &str) -> Result<Vec<Token>, &'static str> {
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum ParseState {
        AnyExpected,
        InName,
        /// Currently in a binary operation repersented with symbols instead of words.
        InSymbolBinOp(BinaryOp),
    }

    let mut state = ParseState::AnyExpected;
    let mut tokens = Vec::new();
    let mut cur_name = String::new();
    for c in s.chars() {
        if let ParseState::InSymbolBinOp(op) = state {
            state = ParseState::AnyExpected;
            if c == op.as_char() {
                // continuning the last bin op (| and || are treated the same)
                continue;
            }
        }

        if state == ParseState::InName {
            let end_cur_token = match c {
                '(' | ')' | '&' | '|' | '!' => true,
                _ if c.is_whitespace() => true,
                _ => false,
            };
            if end_cur_token {
                let lower = cur_name.to_ascii_lowercase();
                if let Some(op) = BinaryOp::from_text(&lower) {
                    tokens.push(Token::BinaryOp(op));
                } else {
                    tokens.push(Token::Name { text: cur_name });
                }
                cur_name = String::new();
                state = ParseState::AnyExpected;
            } else {
                cur_name.push(c);
            }
        }

        if state == ParseState::AnyExpected {
            match c {
                '(' => tokens.push(Token::OpenBracket),
                ')' => tokens.push(Token::CloseBracket),
                '!' => tokens.push(Token::Invert),
                '&' => {
                    tokens.push(Token::BinaryOp(BinaryOp::And));
                    state = ParseState::InSymbolBinOp(BinaryOp::And);
                },
                '|' => {
                    tokens.push(Token::BinaryOp(BinaryOp::Or));
                    state = ParseState::InSymbolBinOp(BinaryOp::Or);
                },
                // ignore whitespace
                _ if c.is_whitespace() => {},
                _ => {
                    state = ParseState::InName;
                    cur_name = String::with_capacity(1);
                    cur_name.push(c);
                }
            }
        }
    }
    Ok(tokens)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AstNode {
    Invert(Box<AstNode>),
    Binary(BinaryOp, Box<AstNode>, Box<AstNode>),
    Name(String),
}

impl AstNode {
    fn munch_tokens(tokens: &mut VecDeque<Token>) -> Result<Self, &'static str> {
        loop {
            let next = match tokens.get(0) {
                Some(x) => x,
                None => return Err("unexpected end of expression"),
            };
            match next {
                Token::CloseBracket => return Err("Unexpected closing bracket"),
                Token::Invert => {
                    tokens.remove(0);
                    // invert exactly the next token
                    // !a & b -> (!a) & b
                    match tokens.get(1) {
                        Some(Token::OpenBracket) => {
                            return Ok(AstNode::Invert(Box::new(Self::munch_tokens(tokens)?)));
                        },
                        Some(Token::Name { text }) => {
                            // is it like "!abc" or "!abc & xyz"
                            let inverted = AstNode::Invert(Box::new(AstNode::Name(text.clone())));
                            match tokens.get(2) {
                                Some(Token::BinaryOp(op)) => {
                                    // "!abc & xyz"
                                    // convert to unambiguous form and try again
                                    // 1 token for invert, 1 for name makes 2
                                    tokens.insert(2, Token::CloseBracket);
                                    tokens.insert(0, Token::OpenBracket);
                                    return Self::munch_tokens(tokens);
                                }
                                None | Some(Token::CloseBracket) => {
                                    // "!abc"
                                    tokens.remove(0); // will return None if empty, that is okay
                                    return Ok(inverted);
                                 }
                                Some(_) => return Err("invalid token after inverted name"),
                            }
                        }
                        Some(Token::Invert) => return Err("can't double invert, that would be pointless"),
                        Some(_) => return Err("expected expression"),
                        None => return Err("Expected token to invert, got EOF"),
                    }
                },
                Token::OpenBracket => {
                    tokens.remove(0); // open bracket
                    let result = Self::munch_tokens(tokens)?;
                    match tokens.remove(0) {
                        Some(Token::CloseBracket) => {},
                        _ => return Err("expected closing bracket"),
                    };
                    // check for binary op afterwards
                    return match tokens.get(0) {
                        Some(Token::BinaryOp(op)) => {
                            let ret = Ok(AstNode::Binary(op.clone(), Box::new(result), Box::new(Self::munch_tokens(tokens)?)));
                            tokens.remove(0);
                            ret
                        }
                        Some(Token::CloseBracket) | None => Ok(result),
                        Some(_) => Err("invald token after closing bracket"),
                    };
                },
                Token::BinaryOp(_) => return Err("Unexpected binary operator"),
                Token::Name { text } => {
                    // could be the start of the binary op or just a lone name
                    match tokens.get(1) {
                        Some(Token::BinaryOp(op)) => {
                            // convert to unambiguous form and try again
                            tokens.insert(1, Token::CloseBracket);
                            tokens.insert(0, Token::OpenBracket);
                            return Self::munch_tokens(tokens);
                        }
                        Some(Token::CloseBracket) | None => {
                            // lone token
                            let text = text.clone();
                            tokens.remove(0);
                            return Ok(AstNode::Name(text));
                        }
                        Some(_) => return Err("name followed by invalid token"),
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExprData {
    Empty,
    HasNodes(AstNode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expr(ExprData); // wrap internal implementation details

impl Expr {
    pub fn from_string(s: &str) -> Result<Self, &'static str> {
        let mut tokens: VecDeque<Token> = lex(s)?.into_iter().collect();
        if tokens.is_empty() {
            return Ok(Self(ExprData::Empty));
        }
        let ast = AstNode::munch_tokens(&mut tokens)?;
        if !tokens.is_empty() {
            return Err("expected EOF, found extra tokens");
        }
        Ok(Self(ExprData::HasNodes(ast)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn nested_lex() {
        let tokens = lex("abc & !(( ! xyz || dwf) | (!abc or dwp) & (dwp and r   ) )  ");
        assert_eq!(tokens, Ok(vec![
            Token::Name { text: "abc".to_string() },
            Token::BinaryOp(BinaryOp::And),
            Token::Invert,
            Token::OpenBracket,
            Token::OpenBracket,
            Token::Invert,
            Token::Name { text: "xyz".to_string() },
            Token::BinaryOp(BinaryOp::Or),
            Token::Name { text: "dwf".to_string() },
            Token::CloseBracket,
            Token::BinaryOp(BinaryOp::Or),
            Token::OpenBracket,
            Token::Invert,
            Token::Name { text: "abc".to_string() },
            Token::BinaryOp(BinaryOp::Or),
            Token::Name { text: "dwp".to_string() },
            Token::CloseBracket,
            Token::BinaryOp(BinaryOp::And),
            Token::OpenBracket,
            Token::Name { text: "dwp".to_string() },
            Token::BinaryOp(BinaryOp::And),
            Token::Name { text: "r".to_string() },
            Token::CloseBracket,
            Token::CloseBracket,
        ]));
    }
}
