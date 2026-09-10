//! The language's expression precedence: a partial order over operator groups.
//!
//! Two operators the order does not relate are a compile error rather than a
//! silent grouping, so this module knows nothing about tokens or parsing.

use crate::ast;

/// A node in the precedence graph. Operators in one group share a precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// `+` and `-`.
    Additive,
    /// `*` and `/`.
    Multiplicative,
    /// `%`.
    Modulo,
    /// `==`, `!=`, `<`, `<=`, `>`, and `>=`.
    Comparison,
    /// `!`.
    Not,
    /// `&&`.
    And,
    /// `||`.
    Or,
}

/// Which of two adjacent operators binds tighter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    /// The enclosing operator binds tighter.
    Left,
    /// The following operator binds tighter.
    Right,
    /// The two are unordered; the source must parenthesize.
    Ambiguous,
}

/// The group an operator belongs to.
pub fn group(op: ast::BinOp) -> Group {
    match op {
        ast::BinOp::Add | ast::BinOp::Sub => Group::Additive,
        ast::BinOp::Mul | ast::BinOp::Div => Group::Multiplicative,
        ast::BinOp::Mod => Group::Modulo,
        ast::BinOp::Eq
        | ast::BinOp::Ne
        | ast::BinOp::Lt
        | ast::BinOp::Le
        | ast::BinOp::Gt
        | ast::BinOp::Ge => Group::Comparison,
        ast::BinOp::And => Group::And,
        ast::BinOp::Or => Group::Or,
    }
}

/// The group a prefix operator belongs to.
pub fn unary_group(op: ast::UnOp) -> Group {
    match op {
        ast::UnOp::Not => Group::Not,
    }
}

