//! Tests for flattening the CST back into its tokens.

use super::flatten::tokens;
use super::node::Program;
use crate::diag;
use crate::lexer;
use crate::parser;

/// The approval program, with comments.
const COMMENTED_APPROVAL: &str = "// The approval program.\nfunc approval() uint64 {\n  var x uint64 = 1 + 2 // one more than two\n  return x\n}\n";

/// The example program of the parameters milestone.
const PARAMETERS: &str = "func add(a uint64, b uint64) uint64 {\n\tvar sum uint64 = a + b\n\t\
                          return sum\n}\n\nfunc approval() uint64 {\n\treturn 1\n}\n";

/// The example program of the calls milestone.
const CALLS: &str = "func approval() uint64 {\n\treturn add(1, double(2))\n}\n\n\
                     func add(a uint64, b uint64) uint64 {\n\treturn a + b\n}\n\n\
                     func double(x uint64) uint64 {\n\treturn x * 2\n}\n";

/// The example program of the variables milestone.
const VARIABLES: &str = "func approval() uint64 {\n  var x uint64 = 1 + 2\n  \
                         var y uint64 = x * 3\n  return y - x\n}\n";

/// Lexes and parses `source`, asserting that both succeeded without
/// diagnostics.
fn parse_cst(source: &str) -> Program {
    let mut diags = diag::Sink::default();
    let tokens = lexer::lex(source, &mut diags).expect("lexing succeeded");
    let program = parser::parse(&tokens, &mut diags).expect("parsing succeeded");
    assert!(diags.is_empty());
    program
}

#[test]
fn the_tree_is_lossless() {
    let sources = [
        COMMENTED_APPROVAL,
        VARIABLES,
        PARAMETERS,
        CALLS,
        "func f() uint64 { return (1 + 2) * 3 // grouped\n}\n",
    ];

    for source in sources {
        let mut diags = diag::Sink::default();
        let lexed = lexer::lex(source, &mut diags).expect("lexing succeeded");
        let tokens = tokens(&parse_cst(source));
        assert_eq!(tokens, lexed, "{source}");

        let text = |span: diag::Span| source.get(span.start..span.end).unwrap_or("");
        let reassembled: String = tokens
            .iter()
            .map(|token| format!("{}{}", text(token.trivia), text(token.span)))
            .collect();
        assert_eq!(reassembled, source);
    }
}
