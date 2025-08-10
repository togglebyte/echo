use crate::error::{Error, Result};
use crate::instruction::{Dest, Instruction, Instructions, Source};
use crate::token::{Token, Tokens};

struct Parser<'src> {
    tokens: Tokens<'src>,
}

impl<'src> Parser<'src> {
    fn new(tokens: Tokens<'src>) -> Self {
        Self { tokens }
    }

    fn parse(&mut self) -> Result<Instructions> {
        let mut instructions = vec![];

        loop {
            match self.tokens.current() {
                Token::Newline | Token::Comment | Token::Whitespace => {
                    self.tokens.consume();
                    continue;
                }
                Token::Eof => break,
                _ => (),
            }

            let inst = self.load()?;
            instructions.push(inst);

            match self.tokens.take() {
                Token::Newline | Token::Comment | Token::Whitespace => continue,
                Token::Eof => break,
                token => {
                    return Error::unexpected_token(
                        "newline or end of file",
                        token,
                        self.tokens.spans(),
                        self.tokens.source,
                    );
                }
            }

            // there has to be either newline OR eof here
        }

        Ok(Instructions::new(instructions))
    }

    fn load(&mut self) -> Result<Instruction> {
        if self.tokens.consume_if(Token::Load) {
            match self.tokens.take() {
                Token::Str(path) => match self.tokens.take() {
                    Token::As => match self.tokens.take() {
                        Token::Ident(key) => Ok(Instruction::Load(path.into(), key)),
                        token => return Error::invalid_arg("ident", token, self.tokens.spans(), self.tokens.source),
                    },
                    token => return Error::invalid_arg("as", token, self.tokens.spans(), self.tokens.source),
                },
                token => Error::invalid_arg("string", token, self.tokens.spans(), self.tokens.source),
            }
        } else {
            self.goto()
        }
    }

    fn goto(&mut self) -> Result<Instruction> {
        // goto <ident>|<int> <int>
        if self.tokens.consume_if(Token::Goto) {
            // <ident>
            let instr = match self.tokens.take() {
                Token::Ident(ident) => Instruction::Goto(Dest::Marker(ident)),
                Token::Int(row) => match self.tokens.take() {
                    Token::Int(col) => Instruction::Goto(Dest::Relative {
                        row: row as i32,
                        col: col as i32,
                    }),
                    token => return Error::invalid_arg("number", token, self.tokens.spans(), self.tokens.source),
                },
                token => return Error::invalid_arg("ident", token, self.tokens.spans(), self.tokens.source),
            };

            Ok(instr)
        } else {
            self.print()
        }
    }

    fn print(&mut self) -> Result<Instruction> {
        // print <string>
        if self.tokens.consume_if(Token::Type) {
            let source = match self.tokens.take() {
                Token::Str(s) => Source::Str(s),
                Token::Ident(ident) => Source::Ident(ident),
                token => return Error::invalid_arg("ident", token, self.tokens.spans(), self.tokens.source),
            };

            let trim_trailing_newline = self.tokens.consume_if(Token::NoNewline);
            Ok(Instruction::Type {
                source,
                trim_trailing_newline,
                prefix_newline: false,
            })
        } else {
            self.printnl()
        }
    }

    fn printnl(&mut self) -> Result<Instruction> {
        // printnl <string> <nonl>?
        if self.tokens.consume_if(Token::TypeNl) {
            let source = match self.tokens.take() {
                Token::Str(s) => Source::Str(s),
                Token::Ident(ident) => Source::Ident(ident),
                token => return Error::invalid_arg("ident", token, self.tokens.spans(), self.tokens.source),
            };

            let trim_trailing_newline = self.tokens.consume_if(Token::NoNewline);

            Ok(Instruction::Type {
                source,
                trim_trailing_newline,
                prefix_newline: true,
            })
        } else {
            self.insert()
        }
    }

    fn insert(&mut self) -> Result<Instruction> {
        // insert <string>
        if self.tokens.consume_if(Token::Insert) {
            match self.tokens.take() {
                Token::Str(s) => return Ok(Instruction::Insert(Source::Str(s))),
                Token::Ident(ident) => return Ok(Instruction::Insert(Source::Ident(ident))),
                token => return Error::invalid_arg("ident", token, self.tokens.spans(), self.tokens.source),
            }
        } else {
            self.change()
        }
    }

    fn change(&mut self) -> Result<Instruction> {
        // change <string> <string|ident>
        if self.tokens.consume_if(Token::Replace) {
            // <string>
            let src = match self.tokens.take() {
                Token::Str(string) => string,
                token => return Error::invalid_arg("string", token, self.tokens.spans(), self.tokens.source),
            };

            // <string|ident>
            let replacement = match self.tokens.take() {
                Token::Str(string) => Source::Str(string),
                Token::Ident(ident) => Source::Ident(ident),
                token => return Error::invalid_arg("string or ident", token, self.tokens.spans(), self.tokens.source),
            };

            let instr = Instruction::Replace { src, replacement };
            Ok(instr)
        } else {
            self.delete()
        }
    }

    fn delete(&mut self) -> Result<Instruction> {
        if self.tokens.consume_if(Token::Delete) { Ok(Instruction::Delete) } else { self.speed() }
    }

    fn speed(&mut self) -> Result<Instruction> {
        // speed <int>
        if self.tokens.consume_if(Token::Speed) {
            // <int>
            let instr = match self.tokens.take() {
                Token::Int(speed) => Instruction::Speed(speed as u64),
                token => return Error::invalid_arg("int", token, self.tokens.spans(), self.tokens.source),
            };

            Ok(instr)
        } else {
            self.select()
        }
    }

