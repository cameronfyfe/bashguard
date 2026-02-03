use anyhow::{bail, Result};

// TODO: evaluate existing OSS lexers/parsers to swap in here

/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// A word (command, argument, etc.)
    Word(String),
    /// Pipe operator |
    Pipe,
    /// Output redirect >
    RedirectOut,
    /// Append redirect >>
    RedirectAppend,
    /// Input redirect <
    RedirectIn,
    /// Background operator &
    Background,
    /// Command separator ;
    Semicolon,
    /// Logical AND &&
    And,
    /// Logical OR ||
    Or,
    /// Environment variable assignment (key=value before command)
    EnvVar(String, String),
}

pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while self.pos < self.input.len() {
            self.skip_whitespace();

            if self.pos >= self.input.len() {
                break;
            }

            let c = self.current_char();

            match c {
                '|' => {
                    if self.peek_char() == Some('|') {
                        self.pos += 2;
                        tokens.push(Token::Or);
                    } else {
                        self.pos += 1;
                        tokens.push(Token::Pipe);
                    }
                }
                '>' => {
                    if self.peek_char() == Some('>') {
                        self.pos += 2;
                        tokens.push(Token::RedirectAppend);
                    } else {
                        self.pos += 1;
                        tokens.push(Token::RedirectOut);
                    }
                }
                '<' => {
                    self.pos += 1;
                    tokens.push(Token::RedirectIn);
                }
                '&' => {
                    if self.peek_char() == Some('&') {
                        self.pos += 2;
                        tokens.push(Token::And);
                    } else {
                        self.pos += 1;
                        tokens.push(Token::Background);
                    }
                }
                ';' => {
                    self.pos += 1;
                    tokens.push(Token::Semicolon);
                }
                '"' => {
                    let word = self.read_double_quoted()?;
                    tokens.push(Token::Word(word));
                }
                '\'' => {
                    let word = self.read_single_quoted()?;
                    tokens.push(Token::Word(word));
                }
                _ => {
                    let word = self.read_word();
                    if word.is_empty() {
                        continue;
                    }

                    // Check if this is an env var assignment (only before command)
                    if tokens.iter().all(|t| matches!(t, Token::EnvVar(_, _)))
                        && word.contains('=')
                        && !word.starts_with('=')
                    {
                        let parts: Vec<&str> = word.splitn(2, '=').collect();
                        if parts.len() == 2 && is_valid_var_name(parts[0]) {
                            tokens.push(Token::EnvVar(parts[0].to_string(), parts[1].to_string()));
                            continue;
                        }
                    }

                    tokens.push(Token::Word(word));
                }
            }
        }

        Ok(tokens)
    }

    fn current_char(&self) -> char {
        self.input[self.pos..].chars().next().unwrap()
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().nth(1)
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            let c = self.current_char();
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();

        while self.pos < self.input.len() {
            let c = self.current_char();

            match c {
                ' ' | '\t' | '\n' | '|' | '>' | '<' | '&' | ';' => break,
                '\\' => {
                    self.pos += 1;
                    if self.pos < self.input.len() {
                        word.push(self.current_char());
                        self.pos += self.current_char().len_utf8();
                    }
                }
                _ => {
                    word.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }

        word
    }

    fn read_double_quoted(&mut self) -> Result<String> {
        self.pos += 1; // Skip opening quote
        let mut word = String::new();

        while self.pos < self.input.len() {
            let c = self.current_char();

            match c {
                '"' => {
                    self.pos += 1;
                    return Ok(word);
                }
                '\\' => {
                    self.pos += 1;
                    if self.pos < self.input.len() {
                        let escaped = self.current_char();
                        match escaped {
                            '"' | '\\' | '$' | '`' => word.push(escaped),
                            'n' => word.push('\n'),
                            't' => word.push('\t'),
                            _ => {
                                word.push('\\');
                                word.push(escaped);
                            }
                        }
                        self.pos += escaped.len_utf8();
                    }
                }
                _ => {
                    word.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }

        bail!("Unterminated string")
    }

    fn read_single_quoted(&mut self) -> Result<String> {
        self.pos += 1; // Skip opening quote
        let mut word = String::new();

        while self.pos < self.input.len() {
            let c = self.current_char();

            if c == '\'' {
                self.pos += 1;
                return Ok(word);
            }

            word.push(c);
            self.pos += c.len_utf8();
        }

        bail!("Unterminated string")
    }
}

fn is_valid_var_name(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_command() {
        let mut lexer = Lexer::new("git status");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![Token::Word("git".into()), Token::Word("status".into())]
        );
    }

    #[test]
    fn test_pipe() {
        let mut lexer = Lexer::new("ls | grep foo");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("ls".into()),
                Token::Pipe,
                Token::Word("grep".into()),
                Token::Word("foo".into())
            ]
        );
    }

    #[test]
    fn test_quoted_string() {
        let mut lexer = Lexer::new(r#"echo "hello world""#);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".into()),
                Token::Word("hello world".into())
            ]
        );
    }

    #[test]
    fn test_env_var() {
        let mut lexer = Lexer::new("FOO=bar npm run");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::EnvVar("FOO".into(), "bar".into()),
                Token::Word("npm".into()),
                Token::Word("run".into())
            ]
        );
    }

    #[test]
    fn test_redirect() {
        let mut lexer = Lexer::new("echo hello > file.txt");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("echo".into()),
                Token::Word("hello".into()),
                Token::RedirectOut,
                Token::Word("file.txt".into())
            ]
        );
    }

    #[test]
    fn test_and_or() {
        let mut lexer = Lexer::new("cmd1 && cmd2 || cmd3");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Word("cmd1".into()),
                Token::And,
                Token::Word("cmd2".into()),
                Token::Or,
                Token::Word("cmd3".into())
            ]
        );
    }
}
