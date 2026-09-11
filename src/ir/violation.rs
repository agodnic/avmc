use super::inst::ValueId;
use crate::typed_ast;

/// A way a function can fail the v0 IR invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    /// An instruction defines a value out of the dense numbering.
    SparseDefinition {
        /// The offending instruction's position.
        index: usize,
        /// The value it defines.
        dest: ValueId,
        /// The value it should have defined.
        expected: ValueId,
    },
    /// An instruction names an operand that is not the one on top of the
    /// stack.
    UseOutOfOrder {
        /// The offending instruction's position.
        index: usize,
        /// Which operand it is, counting from 0.
        position: usize,
        /// The value it uses.
        value: ValueId,
        /// The value it should have used.
        expected: ValueId,
    },
    /// An instruction names more operands than there are live values.
    StackUnderflow {
        /// The offending instruction's position.
        index: usize,
        /// How many operands it names.
        needed: usize,
        /// How many values are live.
        available: usize,
    },
    /// The function ends with values that nothing consumed.
    ValuesLeftOnStack {
        /// How many of them there are.
        count: usize,
    },
    /// An instruction returns without being the last one.
    ReturnNotLast {
        /// The offending instruction's position.
        index: usize,
    },
    /// The function's last instruction is not a `Return`.
    MissingReturn,
    /// The frame holds more slots than one can address.
    FrameTooLarge {
        /// How many slots it holds.
        count: usize,
    },
    /// An instruction names a slot the frame does not have.
    LocalOutOfRange {
        /// The offending instruction's position.
        index: usize,
        /// The slot it names.
        local: typed_ast::LocalId,
        /// How many slots the frame holds.
        count: usize,
    },
    /// The function takes more parameters than one can address.
    TooManyParams {
        /// How many it takes.
        count: usize,
    },
    /// An instruction names a parameter the function does not take.
    ParamOutOfRange {
        /// The offending instruction's position.
        index: usize,
        /// The parameter it names.
        param: typed_ast::ParamId,
        /// How many parameters the function takes.
        count: usize,
    },
    /// An instruction uses an operand of the wrong type.
    OperandType {
        /// The offending instruction's position.
        index: usize,
        /// Which operand it is, counting from 0.
        position: usize,
        /// The value it uses.
        value: ValueId,
        /// The type that value has.
        found: typed_ast::Type,
        /// The type the instruction needs.
        expected: typed_ast::Type,
    },
    /// An instruction calls a function the program does not define.
    UnknownCallee {
        /// The offending instruction's position.
        index: usize,
        /// The function it calls.
        callee: typed_ast::FuncId,
    },
    /// A `Bool` constant holds something other than `0` or `1`.
    BoolOutOfRange {
        /// The offending instruction's position.
        index: usize,
        /// The constant it holds.
        value: u64,
    },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::SparseDefinition {
                index,
                dest,
                expected,
            } => write!(
                f,
                "dense single assignment: instruction {index} defines %{}, expected %{}",
                dest.0, expected.0
            ),
            Violation::UseOutOfOrder {
                index,
                position,
                value,
                expected,
            } => write!(
                f,
                "consumed in stack order: instruction {index} uses %{} as operand {position}, \
                 expected %{}",
                value.0, expected.0
            ),
            Violation::StackUnderflow {
                index,
                needed,
                available,
            } => write!(
                f,
                "consumed in stack order: instruction {index} needs {needed} operands but \
                 {available} values are live"
            ),
            Violation::ValuesLeftOnStack { count } => write!(
                f,
                "consumed in stack order: {count} values are left unconsumed"
            ),
            Violation::ReturnNotLast { index } => write!(
                f,
                "ends with `Return`: instruction {index} returns but is not the last"
            ),
            Violation::MissingReturn => write!(
                f,
                "ends with `Return`: the function does not end with a return"
            ),
            Violation::FrameTooLarge { count } => write!(
                f,
                "addressed within the frame: {count} locals exceed the capacity of {}",
                typed_ast::LocalId::CAPACITY
            ),
            Violation::LocalOutOfRange {
                index,
                local,
                count,
            } => write!(
                f,
                "addressed within the frame: instruction {index} names l{} but the frame has \
                 {count} locals",
                local.0
            ),
            Violation::TooManyParams { count } => write!(
                f,
                "addressed within the frame: {count} parameters exceed the capacity of {}",
                typed_ast::ParamId::CAPACITY
            ),
            Violation::ParamOutOfRange {
                index,
                param,
                count,
            } => write!(
                f,
                "addressed within the frame: instruction {index} names p{} but the function takes \
                 {count} parameters",
                param.0
            ),
            Violation::OperandType {
                index,
                position,
                value,
                found,
                expected,
            } => write!(
                f,
                "well typed: instruction {index} uses %{} of type {found} as operand {position}, \
                 expected {expected}",
                value.0
            ),
            Violation::UnknownCallee { index, callee } => write!(
                f,
                "well typed: instruction {index} calls f{} but the program does not define it",
                callee.0
            ),
            Violation::BoolOutOfRange { index, value } => write!(
                f,
                "well typed: instruction {index} defines a bool constant of {value}, expected 0 \
                 or 1"
            ),
        }
    }
}
