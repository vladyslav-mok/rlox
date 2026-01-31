use crate::chunk::{Chunk, OpCode};
use crate::scanner::{Scanner, Token, TokenType};
use crate::value::{Function, Obj, StringInterner, Value};
use crate::vm;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FunctionType {
    Function,
    Initializer,
    Method,
    Script,
}

#[derive(Debug)]
struct FunctionCompiler<'a> {
    enclosing: Option<Box<FunctionCompiler<'a>>>,
    function: Function,
    function_type: FunctionType,
    locals: Vec<Local<'a>>,
    upvalues: Vec<Upvalue>,
    scope_depth: usize,
}

#[derive(Debug, Clone)]
struct Local<'a> {
    name: &'a str,
    depth: Option<usize>,
    is_captured: bool,
}

#[derive(Debug, Clone, Copy)]
struct Upvalue {
    index: u8,
    is_local: bool,
}

#[derive(Debug)]
struct ClassCompiler {
    enclosing: Option<Box<ClassCompiler>>,
    has_superclass: bool,
}

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    parser: Parser<'a>,
    current: Option<Box<FunctionCompiler<'a>>>,
    current_class: Option<Box<ClassCompiler>>,
    interner: StringInterner,
}

#[derive(Debug)]
struct Parser<'a> {
    current: Option<Token<'a>>,
    previous: Option<Token<'a>>,
    had_error: bool,
    panic_mode: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    None,
    Assignment, // =
    Or,         // or
    And,        // and
    Equality,   // == !=
    Comparison, // < > <= >=
    Term,       // + -
    Factor,     // * /
    Unary,      // ! -
    Call,       // . ()
    Primary,
}

impl Precedence {
    fn next(&self) -> Self {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

type ParseFn<'a> = for<'b> fn(&'b mut Compiler<'a>, bool);

struct ParseRule<'a> {
    prefix: Option<ParseFn<'a>>,
    infix: Option<ParseFn<'a>>,
    precedence: Precedence,
}

impl<'a> Compiler<'a> {
    pub fn compile(source: &'a str) -> Result<Rc<Function>, ()> {
        let scanner = Scanner::new(source);
        let parser = Parser {
            current: None,
            previous: None,
            had_error: false,
            panic_mode: false,
        };

        let mut state = Compiler {
            scanner,
            parser,
            current: None,
            current_class: None,
            interner: StringInterner::new(),
        };

        let mut compiler = FunctionCompiler {
            enclosing: None,
            function: Function::new(),
            function_type: FunctionType::Script,
            locals: Vec::with_capacity(vm::U8_COUNT),
            upvalues: Vec::with_capacity(vm::U8_COUNT),
            scope_depth: 0,
        };
        compiler.locals.push(Local {
            name: "",
            depth: Some(0),
            is_captured: false,
        });

        state.current = Some(Box::new(compiler));
        state.advance();

        while !state.match_token(TokenType::Eof) {
            state.declaration();
        }

        let function = state.end_compiler();

        if state.parser.had_error {
            Err(())
        } else {
            Ok(Rc::new(function))
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.current.as_mut().unwrap().function.chunk
    }

    fn advance(&mut self) {
        self.parser.previous = self.parser.current.take();

        loop {
            let token = self.scanner.scan_token();
            if token.token_type != TokenType::Error {
                self.parser.current = Some(token);
                break;
            } else {
                self.error_at(&token, token.lexeme);
            }
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.parser.current.as_ref().map(|t| t.token_type) == Some(token_type) {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.parser.current.as_ref().map(|t| t.token_type) == Some(token_type)
    }

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }
        self.advance();
        true
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.parser.previous.as_ref().map(|t| t.line).unwrap_or(0);
        self.current_chunk().write(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_return(&mut self) {
        if self.current.as_ref().unwrap().function_type == FunctionType::Initializer {
            self.emit_bytes(OpCode::GetLocal.into(), 0);
        } else {
            self.emit_byte(OpCode::Nil.into());
        }
        self.emit_byte(OpCode::Return.into());
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(OpCode::Constant.into(), constant);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.current_chunk().add_constant(value);
        if constant > u8::MAX as usize {
            self.error("Too many constants in one chunk.");
            return 0;
        }
        constant as u8
    }

    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(u8::MAX);
        self.emit_byte(u8::MAX);
        self.current_chunk().count() - 2
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OpCode::Loop.into());

        let offset = self.current_chunk().count() - loop_start + 2;
        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
        }

        let bytes = (offset as u16).to_be_bytes();
        self.emit_byte(bytes[0]);
        self.emit_byte(bytes[1]);
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().count() - offset - 2;

        if jump > u16::MAX as usize {
            self.error("Too much code to jump over.");
        }

        let bytes = (jump as u16).to_be_bytes();
        self.current_chunk().code[offset] = bytes[0];
        self.current_chunk().code[offset + 1] = bytes[1];
    }

