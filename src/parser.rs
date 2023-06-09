#![allow(clippy::while_let_loop)]

use std::fmt::Display;

use crate::token::{next_universal_index, Token, TokenType};

#[derive(Debug, Clone)]
pub enum Stmt {
	Expr(Expr),
	Print(Expr),
	Var {
		name: Token,
		initializer: Option<Expr>,
	},
	Block(Vec<Stmt>),
	If {
		condition: Expr,
		then_branch: Box<Stmt>,
		else_branch: Option<Box<Stmt>>,
	},
	While {
		condition: Expr,
		body: Box<Stmt>,
	},
	Function(FunctionStatement),
	Return {
		keyword: Token,
		value: Expr,
	},
	Class {
		name: Token,
		superclass: Option<Token>,
		methods: Vec<FunctionStatement>,
	},
}

#[derive(Debug, Clone)]
pub struct FunctionStatement {
	pub name: Token,
	pub params: Vec<Token>,
	pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Expr {
	Literal(Token),
	Variable(Token),
	Assign {
		name: Token,
		value: Box<Expr>,
	},
	Unary {
		operator: Token,
		expr: Box<Expr>,
	},
	Binary {
		left: Box<Expr>,
		operator: Token,
		right: Box<Expr>,
	},
	Grouping(Box<Expr>),
	Logical {
		left: Box<Expr>,
		operator: Token,
		right: Box<Expr>,
	},
	Call {
		callee: Box<Expr>,
		closing_parenthesis: Token,
		arguments: Vec<Expr>,
	},
	Get {
		object: Box<Expr>,
		name: Token,
	},
	Set {
		object: Box<Expr>,
		name: Token,
		value: Box<Expr>,
	},
	This {
		keyword: Token,
	},
	Super {
		keyword: Token,
		method: Token,
	},
}

impl Display for Expr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		print_ast(self, f)
	}
}

pub struct Parser {
	tokens: std::iter::Peekable<std::vec::IntoIter<Token>>,
}

#[derive(Debug)]
pub struct Error {
	pub kind: ErrorKind,
	pub token: Option<Token>,
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug)]
pub enum ErrorKind {
	ExpectedExpression,
	ExpectedLeftParenthesis,
	ExpectedRightParenthesis,
	ExpectedRightBrace,
	ExpectedLeftBrace,
	ExpectedSemicolon,
	ExpectedIdentifier { place: &'static str },
	InvalidAssignmentTarget,
	ExceededArgumentsLimit,
	ExpectedComma,
	ExpectedDot,
}

impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		match &self.token {
			Some(Token { line, .. }) => write!(f, "[line {line}] ")?,
			None => write!(f, "[line unknown] ")?,
		}
		match self.kind {
			ErrorKind::ExpectedExpression => write!(f, "expected expression")?,
			ErrorKind::ExpectedRightParenthesis => write!(f, "expected `)` after expression")?,
			ErrorKind::ExpectedSemicolon => write!(f, "expected `;` after statement")?,
			ErrorKind::ExpectedIdentifier { place } => write!(f, "expected {place} identifier")?,
			ErrorKind::InvalidAssignmentTarget => write!(f, "invalid assignment target")?,
			ErrorKind::ExpectedLeftBrace => write!(f, "expected `{{` at the end of a block")?,
			ErrorKind::ExpectedRightBrace => write!(f, "expected `}}` at the end of a block")?,
			ErrorKind::ExpectedLeftParenthesis => write!(f, "expected `(`")?,
			ErrorKind::ExceededArgumentsLimit => write!(f, "can't have more than 255 arguments")?,
			ErrorKind::ExpectedComma => write!(f, "expected `,`")?,
			ErrorKind::ExpectedDot => write!(f, "expected `.`")?,
		}
		match &self.token {
			None
			| Some(Token {
				token_type: TokenType::Eof,
				..
			}) => write!(f, " at the end")?,
			Some(Token { lexeme, .. }) => write!(f, " at `{lexeme}`")?,
		}
		Ok(())
	}
}

impl std::error::Error for Error {}

macro_rules! expect_token_type {
	($parser:ident, $pattern:pat) => {{
		match $parser.tokens.next() {
			Some(token)
				if matches!(
					token,
					Token {
						token_type: $pattern,
						..
					}
				) =>
			{
				Ok(token)
			}
			token => Err(token),
		}
	}};
}

impl Parser {
	pub fn new(tokens: Vec<Token>) -> Self {
		Parser {
			tokens: tokens.into_iter().peekable(),
		}
	}

