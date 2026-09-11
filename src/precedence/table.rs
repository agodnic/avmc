use super::group::Group;

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