    fn end_compiler(&mut self) -> Function {
        self.emit_return();
        let compiler = self.current.take().unwrap();
        let function = compiler.function;

        if let Some(enclosing) = compiler.enclosing {
            self.current = Some(enclosing);
        }

        function
    }

    fn begin_scope(&mut self) {
        self.current.as_mut().unwrap().scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.current.as_mut().unwrap().scope_depth -= 1;

        while !self.current.as_ref().unwrap().locals.is_empty() {
            let local = self.current.as_ref().unwrap().locals.last().unwrap();
            if local.depth.is_some()
                && local.depth.unwrap() > self.current.as_ref().unwrap().scope_depth
            {
                if local.is_captured {
                    self.emit_byte(OpCode::CloseUpvalue.into());
                } else {
                    self.emit_byte(OpCode::Pop.into());
                }
                self.current.as_mut().unwrap().locals.pop();
            } else {
                break;
            }
        }
    }

    fn declaration(&mut self) {
        if self.match_token(TokenType::Class) {
            self.class_declaration();
        } else if self.match_token(TokenType::Fun) {
            self.fun_declaration();
        } else if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.synchronize();
        }
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expect class name.");
        let class_name = self.parser.previous.as_ref().unwrap().lexeme;
        let name_constant = self.identifier_constant(class_name);
        self.declare_variable();

        self.emit_bytes(OpCode::Class.into(), name_constant);
        self.define_variable(name_constant);

        let mut class_compiler = ClassCompiler {
            enclosing: None,
            has_superclass: false,
        };

        if let Some(current_class) = self.current_class.take() {
            class_compiler.enclosing = Some(current_class);
        }
        self.current_class = Some(Box::new(class_compiler));

        if self.match_token(TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);

            if class_name == self.parser.previous.as_ref().unwrap().lexeme {
                self.error("A class can't inherit from itself.");
            }

            self.begin_scope();
            self.add_local("super");
            self.define_variable(0);

            self.named_variable(class_name, false);
            self.emit_byte(OpCode::Inherit.into());
            self.current_class.as_mut().unwrap().has_superclass = true;
        }

        self.named_variable(class_name, false);
        self.consume(TokenType::LeftBrace, "Expect '{' before class body.");

        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.method();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after class body.");
        self.emit_byte(OpCode::Pop.into());

        if self.current_class.as_ref().unwrap().has_superclass {
            self.end_scope();
        }

        if let Some(enclosing) = self.current_class.as_mut().unwrap().enclosing.take() {
            self.current_class = Some(enclosing);
        } else {
            self.current_class = None;
        }
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expect method name.");
        let name = self.parser.previous.as_ref().unwrap().lexeme;
        let constant = self.identifier_constant(name);

        let function_type = if name == "init" {
            FunctionType::Initializer
        } else {
            FunctionType::Method
        };

        self.function(function_type);
        self.emit_bytes(OpCode::Method.into(), constant);
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn function(&mut self, function_type: FunctionType) {
        let mut compiler = FunctionCompiler {
            enclosing: None,
            function: Function::new(),
            function_type,
            locals: Vec::with_capacity(vm::U8_COUNT),
            upvalues: Vec::with_capacity(vm::U8_COUNT),
            scope_depth: 0,
        };
        compiler.locals.push(Local {
            name: if function_type != FunctionType::Function {
                "this"
            } else {
                ""
            },
            depth: Some(0),
            is_captured: false,
        });

        if function_type != FunctionType::Script {
            let name = self.parser.previous.as_ref().unwrap().lexeme;
            compiler.function.name = Some(Rc::from(name));
        }

        compiler.enclosing = self.current.take();
        self.current = Some(Box::new(compiler));

        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.check(TokenType::RightParen) {
            loop {
                self.current.as_mut().unwrap().function.arity += 1;
                if self.current.as_ref().unwrap().function.arity > 255 {
                    self.error_at_current("Can't have more than 255 parameters.");
                }
                let constant = self.parse_variable("Expect parameter name.");
                self.define_variable(constant);

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        let upvalue_data: Vec<(bool, u8)> = self
            .current
            .as_ref()
            .unwrap()
            .upvalues
            .iter()
            .map(|u| (u.is_local, u.index))
            .collect();

        let function = self.end_compiler();
        let constant = self.make_constant(Value::Obj(Rc::new(Obj::Function(Rc::new(function)))));
        self.emit_bytes(OpCode::Closure.into(), constant);

        upvalue_data.into_iter().for_each(|(is_local, index)| {
            self.emit_byte(if is_local { 1 } else { 0 });
            self.emit_byte(index);
        });
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.match_token(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OpCode::Nil.into());
        }

        self.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );
        self.define_variable(global);
    }

