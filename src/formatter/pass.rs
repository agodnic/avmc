use crate::cst;
use crate::diag;
use crate::lexer;
use crate::parser;

/// Formats `source`, or reports why it cannot be parsed.
pub fn format(source: &str, diags: &mut diag::Sink) -> Option<String> {
    let tokens = lexer::lex(source, diags)?;
    let program = parser::parse(&tokens, diags)?;

    let mut printer = Printer {
        source,
        out: String::new(),
        indent: 0,
        state: State::Empty,
    };
    printer.program(&program);
    Some(printer.finish())
}

/// What separates a token from what precedes it on the same line.
#[derive(Clone, Copy)]
enum Sep {
    /// One space.
    Space,
    /// Nothing.
    Tight,
}

/// Which of the blank lines in a token's trivia are printed. Runs of them
/// collapse to one wherever they are kept.
#[derive(Clone, Copy)]
enum Blanks {
    /// All of them.
    All,
    /// All but the ones directly after the previous token.
    NotLeading,
    /// All but the ones directly before this token.
    NotTrailing,
    /// None.
    None,
    /// None; one blank line is printed in their place.
    Separator,
}

/// Where the printer is on the line it is writing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    /// Nothing has been printed.
    Empty,
    /// The line has text on it.
    InLine,
    /// The line is finished. Its newline is written once the next line
    /// starts, so a comment can still be appended to it.
    Ended,
}

struct Printer<'a> {
    source: &'a str,
    out: String,
    /// The number of tabs a line starts with.
    indent: usize,
    state: State,
}

