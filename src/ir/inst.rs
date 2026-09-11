use crate::ast;
use crate::diag;
use crate::typed_ast;

/// The value a defining instruction produces. Numbered per function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValueId(pub u32);

/// A single instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inst {
    /// Defines `dest` as the constant `value` of type `ty`.
    Const {
        /// The value it defines.
        dest: ValueId,
        /// The type it has.
        ty: typed_ast::Type,
        /// The constant it holds: for a `Bool`, `0` or `1`.
        value: u64,
        /// The literal it came from.
        span: diag::Span,
    },
    /// Defines `dest` as `lhs op rhs`.
    Binary {
        /// The value it defines.
        dest: ValueId,
        /// The operator it applies.
        op: ast::BinOp,
        /// The left operand, consumed first.
        lhs: ValueId,
        /// The right operand, consumed second.
        rhs: ValueId,
        /// The expression it came from.
        span: diag::Span,
    },
    /// Defines `dest` as `op operand`.
    Unary {
        /// The value it defines.
        dest: ValueId,
        /// The operator it applies.
        op: ast::UnOp,
        /// The operand, consumed.
        operand: ValueId,
        /// The expression it came from.
        span: diag::Span,
    },
    /// Writes `value` into frame slot `local`.
    Store {
        /// The slot it writes.
        local: typed_ast::LocalId,
        /// The value it writes, consumed.
        value: ValueId,
        /// The declaration it came from.
        span: diag::Span,
    },
    /// Defines `dest` as a copy of frame slot `local`.
    Load {
        /// The value it defines.
        dest: ValueId,
        /// The slot it reads.
        local: typed_ast::LocalId,
        /// The expression it came from.
        span: diag::Span,
    },
    /// Returns `value` from the enclosing function.
    Return {
        /// The value it returns.
        value: ValueId,
        /// The `return` statement it came from.
        span: diag::Span,
    },
}

impl Inst {
    /// The source it came from.
    pub fn span(&self) -> diag::Span {
        match self {
            Inst::Const { span, .. }
            | Inst::Binary { span, .. }
            | Inst::Unary { span, .. }
            | Inst::Store { span, .. }
            | Inst::Load { span, .. }
            | Inst::Return { span, .. } => *span,
        }
    }
}

/// A function's body, as instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    /// The declared name.
    pub name: String,
    /// The return type.
    pub ret: typed_ast::Type,
    /// The frame: one slot per variable, indexed by `LocalId`.
    pub locals: Vec<typed_ast::Type>,
    /// The instructions, in execution order.
    pub insts: Vec<Inst>,
    /// From `func` through the closing `}`.
    pub span: diag::Span,
}

/// A whole compilation unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Program {
    /// The functions it defines, in source order.
    pub funcs: Vec<Function>,
}