    fn parse_variable(&mut self, error_msg: &str) -> u8 {
        self.consume(TokenType::Identifier, error_msg);
        self.declare_variable();
        if self.current.as_ref().unwrap().scope_depth > 0 {
            return 0;
        }

        let name = self.parser.previous.as_ref().unwrap().lexeme;
        self.identifier_constant(name)
    }

    fn identifier_constant(&mut self, name: &str) -> u8 {
        let interned_string = self.interner.intern(name);
        let value = Value::Obj(Rc::new(Obj::String(interned_string)));
        self.make_constant(value)
    }

    fn declare_variable(&mut self) {
        if self.current.as_ref().unwrap().scope_depth == 0 {
            return;
        }

        let name = self.parser.previous.as_ref().unwrap().lexeme;

        let scope_depth = self.current.as_ref().unwrap().scope_depth;
        let has_duplicate = self
            .current
            .as_ref()
            .unwrap()
            .locals
            .iter()
            .rev()
            .take_while(|local| local.depth.is_none_or(|depth| depth >= scope_depth))
            .any(|local| local.name == name);

        if has_duplicate {
            self.error("Already a variable with this name in this scope.");
        }

        self.add_local(name);
    }

    fn add_local(&mut self, name: &'a str) {
        if self.current.as_ref().unwrap().locals.len() >= vm::U8_COUNT {
            self.error("Too many local variables in function.");
            return;
        }

        self.current.as_mut().unwrap().locals.push(Local {
            name,
            depth: None,
            is_captured: false,
        });
    }

    fn define_variable(&mut self, global: u8) {
        if self.current.as_ref().unwrap().scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.emit_bytes(OpCode::DefineGlobal.into(), global);
    }

    fn mark_initialized(&mut self) {
        if self.current.as_ref().unwrap().scope_depth == 0 {
            return;
        }
        let scope_depth = self.current.as_ref().unwrap().scope_depth;
        if let Some(local) = self.current.as_mut().unwrap().locals.last_mut() {
            local.depth = Some(scope_depth);
        }
    }

    fn statement(&mut self) {
        if self.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.match_token(TokenType::For) {
            self.for_statement();
        } else if self.match_token(TokenType::If) {
            self.if_statement();
        } else if self.match_token(TokenType::Return) {
            self.return_statement();
        } else if self.match_token(TokenType::While) {
            self.while_statement();
        } else if self.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn return_statement(&mut self) {
        if self.current.as_ref().unwrap().function_type == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }

        if self.match_token(TokenType::Semicolon) {
            self.emit_return();
        } else {
            if self.current.as_ref().unwrap().function_type == FunctionType::Initializer {
                self.error("Can't return a value from an initializer.");
            }

            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_byte(OpCode::Return.into());
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OpCode::Print.into());
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OpCode::JumpIfFalse.into());
        self.emit_byte(OpCode::Pop.into());
        self.statement();

        let else_jump = self.emit_jump(OpCode::Jump.into());
        self.patch_jump(then_jump);
        self.emit_byte(OpCode::Pop.into());

