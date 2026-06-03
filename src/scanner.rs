use crate::token::{Token, TokenType};

pub struct Scanner {
    start: *const u8,
    current: *const u8,
    line: usize,
}

impl Scanner {
    pub fn new(source: *const u8) -> Self {
        Self {
            start: source,
            current: source,
            line: 1,
        }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn scan_token(&mut self) -> Token {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            // ← проверяем ДО advance
            return self.make_token(TokenType::EOF);
        }

        let c = self.advance();

        if Self::is_alpha(c) {
            return self.identifier();
        }

        if Self::is_digit(c) {
            return self.number();
        }

        match c {
            b'(' => self.make_token(TokenType::LEFT_PAREN),
            b')' => self.make_token(TokenType::RIGHT_PAREN),
            b'{' => self.make_token(TokenType::LEFT_BRACE),
            b'}' => self.make_token(TokenType::RIGHT_BRACE),
            b';' => self.make_token(TokenType::SEMICOLON),
            b',' => self.make_token(TokenType::COMMA),
            b'.' => self.make_token(TokenType::DOT),
            b'-' => self.make_token(TokenType::MINUS),
            b'+' => self.make_token(TokenType::PLUS),
            b'/' => self.make_token(TokenType::SLASH),
            b'*' => self.make_token(TokenType::STAR),

            b'!' => {
                let token_type = if self.matches(b'=') {
                    TokenType::BANG_EQUAL
                } else {
                    TokenType::BANG
                };

                self.make_token(token_type)
            }

            b'=' => {
                let token_type = if self.matches(b'=') {
                    TokenType::EQUAL_EQUAL
                } else {
                    TokenType::EQUAL
                };

                self.make_token(token_type)
            }

            b'<' => {
                let token_type = if self.matches(b'=') {
                    TokenType::LESS_EQUAL
                } else {
                    TokenType::LESS
                };

                self.make_token(token_type)
            }

            b'>' => {
                let token_type = if self.matches(b'=') {
                    TokenType::GREATER_EQUAL
                } else {
                    TokenType::GREATER
                };

                self.make_token(token_type)
            }

            b'"' => self.string(),

            _ => self.error_token("Unexpected character."),
        }
    }

    #[inline(always)]
    fn is_alpha(c: u8) -> bool {
        matches!(c, b'a'..b'z' | b'A'..b'Z' | b'_')
    }

    #[inline(always)]
    fn is_digit(c: u8) -> bool {
        matches!(c, b'0'..b'9')
    }

    fn identifier(&mut self) -> Token {
        while Self::is_alpha(self.peek()) || Self::is_digit(self.peek()) {
            self.advance();
        }

        self.make_token(self.identifier_type())
    }

    fn identifier_type(&self) -> TokenType {
        match unsafe { *self.start } {
            b'a' => self.check_keyword(1, 2, b"nd", TokenType::AND),
            b'c' => self.check_keyword(1, 4, b"lass", TokenType::CLASS),
            b'e' => self.check_keyword(1, 3, b"lse", TokenType::ELSE),
            b'f' if unsafe { self.current.offset_from(self.start) } > 1 => {
                match unsafe { *self.start.add(1) } {
                    b'a' => self.check_keyword(2, 3, b"lse", TokenType::FALSE),
                    b'o' => self.check_keyword(2, 1, b"r", TokenType::FOR),
                    b'u' => self.check_keyword(2, 1, b"n", TokenType::FUN),
                    _ => TokenType::IDENTIFIER,
                }
            }
            b'i' => self.check_keyword(1, 1, b"f", TokenType::IF),
            b'n' => self.check_keyword(1, 2, b"il", TokenType::NIL),
            b'o' => self.check_keyword(1, 1, b"r", TokenType::OR),
            b'p' => self.check_keyword(1, 4, b"rint", TokenType::PRINT),
            b'r' => self.check_keyword(1, 5, b"eturn", TokenType::RETURN),
            b's' => self.check_keyword(1, 4, b"uper", TokenType::SUPER),
            b't' if unsafe { self.current.offset_from(self.start) } > 1 => {
                match unsafe { *self.start.add(1) } {
                    b'h' => self.check_keyword(2, 2, b"is", TokenType::THIS),
                    b'r' => self.check_keyword(2, 2, b"ue", TokenType::TRUE),
                    _ => TokenType::IDENTIFIER,
                }
            }
            b'v' => self.check_keyword(1, 2, b"ar", TokenType::VAR),
            b'w' => self.check_keyword(1, 4, b"hile", TokenType::WHILE),

            _ => TokenType::IDENTIFIER,
        }
    }

    unsafe fn memcmp(ptr1: *const u8, ptr2: *const u8, n: usize) -> i32 {
        for i in 0..n {
            if *ptr1.add(i) != *ptr2.add(i) {
                return (*ptr1.add(i) as i32) - (*ptr2.add(i) as i32);
            }
        }
        0
    }

    fn check_keyword(
        &self,
        start: usize,
        length: usize,
        rest: &'static [u8],
        typ: TokenType,
    ) -> TokenType {
        if unsafe {
            self.current.offset_from(self.start) as usize == start + length
                && Self::memcmp(self.start.add(start), rest.as_ptr(), length) == 0
        } {
            return typ;
        }

        TokenType::IDENTIFIER
    }

    fn number(&mut self) -> Token {
        while Self::is_digit(self.peek()) {
            self.advance();
        }

        if self.peek() == b'.' && Self::is_digit(self.peek_next()) {
            // consume the '.'
            self.advance();

            while Self::is_digit(self.peek()) {
                self.advance();
            }
        }

        self.make_token(TokenType::NUMBER)
    }

    fn string(&mut self) -> Token {
        while self.peek() != b'"' && !self.is_at_end() {
            if self.peek() == b'\n' {
                self.line += 1;
            }
            self.advance();
        }

        if self.is_at_end() {
            self.error_token("Unterminated string.")
        } else {
            self.advance();
            self.make_token(TokenType::STRING)
        }
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                b' ' | b'\r' | b'\t' => {
                    self.advance();
                }
                b'\n' => {
                    self.line += 1;
                    self.advance();
                }
                b'/' => {
                    if self.peek_next() == b'/' {
                        // A comment goes until the end of the line.
                        while self.peek() != b'\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    #[inline(always)]
    fn is_at_end(&self) -> bool {
        unsafe { *self.current == 0 }
    }

    #[inline(always)]
    fn peek(&self) -> u8 {
        unsafe { *self.current }
    }

    #[inline(always)]
    fn peek_next(&self) -> u8 {
        if self.is_at_end() {
            b'\0'
        } else {
            unsafe { *self.current.add(1) }
        }
    }

    #[inline(always)]
    fn matches(&mut self, expected: u8) -> bool {
        if self.is_at_end() {
            return false;
        }

        if unsafe { *self.current } != expected {
            return false;
        }

        self.current = unsafe { self.current.add(1) };

        true
    }

    #[inline(always)]
    fn advance(&mut self) -> u8 {
        let c = unsafe { *self.current };
        self.current = unsafe { self.current.add(1) };
        c
    }

    #[inline(always)]
    fn error_token(&self, msg: &'static str) -> Token {
        Token {
            typ: TokenType::ERROR,
            start: msg.as_ptr(),
            length: msg.len(),
            line: self.line,
        }
    }

    #[inline(always)]
    fn make_token(&self, typ: TokenType) -> Token {
        Token {
            typ,
            start: self.start,
            length: unsafe { self.current.offset_from(self.start) } as usize,
            line: self.line,
        }
    }
}
