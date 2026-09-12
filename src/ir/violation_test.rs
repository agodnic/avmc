//! Tests for the messages violations render.

use super::inst::ValueId;
use super::violation::Violation;
use crate::typed_ast;

#[test]
fn violations_describe_themselves() {
    assert_eq!(
        Violation::UseOutOfOrder {
            index: 3,
            position: 0,
            value: ValueId(1),
            expected: ValueId(0),
        }
        .to_string(),
        "consumed in stack order: instruction 3 uses %1 as operand 0, expected %0"
    );
    assert_eq!(
        Violation::StackUnderflow {
            index: 1,
            needed: 2,
            available: 1,
        }
        .to_string(),
        "consumed in stack order: instruction 1 needs 2 operands but 1 values are live"
    );
    assert_eq!(
        Violation::ValuesLeftOnStack { count: 1 }.to_string(),
        "consumed in stack order: 1 values are left unconsumed"
    );
    assert_eq!(
        Violation::FrameTooLarge { count: 129 }.to_string(),
        "addressed within the frame: 129 locals exceed the capacity of 128"
    );
    assert_eq!(
        Violation::LocalOutOfRange {
            index: 3,
            local: typed_ast::LocalId(2),
            count: 2,
        }
        .to_string(),
        "addressed within the frame: instruction 3 names l2 but the frame has 2 locals"
    );
    assert_eq!(
        Violation::OperandType {
            index: 2,
            position: 0,
            value: ValueId(0),
            found: typed_ast::Type::Bool,
            expected: typed_ast::Type::Uint64,
        }
        .to_string(),
        "well typed: instruction 2 uses %0 of type bool as operand 0, expected uint64"
    );
}