impl Printer<'_> {
    fn program(&mut self, program: &cst::Program) {
        for (index, func) in program.funcs.iter().enumerate() {
            let blanks = if index == 0 {
                Blanks::None
            } else {
                Blanks::Separator
            };
            self.func(func, blanks);
        }
        // Whatever the file ends with: comments after the last function.
        self.trivia(program.eof.trivia, Blanks::NotTrailing);
    }

    fn func(&mut self, func: &cst::FuncDecl, blanks: Blanks) {
        self.token_with_blanks(&func.func, Sep::Tight, blanks);
        self.token(&func.name, Sep::Space);
        self.token(&func.lparen, Sep::Tight);
        self.token(&func.rparen, Sep::Tight);
        self.token(&func.ret, Sep::Space);
        self.token(&func.lbrace, Sep::Space);
        self.end_line();

        self.indent += 1;
        for (index, stmt) in func.body.iter().enumerate() {
            let blanks = if index == 0 {
                Blanks::NotLeading
            } else {
                Blanks::All
            };
            self.stmt(stmt, blanks);
            self.end_line();
        }

        // A comment before `}` is indented as the statements are.
        self.trivia(func.rbrace.trivia, Blanks::NotTrailing);
        self.indent -= 1;
        self.text(slice(self.source, func.rbrace.span), Sep::Tight);
        self.end_line();
    }

    /// Prints `stmt` on a line of its own. `blanks` governs the blank lines
    /// before it.
    fn stmt(&mut self, stmt: &cst::Stmt, blanks: Blanks) {
        match stmt {
            cst::Stmt::Var {
                var,
                name,
                ty,
                equals,
                init,
            } => {
                self.token_with_blanks(var, Sep::Tight, blanks);
                self.token(name, Sep::Space);
                self.token(ty, Sep::Space);
                self.token(equals, Sep::Space);
                self.expr(init, Sep::Space);
            }
            cst::Stmt::Return { ret, expr } => {
                self.token_with_blanks(ret, Sep::Tight, blanks);
                self.expr(expr, Sep::Space);
            }
        }
    }

    /// Prints `expr`, `sep` separating its first token from what precedes it.
    fn expr(&mut self, expr: &cst::Expr, sep: Sep) {
        match expr {
            cst::Expr::IntLit(token) | cst::Expr::BoolLit(token) | cst::Expr::Var(token) => {
                self.token(token, sep);
            }
            cst::Expr::Binary { lhs, op, rhs } => {
                self.expr(lhs, sep);
                self.token(op, Sep::Space);
                self.expr(rhs, Sep::Space);
            }
            cst::Expr::Unary { op, operand } => {
                self.token(op, sep);
                self.expr(operand, Sep::Tight);
            }
            cst::Expr::Paren {
                lparen,
                inner,
                rparen,
            } => {
                self.token(lparen, sep);
                self.expr(inner, Sep::Tight);
                self.token(rparen, Sep::Tight);
            }
        }
    }

    /// Prints `token` and the trivia before it, keeping its blank lines.
    fn token(&mut self, token: &lexer::Token, sep: Sep) {
        self.token_with_blanks(token, sep, Blanks::All);
    }

    fn token_with_blanks(&mut self, token: &lexer::Token, sep: Sep, blanks: Blanks) {
        self.trivia(token.trivia, blanks);
        self.text(slice(self.source, token.span), sep);
    }

    /// Prints the comments and blank lines that `trivia` covers.
    fn trivia(&mut self, trivia: diag::Span, blanks: Blanks) {
        let text = slice(self.source, trivia);
        // The head is on the previous token's line. The tail holds whole
        // lines, and after its last newline the indentation before the token.
        let (head, tail) = text.split_once('\n').unwrap_or((text, ""));
        if let Some(comment) = comment(head) {
            self.trailing_comment(comment);
        }
        if matches!(blanks, Blanks::Separator) {
            self.blank_line();
        }

        let lines: Vec<&str> = match tail.rsplit_once('\n') {
            Some((lines, _)) => lines.split('\n').collect(),
            None => Vec::new(),
        };
        let first_comment = lines.iter().position(|&line| comment(line).is_some());
        let last_comment = lines.iter().rposition(|&line| comment(line).is_some());

        // A run of blank lines is one pending blank line, printed only if
        // something follows it.
        let mut blank = false;
        for (index, &line) in lines.iter().enumerate() {
            let Some(comment) = comment(line) else {
                blank |= match blanks {
                    Blanks::All => true,
                    Blanks::NotLeading => first_comment.is_some_and(|first| index > first),
                    Blanks::NotTrailing => last_comment.is_some_and(|last| index < last),
                    Blanks::None | Blanks::Separator => false,
                };
                continue;
            };
            if blank {
                self.blank_line();
                blank = false;
            }
            self.comment_line(comment);
        }
        if blank {
            self.blank_line();
        }
    }

    /// Appends `comment` to the line already written, or starts the output
    /// with it when there is no such line.
    fn trailing_comment(&mut self, comment: &str) {
        match self.state {
            State::Empty => self.text(comment, Sep::Tight),
            State::InLine | State::Ended => {
                self.out.push(' ');
                self.out.push_str(comment);
            }
        }
        self.end_line();
    }

    /// Prints `comment` on a line of its own, at the current indentation.
    fn comment_line(&mut self, comment: &str) {
        self.end_line();
        self.text(comment, Sep::Tight);
        self.end_line();
    }

    /// Prints `text`, starting a new line if the last one is finished.
    fn text(&mut self, text: &str, sep: Sep) {
        match self.state {
            State::Empty => self.push_indent(),
            State::Ended => {
                self.out.push('\n');
                self.push_indent();
            }
            State::InLine => {
                if matches!(sep, Sep::Space) {
                    self.out.push(' ');
                }
            }
        }
        self.out.push_str(text);
        self.state = State::InLine;
    }

    /// Finishes the current line. Its newline waits for the next one.
    fn end_line(&mut self) {
        if self.state != State::Empty {
            self.state = State::Ended;
        }
    }

    /// Leaves an empty line, if there is a finished line to leave it after.
    fn blank_line(&mut self) {
        if self.state == State::Ended {
            self.out.push('\n');
        }
    }

    fn push_indent(&mut self) {
        for _ in 0..self.indent {
            self.out.push('\t');
        }
    }

    /// The formatted source: one newline ends it, unless it is empty.
    fn finish(mut self) -> String {
        if self.state != State::Empty {
            self.out.push('\n');
        }
        self.out
    }
}

/// The text `span` covers, empty if it is not a range of `source`.
fn slice(source: &str, span: diag::Span) -> &str {
    source.get(span.start..span.end).unwrap_or("")
}

/// The comment in `line`, from `//` to its end, with trailing whitespace
/// trimmed. `line` holds no newline, so the comment ends where it does.
fn comment(line: &str) -> Option<&str> {
    let start = line.find("//")?;
    Some(line.get(start..)?.trim_end())
}