/// How `left`, the enclosing operator, binds against `right`, the operator
/// that follows it.
///
/// `Left` on the diagonal is left-associativity; `Ambiguous` on it is
/// non-associativity.
pub fn priority(left: Group, right: Group) -> Priority {
    // Exhaustive, with no wildcard arm, so that a new group does not compile
    // until it has been placed against every other one.
    match (left, right) {
        (Group::Additive, Group::Additive) => Priority::Left,
        (Group::Additive, Group::Multiplicative) => Priority::Right,
        (Group::Additive, Group::Modulo) => Priority::Ambiguous,
        (Group::Additive, Group::Comparison) => Priority::Left,
        (Group::Additive, Group::Not) => Priority::Ambiguous,
        (Group::Additive, Group::And) => Priority::Left,
        (Group::Additive, Group::Or) => Priority::Left,
        (Group::Multiplicative, Group::Additive) => Priority::Left,
        (Group::Multiplicative, Group::Multiplicative) => Priority::Left,
        (Group::Multiplicative, Group::Modulo) => Priority::Ambiguous,
        (Group::Multiplicative, Group::Comparison) => Priority::Left,
        (Group::Multiplicative, Group::Not) => Priority::Ambiguous,
        (Group::Multiplicative, Group::And) => Priority::Left,
        (Group::Multiplicative, Group::Or) => Priority::Left,
        (Group::Modulo, Group::Additive) => Priority::Ambiguous,
        (Group::Modulo, Group::Multiplicative) => Priority::Ambiguous,
        (Group::Modulo, Group::Modulo) => Priority::Ambiguous,
        (Group::Modulo, Group::Comparison) => Priority::Left,
        (Group::Modulo, Group::Not) => Priority::Ambiguous,
        (Group::Modulo, Group::And) => Priority::Left,
        (Group::Modulo, Group::Or) => Priority::Left,
        (Group::Comparison, Group::Additive) => Priority::Right,
        (Group::Comparison, Group::Multiplicative) => Priority::Right,
        (Group::Comparison, Group::Modulo) => Priority::Right,
        (Group::Comparison, Group::Comparison) => Priority::Ambiguous,
        (Group::Comparison, Group::Not) => Priority::Ambiguous,
        (Group::Comparison, Group::And) => Priority::Left,
        (Group::Comparison, Group::Or) => Priority::Left,
        (Group::Not, Group::Additive) => Priority::Ambiguous,
        (Group::Not, Group::Multiplicative) => Priority::Ambiguous,
        (Group::Not, Group::Modulo) => Priority::Ambiguous,
        (Group::Not, Group::Comparison) => Priority::Ambiguous,
        (Group::Not, Group::Not) => Priority::Ambiguous,
        (Group::Not, Group::And) => Priority::Left,
        (Group::Not, Group::Or) => Priority::Left,
        (Group::And, Group::Additive) => Priority::Right,
        (Group::And, Group::Multiplicative) => Priority::Right,
        (Group::And, Group::Modulo) => Priority::Right,
        (Group::And, Group::Comparison) => Priority::Right,
        (Group::And, Group::Not) => Priority::Right,
        (Group::And, Group::And) => Priority::Left,
        (Group::And, Group::Or) => Priority::Ambiguous,
        (Group::Or, Group::Additive) => Priority::Right,
        (Group::Or, Group::Multiplicative) => Priority::Right,
        (Group::Or, Group::Modulo) => Priority::Right,
        (Group::Or, Group::Comparison) => Priority::Right,
        (Group::Or, Group::Not) => Priority::Right,
        (Group::Or, Group::And) => Priority::Ambiguous,
        (Group::Or, Group::Or) => Priority::Left,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUPS: [Group; 7] = [
        Group::Additive,
        Group::Multiplicative,
        Group::Modulo,
        Group::Comparison,
        Group::Not,
        Group::And,
        Group::Or,
    ];

    #[test]
    fn every_operator_has_its_group() {
        assert_eq!(group(ast::BinOp::Add), Group::Additive);
        assert_eq!(group(ast::BinOp::Sub), Group::Additive);
        assert_eq!(group(ast::BinOp::Mul), Group::Multiplicative);
        assert_eq!(group(ast::BinOp::Div), Group::Multiplicative);
        assert_eq!(group(ast::BinOp::Mod), Group::Modulo);
        for op in [
            ast::BinOp::Eq,
            ast::BinOp::Ne,
            ast::BinOp::Lt,
            ast::BinOp::Le,
            ast::BinOp::Gt,
            ast::BinOp::Ge,
        ] {
            assert_eq!(group(op), Group::Comparison, "{op:?}");
        }
        assert_eq!(group(ast::BinOp::And), Group::And);
        assert_eq!(group(ast::BinOp::Or), Group::Or);
        assert_eq!(unary_group(ast::UnOp::Not), Group::Not);
    }

    #[test]
    fn the_table_is_what_the_design_says() {
        use Group::{Additive, And, Comparison, Modulo, Multiplicative, Not, Or};
        use Priority::{Ambiguous, Left, Right};

        assert_eq!(priority(Additive, Additive), Left);
        assert_eq!(priority(Additive, Multiplicative), Right);
        assert_eq!(priority(Additive, Modulo), Ambiguous);
        assert_eq!(priority(Additive, Comparison), Left);
        assert_eq!(priority(Additive, Not), Ambiguous);
        assert_eq!(priority(Additive, And), Left);
        assert_eq!(priority(Additive, Or), Left);
        assert_eq!(priority(Multiplicative, Additive), Left);
        assert_eq!(priority(Multiplicative, Multiplicative), Left);
        assert_eq!(priority(Multiplicative, Modulo), Ambiguous);
        assert_eq!(priority(Multiplicative, Comparison), Left);
        assert_eq!(priority(Multiplicative, Not), Ambiguous);
        assert_eq!(priority(Multiplicative, And), Left);
        assert_eq!(priority(Multiplicative, Or), Left);
        assert_eq!(priority(Modulo, Additive), Ambiguous);
        assert_eq!(priority(Modulo, Multiplicative), Ambiguous);
        assert_eq!(priority(Modulo, Modulo), Ambiguous);
        assert_eq!(priority(Modulo, Comparison), Left);
        assert_eq!(priority(Modulo, Not), Ambiguous);
        assert_eq!(priority(Modulo, And), Left);
        assert_eq!(priority(Modulo, Or), Left);
        assert_eq!(priority(Comparison, Additive), Right);
        assert_eq!(priority(Comparison, Multiplicative), Right);
        assert_eq!(priority(Comparison, Modulo), Right);
        assert_eq!(priority(Comparison, Comparison), Ambiguous);
        assert_eq!(priority(Comparison, Not), Ambiguous);
        assert_eq!(priority(Comparison, And), Left);
        assert_eq!(priority(Comparison, Or), Left);
        assert_eq!(priority(Not, Additive), Ambiguous);
        assert_eq!(priority(Not, Multiplicative), Ambiguous);
        assert_eq!(priority(Not, Modulo), Ambiguous);
        assert_eq!(priority(Not, Comparison), Ambiguous);
        assert_eq!(priority(Not, Not), Ambiguous);
        assert_eq!(priority(Not, And), Left);
        assert_eq!(priority(Not, Or), Left);
        assert_eq!(priority(And, Additive), Right);
        assert_eq!(priority(And, Multiplicative), Right);
        assert_eq!(priority(And, Modulo), Right);
        assert_eq!(priority(And, Comparison), Right);
        assert_eq!(priority(And, Not), Right);
        assert_eq!(priority(And, And), Left);
        assert_eq!(priority(And, Or), Ambiguous);
        assert_eq!(priority(Or, Additive), Right);
        assert_eq!(priority(Or, Multiplicative), Right);
        assert_eq!(priority(Or, Modulo), Right);
        assert_eq!(priority(Or, Comparison), Right);
        assert_eq!(priority(Or, Not), Right);
        assert_eq!(priority(Or, And), Ambiguous);
        assert_eq!(priority(Or, Or), Left);
    }

    #[test]
    fn ordered_pairs_are_antisymmetric() {
        for left in GROUPS {
            for right in GROUPS {
                if priority(left, right) == Priority::Right {
                    assert_eq!(priority(right, left), Priority::Left, "{left:?} {right:?}");
                }
            }
        }
    }

    #[test]
    fn ambiguity_is_symmetric() {
        for left in GROUPS {
            for right in GROUPS {
                if priority(left, right) == Priority::Ambiguous {
                    assert_eq!(
                        priority(right, left),
                        Priority::Ambiguous,
                        "{left:?} {right:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn nothing_binds_tighter_than_the_operand_of_a_prefix_operator() {
        for left in GROUPS {
            assert_ne!(priority(left, Group::Not), Priority::Left, "{left:?}");
        }
    }
}