        if self.match_token(TokenType::Else) {
            self.statement();
        }
        self.patch_jump(else_jump);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().count();

        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::JumpIfFalse.into());
        self.emit_byte(OpCode::Pop.into());
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(OpCode::Pop.into());
    }

    fn for_statement(&mut self) {
        self.begin_scope();

        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");

        if self.match_token(TokenType::Semicolon) {
            // No initializer.
        } else if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().count();

        let mut exit_jump = None;
        if !self.match_token(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            // Jump out of the loop if the condition is false.
            exit_jump = Some(self.emit_jump(OpCode::JumpIfFalse.into()));
            self.emit_byte(OpCode::Pop.into()); // Condition.
        }

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::Jump.into());
            let increment_start = self.current_chunk().count();
            self.expression();
            self.emit_byte(OpCode::Pop.into());
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit) = exit_jump {
            self.patch_jump(exit);
            self.emit_byte(OpCode::Pop.into()); // Condition.
        }

        self.end_scope();
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OpCode::Pop.into());
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::Eof) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = Self::get_rule(self.parser.previous.as_ref().unwrap().token_type).prefix;

        match prefix_rule {
            Some(prefix_fn) => {
                let can_assign = precedence <= Precedence::Assignment;
                prefix_fn(self, can_assign);

                while precedence
                    <= Self::get_rule(self.parser.current.as_ref().unwrap().token_type).precedence
                {
                    self.advance();
                    let infix_rule =
                        Self::get_rule(self.parser.previous.as_ref().unwrap().token_type).infix;
                    if let Some(infix_fn) = infix_rule {
                        infix_fn(self, can_assign);
                    }
                }

                if can_assign && self.match_token(TokenType::Equal) {
                    self.error("Invalid assignment target.");
                }
            }
            None => {
                self.error("Expect expression.");
            }
        }
    }

    fn get_rule(token_type: TokenType) -> ParseRule<'a> {
        match token_type {
            TokenType::LeftParen => ParseRule {
                prefix: Some(Self::grouping),
                infix: Some(Self::call),
                precedence: Precedence::Call,
            },
            TokenType::Dot => ParseRule {
                prefix: None,
                infix: Some(Self::dot),
                precedence: Precedence::Call,
            },
            TokenType::Minus => ParseRule {
                prefix: Some(Self::unary),
                infix: Some(Self::binary),
                precedence: Precedence::Term,
            },
            TokenType::Plus => ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Term,
            },
            TokenType::Slash | TokenType::Star => ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Factor,
            },
            TokenType::Number => ParseRule {
                prefix: Some(Self::number),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::False | TokenType::True | TokenType::Nil => ParseRule {
                prefix: Some(Self::literal),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Bang => ParseRule {
                prefix: Some(Self::unary),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::BangEqual | TokenType::EqualEqual => ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Equality,
            },
            TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Less
            | TokenType::LessEqual => ParseRule {
                prefix: None,
                infix: Some(Self::binary),
                precedence: Precedence::Comparison,
            },
            TokenType::String => ParseRule {
                prefix: Some(Self::string),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Identifier => ParseRule {
                prefix: Some(Self::variable),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::And => ParseRule {
                prefix: None,
                infix: Some(Self::and_),
                precedence: Precedence::And,
            },
            TokenType::Or => ParseRule {
                prefix: None,
                infix: Some(Self::or_),
                precedence: Precedence::Or,
            },
            TokenType::This => ParseRule {
                prefix: Some(Self::this_),
                infix: None,
                precedence: Precedence::None,
            },
            TokenType::Super => ParseRule {
                prefix: Some(Self::super_),
                infix: None,
                precedence: Precedence::None,
            },
            _ => ParseRule {
                prefix: None,
                infix: None,
                precedence: Precedence::None,
            },
        }
    }

    fn number(&mut self, _can_assign: bool) {
        let value: f64 = self
            .parser
            .previous
            .as_ref()
            .unwrap()
            .lexeme
            .parse()
            .unwrap();
        self.emit_constant(Value::Number(value));
    }

    fn literal(&mut self, _can_assign: bool) {
        match self.parser.previous.as_ref().unwrap().token_type {
            TokenType::False => self.emit_byte(OpCode::False.into()),
            TokenType::Nil => self.emit_byte(OpCode::Nil.into()),
            TokenType::True => self.emit_byte(OpCode::True.into()),
            _ => unreachable!(),
        }
    }

    fn string(&mut self, _can_assign: bool) {
        let lexeme = self.parser.previous.as_ref().unwrap().lexeme;
        let string_value = &lexeme[1..lexeme.len() - 1];
        let interned_string = self.interner.intern(string_value);
        let value = Value::Obj(Rc::new(Obj::String(interned_string)));
        self.emit_constant(value);
    }

    fn variable(&mut self, can_assign: bool) {
        let name = self.parser.previous.as_ref().unwrap().lexeme;
        self.named_variable(name, can_assign);
    }

    fn this_(&mut self, _can_assign: bool) {
        if self.current_class.is_none() {
            self.error("Can't use 'this' outside of a class.");
            return;
        }
        self.variable(false);
    }

    fn super_(&mut self, _can_assign: bool) {
        match &self.current_class {
            None => {
                self.error("Can't use 'super' outside of a class.");
            }
            Some(class_compiler) if !class_compiler.has_superclass => {
                self.error("Can't use 'super' in a class with no superclass.");
            }
            _ => {}
        }

        self.consume(TokenType::Dot, "Expect '.' after 'super'.");
        self.consume(TokenType::Identifier, "Expect superclass method name.");
        let name = self.parser.previous.as_ref().unwrap().lexeme;
        let name_constant = self.identifier_constant(name);

        self.named_variable("this", false);
        if self.match_token(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.named_variable("super", false);
            self.emit_bytes(OpCode::SuperInvoke.into(), name_constant);
            self.emit_byte(arg_count);
        } else {
            self.named_variable("super", false);
            self.emit_bytes(OpCode::GetSuper.into(), name_constant);
        }
    }

    fn named_variable(&mut self, name: &str, can_assign: bool) {
        let (get_op, set_op, arg) = if let Some(arg) = self.resolve_local(name) {
            (OpCode::GetLocal.into(), OpCode::SetLocal.into(), arg)
        } else if let Some(arg) = self.resolve_upvalue(name) {
            (OpCode::GetUpvalue.into(), OpCode::SetUpvalue.into(), arg)
        } else {
            let arg = self.identifier_constant(name);
            (OpCode::GetGlobal.into(), OpCode::SetGlobal.into(), arg)
        };

        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op, arg);
        } else {
            self.emit_bytes(get_op, arg);
        }
    }

    fn resolve_local(&mut self, name: &str) -> Option<u8> {
        let result = self
            .current
            .as_ref()
            .unwrap()
            .locals
            .iter()
            .enumerate()
            .rev()
            .find(|(_, local)| local.name == name)
            .map(|(i, local)| (i, local.depth.is_none()));

        match result {
            Some((i, is_uninitialized)) if is_uninitialized => {
                self.error("Can't read local variable in its own initializer.");
                None
            }
            Some((i, _)) => Some(i as u8),
            None => None,
        }
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u8> {
        self.current.as_ref().unwrap().enclosing.as_ref()?;

        let local_result = self
            .current
            .as_ref()
            .unwrap()
            .enclosing
            .as_ref()
            .unwrap()
            .locals
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.name == name)
            .and_then(|(i, l)| if l.depth.is_some() { Some(i) } else { None });

        if let Some(local) = local_result {
            if let Some(enclosing) = self.current.as_mut().unwrap().enclosing.as_mut() {
                enclosing.locals[local].is_captured = true;
            }
            return Some(self.add_upvalue(local as u8, true));
        }

        let mut enclosing = self.current.as_mut().unwrap().enclosing.take().unwrap();
        let mut temp_state = Compiler {
            scanner: Scanner::new(""),
            parser: Parser {
                current: None,
                previous: None,
                had_error: false,
                panic_mode: false,
            },
            current: Some(enclosing),
            current_class: None,
            interner: StringInterner::new(),
        };

        let upvalue_result = temp_state.resolve_upvalue(name);

        enclosing = temp_state.current.take().unwrap();
        self.current.as_mut().unwrap().enclosing = Some(enclosing);

        if let Some(upvalue) = upvalue_result {
            return Some(self.add_upvalue(upvalue, false));
        }

        None
    }

    fn add_upvalue(&mut self, index: u8, is_local: bool) -> u8 {
        let upvalue_count = self.current.as_ref().unwrap().function.upvalue_count;

        if let Some(i) = self.current.as_ref().unwrap().upvalues[..upvalue_count]
            .iter()
            .position(|upvalue| upvalue.index == index && upvalue.is_local == is_local)
        {
            return i as u8;
        }

        if upvalue_count >= vm::U8_COUNT {
            self.error("Too many closure variables in function.");
            return 0;
        }

        let function_compiler = self.current.as_mut().unwrap();
        function_compiler.upvalues.push(Upvalue { index, is_local });
        function_compiler.function.upvalue_count += 1;
        (upvalue_count) as u8
    }

    fn grouping(&mut self, _can_assign: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn call(&mut self, _can_assign: bool) {
        let arg_count = self.argument_list();
        self.emit_bytes(OpCode::Call.into(), arg_count);
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expect property name after '.'.");
        let name = self.parser.previous.as_ref().unwrap().lexeme;
        let name_constant = self.identifier_constant(name);

        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.emit_bytes(OpCode::SetProperty.into(), name_constant);
        } else if self.match_token(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_bytes(OpCode::Invoke.into(), name_constant);
            self.emit_byte(arg_count);
        } else {
            self.emit_bytes(OpCode::GetProperty.into(), name_constant);
        }
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;
        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                } else {
                    arg_count += 1;
                }
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn unary(&mut self, _can_assign: bool) {
        let operator_type = self.parser.previous.as_ref().unwrap().token_type;

        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.emit_byte(OpCode::Negate.into()),
            TokenType::Bang => self.emit_byte(OpCode::Not.into()),
            _ => unreachable!(),
        }
    }

    fn binary(&mut self, _can_assign: bool) {
        let operator_type = self.parser.previous.as_ref().unwrap().token_type;
        let rule = Self::get_rule(operator_type);
        self.parse_precedence(rule.precedence.next());

        match operator_type {
            TokenType::Plus => self.emit_byte(OpCode::Add.into()),
            TokenType::Minus => self.emit_byte(OpCode::Subtract.into()),
            TokenType::Star => self.emit_byte(OpCode::Multiply.into()),
            TokenType::Slash => self.emit_byte(OpCode::Divide.into()),
            TokenType::BangEqual => self.emit_bytes(OpCode::Equal.into(), OpCode::Not.into()),
            TokenType::EqualEqual => self.emit_byte(OpCode::Equal.into()),
            TokenType::Greater => self.emit_byte(OpCode::Greater.into()),
            TokenType::GreaterEqual => self.emit_bytes(OpCode::Less.into(), OpCode::Not.into()),
            TokenType::Less => self.emit_byte(OpCode::Less.into()),
            TokenType::LessEqual => self.emit_bytes(OpCode::Greater.into(), OpCode::Not.into()),
            _ => unreachable!(),
        }
    }

    fn and_(&mut self, _can_assign: bool) {
        let end_jump = self.emit_jump(OpCode::JumpIfFalse.into());

        self.emit_byte(OpCode::Pop.into());
        self.parse_precedence(Precedence::And);

        self.patch_jump(end_jump);
    }

    fn or_(&mut self, _can_assign: bool) {
        let else_jump = self.emit_jump(OpCode::JumpIfFalse.into());
        let end_jump = self.emit_jump(OpCode::Jump.into());

        self.patch_jump(else_jump);
        self.emit_byte(OpCode::Pop.into());

        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
    }

    fn synchronize(&mut self) {
        self.parser.panic_mode = false;

        while self.parser.current.as_ref().map(|t| t.token_type) != Some(TokenType::Eof) {
            if self.parser.previous.as_ref().map(|t| t.token_type) == Some(TokenType::Semicolon) {
                return;
            }

            match self.parser.current.as_ref().map(|t| t.token_type) {
                Some(TokenType::Class)
                | Some(TokenType::Fun)
                | Some(TokenType::Var)
                | Some(TokenType::For)
                | Some(TokenType::If)
                | Some(TokenType::While)
                | Some(TokenType::Print)
                | Some(TokenType::Return) => return,
                _ => {} // Do nothing.
            }

            self.advance();
        }
    }

    fn error_at(&mut self, token: &Token, message: &str) {
        if self.parser.panic_mode {
            return;
        }
        self.parser.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        if token.token_type == TokenType::Eof {
            eprint!(" at end");
        } else if token.token_type == TokenType::Error {
            // Nothing.
        } else {
            eprint!(" at '{}'", token.lexeme);
        }

        eprintln!(": {}", message);
        self.parser.had_error = true;
    }

    fn error(&mut self, message: &str) {
        if let Some(prev) = self.parser.previous {
            self.error_at(&prev, message);
        }
    }

    fn error_at_current(&mut self, message: &str) {
        if let Some(curr) = self.parser.current {
            self.error_at(&curr, message);
        }
    }
}
