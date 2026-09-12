//! Tests for the precedence groups.

use super::group::{Group, group, unary_group};
use crate::ast;

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