	pub fn parse(mut self) -> Result<Vec<Stmt>, Error> {
		let mut statements = Vec::new();
		while self
			.tokens
			.peek()
			.map(|t| !matches!(t.token_type, TokenType::Eof))
			.unwrap_or_default()
		{
			//TODO(aqatl): if this fails, we should call [self.synchronize]
			let declaration = self.declaration()?;
			statements.push(declaration);
		}
		Ok(statements)
	}

	fn declaration(&mut self) -> Result<Stmt, Error> {
		match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Fun,
				..
			}) => {
				let _ = self.tokens.next().unwrap();
				self.function("function")
			}
			Some(Token {
				token_type: TokenType::Var,
				..
			}) => {
				let _ = self.tokens.next().unwrap();
				self.var_declaration()
			}
			Some(Token {
				token_type: TokenType::Class,
				..
			}) => {
				let _ = self.tokens.next().unwrap();
				self.class_declaration()
			}
			_ => self.statement(),
		}
	}

	fn var_declaration(&mut self) -> Result<Stmt, Error> {
		let name = match self.tokens.next() {
			Some(
				t @ Token {
					token_type: TokenType::Identifier(_),
					..
				},
			) => t,
			t => {
				return Err(Error {
					kind: ErrorKind::ExpectedIdentifier { place: "variable" },
					token: t,
				})
			}
		};

		let initializer = match self.tokens.next() {
			Some(Token {
				token_type: TokenType::Equal,
				..
			}) => Some(self.expression()?),
			Some(Token {
				token_type: TokenType::Semicolon,
				..
			}) => None,
			token => {
				return Err(Error {
					kind: ErrorKind::ExpectedSemicolon,
					token,
				})
			}
		};

		if initializer.is_some() {
			match self.tokens.next() {
				Some(Token {
					token_type: TokenType::Semicolon,
					..
				}) => (),
				t => {
					return Err(Error {
						kind: ErrorKind::ExpectedSemicolon,
						token: t,
					})
				}
			}
		}

		Ok(Stmt::Var { name, initializer })
	}

	fn class_declaration(&mut self) -> Result<Stmt, Error> {
		let name = expect_token_type!(self, TokenType::Identifier(_)).map_err(|token| Error {
			kind: ErrorKind::ExpectedIdentifier { place: "class" },
			token,
		})?;

		let superclass = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Less,
				..
			}) => {
				let _ = self.tokens.next();
				Some(
					expect_token_type!(self, TokenType::Identifier(_)).map_err(|token| Error {
						kind: ErrorKind::ExpectedIdentifier {
							place: "superclass",
						},
						token,
					})?,
				)
			}
			_ => None,
		};

		expect_token_type!(self, TokenType::LeftBrace).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftBrace,
			token,
		})?;

		let mut methods = Vec::new();

		loop {
			if let Some(Token {
				token_type: TokenType::RightBrace | TokenType::Eof,
				..
			})
			| None = self.tokens.peek()
			{
				break;
			}
			let method = match self.function("method")? {
				Stmt::Function(v) => v,
				_ => panic!(),
			};
			methods.push(method);
		}

		expect_token_type!(self, TokenType::RightBrace).map_err(|token| Error {
			kind: ErrorKind::ExpectedRightBrace,
			token,
		})?;

		Ok(Stmt::Class {
			name,
			superclass,
			methods,
		})
	}

	fn function(&mut self, place: &'static str) -> Result<Stmt, Error> {
		let name = expect_token_type!(self, TokenType::Identifier(_)).map_err(|token| Error {
			kind: ErrorKind::ExpectedIdentifier { place },
			token,
		})?;

		expect_token_type!(self, TokenType::LeftParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftParenthesis,
			token,
		})?;

		let mut params = Vec::new();

		loop {
			if params.len() >= 255 {
				return Err(Error {
					kind: ErrorKind::ExceededArgumentsLimit,
					token: params.pop(),
				});
			}
			match self.tokens.next() {
				Some(Token {
					token_type: TokenType::RightParen,
					..
				}) => break,
				Some(
					token @ Token {
						token_type: TokenType::Identifier(_),
						..
					},
				) => {
					params.push(token);
					match self.tokens.next() {
						Some(Token {
							token_type: TokenType::Comma,
							..
						}) => continue,
						Some(Token {
							token_type: TokenType::RightParen,
							..
						}) => break,
						token => {
							return Err(Error {
								kind: ErrorKind::ExpectedComma,
								token,
							})
						}
					}
				}
				token => {
					return Err(Error {
						kind: ErrorKind::ExpectedIdentifier { place: "parameter" },
						token,
					});
				}
			}
		}

		expect_token_type!(self, TokenType::LeftBrace).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftBrace,
			token,
		})?;

		let body = self.block()?;

		Ok(Stmt::Function(FunctionStatement { name, params, body }))
	}

	fn statement(&mut self) -> Result<Stmt, Error> {
		match self.tokens.peek().map(|t| &t.token_type) {
			Some(TokenType::If) => {
				let _ = self.tokens.next().unwrap();
				self.if_statement()
			}
			Some(TokenType::Print) => {
				let _ = self.tokens.next().unwrap();
				self.print_statement()
			}
			Some(TokenType::While) => {
				let _ = self.tokens.next().unwrap();
				self.while_statement()
			}
			Some(TokenType::For) => {
				let _ = self.tokens.next().unwrap();
				self.for_statement()
			}
			Some(TokenType::LeftBrace) => {
				let _ = self.tokens.next().unwrap();
				self.block().map(Stmt::Block)
			}
			Some(TokenType::Return) => {
				let keyword = self.tokens.next().unwrap();
				self.return_statement(keyword)
			}
			_ => self.expression_statement(),
		}
	}

	fn if_statement(&mut self) -> Result<Stmt, Error> {
		expect_token_type!(self, TokenType::LeftParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftParenthesis,
			token,
		})?;
		let condition = self.expression()?;
		expect_token_type!(self, TokenType::RightParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedRightParenthesis,
			token,
		})?;

		let then_branch = Box::new(self.statement()?);
		let else_branch = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Else,
				..
			}) => {
				let _ = self.tokens.next();
				Some(Box::new(self.statement()?))
			}
			_ => None,
		};

		Ok(Stmt::If {
			condition,
			then_branch,
			else_branch,
		})
	}

	fn print_statement(&mut self) -> Result<Stmt, Error> {
		let value = self.expression()?;
		//TODO(aqatl): report token
		let token = self.tokens.next().ok_or(Error {
			kind: ErrorKind::ExpectedSemicolon,
			token: None,
		})?;
		if !matches!(token.token_type, TokenType::Semicolon) {
			return Err(Error {
				kind: ErrorKind::ExpectedSemicolon,
				token: None,
			});
		}
		Ok(Stmt::Print(value))
	}

	fn while_statement(&mut self) -> Result<Stmt, Error> {
		expect_token_type!(self, TokenType::LeftParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftParenthesis,
			token,
		})?;
		let condition = self.expression()?;
		expect_token_type!(self, TokenType::RightParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedRightParenthesis,
			token,
		})?;
		let body = self.statement()?;
		Ok(Stmt::While {
			condition,
			body: Box::new(body),
		})
	}

	fn for_statement(&mut self) -> Result<Stmt, Error> {
		expect_token_type!(self, TokenType::LeftParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedLeftParenthesis,
			token,
		})?;

		let initializer = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Semicolon,
				..
			}) => {
				let _ = self.tokens.next();
				None
			}
			Some(Token {
				token_type: TokenType::Var,
				..
			}) => {
				let _ = self.tokens.next();
				Some(self.var_declaration()?)
			}
			_ => Some(self.expression_statement()?),
		};

		let condition = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Semicolon,
				..
			}) => None,
			_ => Some(self.expression()?),
		};

		expect_token_type!(self, TokenType::Semicolon).map_err(|token| Error {
			kind: ErrorKind::ExpectedSemicolon,
			token,
		})?;

		let increment = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::RightParen,
				..
			}) => None,
			_ => Some(self.expression()?),
		};

		expect_token_type!(self, TokenType::RightParen).map_err(|token| Error {
			kind: ErrorKind::ExpectedRightParenthesis,
			token,
		})?;

		let mut body = self.statement()?;

		// desugar into while loop

		if let Some(increment) = increment {
			body = Stmt::Block(vec![body, Stmt::Expr(increment)]);
		}

		let condition = condition.unwrap_or_else(|| {
			Expr::Literal(Token {
				token_type: TokenType::True,
				lexeme: "".to_string(),
				line: 1,
				universal_index: next_universal_index(),
			})
		});

		if let Some(initializer) = initializer {
			body = Stmt::Block(vec![
				initializer,
				Stmt::While {
					condition,
					body: Box::new(body),
				},
			]);
		}

		Ok(body)
	}

	fn block(&mut self) -> Result<Vec<Stmt>, Error> {
		let mut statements = Vec::new();

		loop {
			match self.tokens.peek() {
				Some(Token {
					token_type: TokenType::RightBrace,
					..
				})
				| None => {
					break;
				}
				_ => (),
			}
			let statement = self.declaration()?;
			statements.push(statement);
		}

		let token = self.tokens.next();
		if !matches!(
			token,
			Some(Token {
				token_type: TokenType::RightBrace,
				..
			})
		) {
			return Err(Error {
				kind: ErrorKind::ExpectedRightBrace,
				token,
			});
		}

		Ok(statements)
	}

	fn return_statement(&mut self, keyword: Token) -> Result<Stmt, Error> {
		let value = match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Semicolon,
				..
			}) => Expr::Literal(Token {
				token_type: TokenType::Nil,
				lexeme: keyword.lexeme.clone(),
				line: keyword.line,
				universal_index: next_universal_index(),
			}),
			_ => self.expression()?,
		};

		expect_token_type!(self, TokenType::Semicolon).map_err(|token| Error {
			kind: ErrorKind::ExpectedSemicolon,
			token,
		})?;

		Ok(Stmt::Return { keyword, value })
	}

	fn expression_statement(&mut self) -> Result<Stmt, Error> {
		let expr = self.expression()?;
		let token = self.tokens.next().ok_or(Error {
			kind: ErrorKind::ExpectedSemicolon,
			token: None,
		})?;
		if !matches!(token.token_type, TokenType::Semicolon) {
			return Err(Error {
				kind: ErrorKind::ExpectedSemicolon,
				token: None,
			});
		}
		Ok(Stmt::Expr(expr))
	}

	fn expression(&mut self) -> Result<Expr, Error> {
		self.assignment()
	}

	fn assignment(&mut self) -> Result<Expr, Error> {
		let expr = self.or()?;

		match self.tokens.peek() {
			Some(Token {
				token_type: TokenType::Equal,
				..
			}) => {
				let equals = self.tokens.next();
				let value = self.assignment()?;
				match expr {
					Expr::Variable(name) => Ok(Expr::Assign {
						name,
						value: Box::new(value),
					}),
					Expr::Get { object, name } => Ok(Expr::Set {
						object,
						name,
						value: Box::new(value),
					}),
					_ => Err(Error {
						kind: ErrorKind::InvalidAssignmentTarget,
						token: equals,
					}),
				}
			}
			_ => Ok(expr),
		}
	}

	fn or(&mut self) -> Result<Expr, Error> {
		let mut expr = self.and()?;

		loop {
			match self.tokens.peek() {
				Some(Token {
					token_type: TokenType::Or,
					..
				}) => {
					let operator = self.tokens.next().unwrap();
					let right = self.and()?;
					expr = Expr::Logical {
						left: Box::new(expr),
						operator,
						right: Box::new(right),
					};
				}
				_ => break,
			}
		}

		Ok(expr)
	}

	fn and(&mut self) -> Result<Expr, Error> {
		let mut expr = self.equality()?;

		loop {
			match self.tokens.peek() {
				Some(Token {
					token_type: TokenType::And,
					..
				}) => {
					let operator = self.tokens.next().unwrap();
					let right = self.equality()?;
					expr = Expr::Logical {
						left: Box::new(expr),
						operator,
						right: Box::new(right),
					};
				}
				_ => break,
			}
		}

		Ok(expr)
	}

	fn equality(&mut self) -> Result<Expr, Error> {
		let mut expr = self.comparison()?;
		loop {
			let operator = match self.tokens.peek().map(|t| &t.token_type) {
				Some(TokenType::BangEqual | TokenType::EqualEqual) => self.tokens.next().unwrap(),
				_ => break,
			};

			expr = Expr::Binary {
				left: Box::new(expr),
				operator,
				right: Box::new(self.comparison()?),
			};
		}

		Ok(expr)
	}

	fn comparison(&mut self) -> Result<Expr, Error> {
		let mut expr = self.term()?;

		loop {
			let operator = match self.tokens.peek().map(|t| &t.token_type) {
				Some(
					TokenType::Greater
					| TokenType::GreaterEqual
					| TokenType::Less
					| TokenType::LessEqual,
				) => self.tokens.next().unwrap(),
				_ => break,
			};

			expr = Expr::Binary {
				left: Box::new(expr),
				operator,
				right: Box::new(self.term()?),
			};
		}

		Ok(expr)
	}

	fn term(&mut self) -> Result<Expr, Error> {
		let mut expr = self.factor()?;

		loop {
			let operator = match self.tokens.peek().map(|t| &t.token_type) {
				Some(TokenType::Minus | TokenType::Plus) => self.tokens.next().unwrap(),
				_ => break,
			};

			expr = Expr::Binary {
				left: Box::new(expr),
				operator,
				right: Box::new(self.factor()?),
			};
		}

		Ok(expr)
	}

	fn factor(&mut self) -> Result<Expr, Error> {
		let mut expr = self.unary()?;

		loop {
			let operator = match self.tokens.peek().map(|t| &t.token_type) {
				Some(TokenType::Slash | TokenType::Star) => self.tokens.next().unwrap(),
				_ => break,
			};

			expr = Expr::Binary {
				left: Box::new(expr),
				operator,
				right: Box::new(self.unary()?),
			};
		}

		Ok(expr)
	}

	fn unary(&mut self) -> Result<Expr, Error> {
		let token = self.tokens.peek().ok_or(Error {
			kind: ErrorKind::ExpectedExpression,
			token: None,
		})?;

		match token.token_type {
			TokenType::Bang | TokenType::Minus => {
				let token = self.tokens.next().unwrap();
				Ok(Expr::Unary {
					operator: token,
					expr: Box::new(self.unary()?),
				})
			}
			_ => self.call(),
		}
	}

	fn call(&mut self) -> Result<Expr, Error> {
		let mut expr = self.primary()?;

		loop {
			match self.tokens.peek() {
				Some(Token {
					token_type: TokenType::LeftParen,
					..
				}) => {
					let _ = self.tokens.next();
					expr = self.finish_call(expr)?;
				}
				Some(Token {
					token_type: TokenType::Dot,
					..
				}) => {
					let _ = self.tokens.next();
					let name =
						expect_token_type!(self, TokenType::Identifier(_)).map_err(|token| {
							Error {
								kind: ErrorKind::ExpectedIdentifier {
									place: "property name",
								},
								token,
							}
						})?;

					expr = Expr::Get {
						object: Box::new(expr),
						name,
					};
				}
				_ => break,
			}
		}

		Ok(expr)
	}

	fn finish_call(&mut self, callee: Expr) -> Result<Expr, Error> {
		let mut arguments = Vec::new();

		if !matches!(
			self.tokens.peek(),
			Some(Token {
				token_type: TokenType::RightParen,
				..
			})
		) {
			loop {
				if arguments.len() >= 255 {
					// In the book, here we only report the error, not throw it
					return Err(Error {
						kind: ErrorKind::ExceededArgumentsLimit,
						token: self.tokens.peek().cloned(),
					});
				}
				arguments.push(self.expression()?);
				match self.tokens.peek() {
					Some(Token {
						token_type: TokenType::Comma,
						..
					}) => {
						let _ = self.tokens.next();
					}
					_ => break,
				}
			}
		}

		let closing_parenthesis =
			expect_token_type!(self, TokenType::RightParen).map_err(|token| Error {
				kind: ErrorKind::ExpectedRightParenthesis,
				token,
			})?;
		Ok(Expr::Call {
			callee: Box::new(callee),
			closing_parenthesis,
			arguments,
		})
	}

	fn primary(&mut self) -> Result<Expr, Error> {
		let token = self.tokens.next().ok_or(Error {
			kind: ErrorKind::ExpectedExpression,
			token: None,
		})?;

		match token.token_type {
			TokenType::Identifier(_) => Ok(Expr::Variable(token)),
			TokenType::Number(_)
			| TokenType::String(_)
			| TokenType::True
			| TokenType::False
			| TokenType::Nil => Ok(Expr::Literal(token)),
			TokenType::This => Ok(Expr::This { keyword: token }),
			TokenType::Super => {
				expect_token_type!(self, TokenType::Dot).map_err(|token| Error {
					kind: ErrorKind::ExpectedDot,
					token,
				})?;

				let method =
					expect_token_type!(self, TokenType::Identifier(_)).map_err(|token| Error {
						kind: ErrorKind::ExpectedIdentifier { place: "super" },
						token,
					})?;
				Ok(Expr::Super {
					keyword: token,
					method,
				})
			}
			TokenType::LeftParen => {
				let expr = self.expression()?;
				match self.tokens.next() {
					Some(Token {
						token_type: TokenType::RightParen,
						..
					}) => (),
					Some(token) => {
						return Err(Error {
							kind: ErrorKind::ExpectedRightParenthesis,
							token: Some(token),
						})
					}
					None => {
						return Err(Error {
							kind: ErrorKind::ExpectedRightParenthesis,
							token: None,
						})
					}
				}
				Ok(Expr::Grouping(Box::new(expr)))
			}
			_ => Err(Error {
				kind: ErrorKind::ExpectedExpression,
				token: Some(token),
			}),
		}
	}

	#[allow(dead_code)]
	fn synchronize(&mut self) {
		while let Some(token) = self.tokens.next() {
			if matches!(token.token_type, TokenType::Semicolon) {
				return;
			}

			let Some(token) = self.tokens.peek() else {
                return;
            };
			match token.token_type {
				TokenType::Class
				| TokenType::For
				| TokenType::Fun
				| TokenType::If
				| TokenType::Print
				| TokenType::Return
				| TokenType::Var
				| TokenType::While => {
					return;
				}
				_ => (),
			}
		}
	}
}

