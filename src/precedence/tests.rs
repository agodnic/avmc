//! Tests for the precedence groups and the order over them.

use super::group::{Group, group, unary_group};
use super::table::{Priority, priority};
use crate::ast;

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