    fn select(&mut self) -> Result<Instruction> {
        // select <ident>|<int> <int>
        if self.tokens.consume_if(Token::Select) {
            let instr = match self.tokens.take() {
                // Token::Ident(ident) => Instruction::Select(Dest::Marker(ident)),
                Token::Int(width) => match self.tokens.take() {
                    Token::Int(height) => Instruction::Select {
                        width: width as u16,
                        height: height as u16,
                    },
                    token => return Error::invalid_arg("number", token, self.tokens.spans(), self.tokens.source),
                },
                token => return Error::invalid_arg("ident or row", token, self.tokens.spans(), self.tokens.source),
            };

            Ok(instr)
        } else {
            self.find()
        }
    }

    fn find(&mut self) -> Result<Instruction> {
        // find <string>
        if self.tokens.consume_if(Token::Find) {
            let instr = match self.tokens.take() {
                Token::Str(needle) => Instruction::Find(needle),
                token => return Error::invalid_arg("string", token, self.tokens.spans(), self.tokens.source),
            };

            Ok(instr)
        } else {
            self.linepause()
        }
    }

    fn linepause(&mut self) -> Result<Instruction> {
        if self.tokens.consume_if(Token::LinePause) {
            let instr = match self.tokens.take() {
                Token::Int(ms) => Instruction::LinePause(ms as u64),
                token => return Error::invalid_arg("int", token, self.tokens.spans(), self.tokens.source),
            };

            Ok(instr)
        } else {
            self.wait()
        }
    }

    fn wait(&mut self) -> Result<Instruction> {
        // if not wait then error

        match self.tokens.take() {
            Token::Wait => {
                let instr = match self.tokens.take() {
                    Token::Int(seconds) => Instruction::Wait(seconds as u64),
                    token => return Error::invalid_arg("seconds", token, self.tokens.spans(), self.tokens.source),
                };

                Ok(instr)
            }
            token => Error::invalid_instruction(token, self.tokens.spans(), self.tokens.source),
        }
    }
}

pub fn parse(tokens: Tokens<'_>) -> Result<Instructions> {
    Parser::new(tokens).parse()
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::*;
    use crate::lexer::lex;

    fn parse(input: &str) -> Result<Vec<Instruction>> {
        let tokens = lex(input, "//")?;
        super::parse(tokens).map(|i| i.take_instructions())
    }

    fn parse_ok(input: &str) -> Vec<Instruction> {
        parse(input).unwrap()
    }

    // -----------------------------------------------------------------------------
    //   - Util functions -
    // -----------------------------------------------------------------------------
    fn load(path: impl Into<PathBuf>, key: impl Into<String>) -> Instruction {
        let path = path.into();
        let key = key.into();
        Instruction::Load(path, key)
    }

    fn goto(dest: impl Into<Dest>) -> Instruction {
        Instruction::Goto(dest.into())
    }

    fn print_str(s: &str) -> Instruction {
        Instruction::Type {
            source: Source::Str(s.into()),
            trim_trailing_newline: false,
            prefix_newline: false,
        }
    }

    fn print_ident(s: &str) -> Instruction {
        Instruction::Type {
            source: Source::Ident(s.into()),
            trim_trailing_newline: false,
            prefix_newline: false,
        }
    }

    fn replace_str(src: &str, s: &str) -> Instruction {
        let src = src.into();
        Instruction::Replace {
            src,
            replacement: Source::Str(s.into()),
        }
    }

    fn replace_ident(src: &str, s: &str) -> Instruction {
        let src = src.into();
        Instruction::Replace {
            src,
            replacement: Source::Ident(s.into()),
        }
    }

    fn wait(secs: u64) -> Instruction {
        Instruction::Wait(secs)
    }

    #[test]
    fn parse_load() {
        let output = parse_ok("load \"foo.rs\" as hoppy");
        let expected = vec![load("foo.rs", "hoppy")];
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_goto() {
        let output = parse_ok("goto aaa");
        let expected = vec![goto("aaa")];
        assert_eq!(output, expected);

        let output = parse_ok("goto 1, 2");
        let expected = vec![goto((1, 2))];
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_type() {
        let output = parse_ok("type \"a string\"");
        let expected = vec![print_str("a string")];
        assert_eq!(output, expected);

        let output = parse_ok("type aaa");
        let expected = vec![print_ident("aaa")];
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_replace() {
        let output = parse_ok("replace \"a\" \"b\"");
        let expected = vec![replace_str("a", "b")];
        assert_eq!(output, expected);

        let output = parse_ok("replace \"a\" b");
        let expected = vec![replace_ident("a", "b")];
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_wait() {
        let output = parse_ok("wait 123");
        let expected = vec![wait(123)];
        assert_eq!(output, expected);
    }

    #[test]
    fn parse_goto_negatives() {
        let output = parse_ok("goto -1 -2");
        let expected = vec![goto((-1, -2))];
        assert_eq!(output, expected);
    }

    #[test]
    fn multi_lines() {
        let output = parse_ok(
            "

        //
goto 1     2
        //
            wait 1
            // waffles
            wait 2
            // waffles
            ",
        );
        let expected = vec![goto((1, 2)), wait(1), wait(2)];
        assert_eq!(output, expected);
    }
}