fn print_ast(expr: &Expr, w: &mut impl std::fmt::Write) -> std::fmt::Result {
	fn parenthesize(w: &mut impl std::fmt::Write, name: &str, exprs: &[&Expr]) -> std::fmt::Result {
		write!(w, "({name}")?;
		for expr in exprs {
			write!(w, " ")?;
			print_ast(expr, w)?;
		}
		write!(w, ")")?;
		Ok(())
	}

	match expr {
		Expr::Literal(Token {
			token_type: TokenType::Number(v),
			..
		}) => write!(w, "{v}"),
		Expr::Literal(Token {
			token_type: TokenType::String(v),
			..
		}) => write!(w, "{v}"),
		Expr::Literal(Token {
			token_type: TokenType::Identifier(v),
			..
		}) => write!(w, "{v}"),
		Expr::Literal(Token {
			token_type: TokenType::True,
			..
		}) => write!(w, "true"),
		Expr::Literal(Token {
			token_type: TokenType::False,
			..
		}) => write!(w, "false"),
		Expr::Literal(Token {
			token_type: TokenType::Nil,
			..
		}) => write!(w, "nil"),
		Expr::Literal(l) => panic!("{l:?}"),
		Expr::Variable(Token {
			token_type: TokenType::Identifier(var_name),
			..
		}) => write!(w, "{var_name}"),
		Expr::Variable(v) => panic!("{v:?}"),
		Expr::Assign {
			name: Token {
				token_type: TokenType::Identifier(name),
				..
			},
			value,
		} => parenthesize(w, &format!("= {name}"), &[value]),
		Expr::Assign { name, .. } => panic!("{name:?}"),
		Expr::Binary {
			left,
			operator: Token { lexeme, .. },
			right,
		} => parenthesize(w, lexeme, &[left, right]),
		Expr::Grouping(expr) => parenthesize(w, "group", &[expr]),
		Expr::Unary { operator, expr } => parenthesize(w, &operator.lexeme, &[expr]),
		Expr::Logical {
			left,
			operator,
			right,
		} => parenthesize(w, &operator.lexeme, &[left, right]),
		expr => todo!("{expr:?}"),
	}
}

#[cfg(test)]
mod tests {
	use super::Expr;
	use crate::token::{Token, TokenType};

	#[test]
	fn test_ast_printer() {
		let expr = Expr::Binary {
			left: Box::new(Expr::Unary {
				operator: Token {
					token_type: TokenType::Minus,
					lexeme: "-".to_string(),
					line: 1,
					universal_index: 0,
				},
				expr: Box::new(Expr::Literal(Token {
					token_type: TokenType::Number(123.0),
					lexeme: "123".to_string(),
					line: 1,
					universal_index: 1,
				})),
			}),
			operator: Token {
				token_type: TokenType::Star,
				lexeme: "*".to_string(),
				line: 1,
				universal_index: 2,
			},
			right: Box::new(Expr::Grouping(Box::new(Expr::Literal(Token {
				token_type: TokenType::Number(45.67),
				lexeme: "45.67".to_string(),
				line: 1,
				universal_index: 3,
			})))),
		};

		let expected = "(* (- 123) (group 45.67))";

		let mut actual = String::new();
		super::print_ast(&expr, &mut actual).unwrap();

		assert_eq!(expected, actual);
	}
}
