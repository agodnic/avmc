//! The language's expression precedence: a partial order over operator groups.
//!
//! Two operators the order does not relate are a compile error rather than a
//! silent grouping, so this module knows nothing about tokens or parsing.

use crate::ast::BinaryOp;

/// A node in the precedence graph. Operators in one group share a precedence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Group {
    /// `+` and `-`.
    Additive,
    /// `*` and `/`.
    Multiplicative,
    /// `%`.
    Modulo,
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
pub fn group(op: BinaryOp) -> Group {
    match op {
        BinaryOp::Add | BinaryOp::Sub => Group::Additive,
        BinaryOp::Mul | BinaryOp::Div => Group::Multiplicative,
        BinaryOp::Mod => Group::Modulo,
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
        (Group::Multiplicative, Group::Additive) => Priority::Left,
        (Group::Multiplicative, Group::Multiplicative) => Priority::Left,
        (Group::Multiplicative, Group::Modulo) => Priority::Ambiguous,
        (Group::Modulo, Group::Additive) => Priority::Ambiguous,
        (Group::Modulo, Group::Multiplicative) => Priority::Ambiguous,
        (Group::Modulo, Group::Modulo) => Priority::Ambiguous,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GROUPS: [Group; 3] = [Group::Additive, Group::Multiplicative, Group::Modulo];

    #[test]
    fn every_operator_has_its_group() {
        assert_eq!(group(BinaryOp::Add), Group::Additive);
        assert_eq!(group(BinaryOp::Sub), Group::Additive);
        assert_eq!(group(BinaryOp::Mul), Group::Multiplicative);
        assert_eq!(group(BinaryOp::Div), Group::Multiplicative);
        assert_eq!(group(BinaryOp::Mod), Group::Modulo);
    }

    #[test]
    fn the_table_is_what_the_design_says() {
        use Group::{Additive, Modulo, Multiplicative};
        use Priority::{Ambiguous, Left, Right};

        assert_eq!(priority(Additive, Additive), Left);
        assert_eq!(priority(Additive, Multiplicative), Right);
        assert_eq!(priority(Additive, Modulo), Ambiguous);
        assert_eq!(priority(Multiplicative, Additive), Left);
        assert_eq!(priority(Multiplicative, Multiplicative), Left);
        assert_eq!(priority(Multiplicative, Modulo), Ambiguous);
        assert_eq!(priority(Modulo, Additive), Ambiguous);
        assert_eq!(priority(Modulo, Multiplicative), Ambiguous);
        assert_eq!(priority(Modulo, Modulo), Ambiguous);
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
}
