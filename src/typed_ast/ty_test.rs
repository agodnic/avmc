//! Tests for the operator typing rules.

use super::ty::{self, Type};
use crate::ast;

#[test]
fn arithmetic_takes_and_produces_uint64() {
    for op in [
        ast::BinOp::Add,
        ast::BinOp::Sub,
        ast::BinOp::Mul,
        ast::BinOp::Div,
        ast::BinOp::Mod,
    ] {
        assert_eq!(ty::operand_type(op), Some(Type::Uint64), "{op:?}");
        assert_eq!(ty::result_type(op), Type::Uint64, "{op:?}");
    }
}

#[test]
fn equality_takes_operands_that_agree() {
    for op in [ast::BinOp::Eq, ast::BinOp::Ne] {
        assert_eq!(ty::operand_type(op), None, "{op:?}");
        assert_eq!(ty::result_type(op), Type::Bool, "{op:?}");
    }
}

#[test]
fn ordering_takes_uint64_and_produces_bool() {
    for op in [
        ast::BinOp::Lt,
        ast::BinOp::Le,
        ast::BinOp::Gt,
        ast::BinOp::Ge,
    ] {
        assert_eq!(ty::operand_type(op), Some(Type::Uint64), "{op:?}");
        assert_eq!(ty::result_type(op), Type::Bool, "{op:?}");
    }
}

#[test]
fn logic_takes_and_produces_bool() {
    for op in [ast::BinOp::And, ast::BinOp::Or] {
        assert_eq!(ty::operand_type(op), Some(Type::Bool), "{op:?}");
        assert_eq!(ty::result_type(op), Type::Bool, "{op:?}");
    }
}
