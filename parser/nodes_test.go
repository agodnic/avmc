package parser

import (
	"testing"

	"github.com/alecthomas/participle/v2"
	"github.com/stretchr/testify/assert"
)

// TestCase defines a single input-output test case for a grammar element T
type TestCase[T any] struct {
	Name       string
	SourceCode string
	Error      bool
	Expected   T
}

// testAll is a helper that runs all the test cases defined in a given table
//
// This is meant to be used in a table-driven test style.
func testAll[T any](t *testing.T, testCases []TestCase[T]) {

	for _, testCase := range testCases {
		t.Run(testCase.Name, func(t *testing.T) {
			returnStmt := mustParse[T](t, testCase.SourceCode)
			assert.Equal(t, testCase.Expected, *returnStmt)
		})
	}
}

// // mustParse is a helper that parses a grammar element of type T from source code
func mustParse[T any](t *testing.T, sourceCode string) *T {

	parser, err := participle.Build[T](
		participle.Unquote("String"), //FIXME figure out what this does
		//participle.Union[Stmt](AssignmentStmt{}, ReturnStmt{}, VariableDeclarationStmt{}),
		//participle.Union[Expr](CallExpr{}, IntegerExpr{}, StringExpr{}, UnaryExpr{}, VarExpr{}),
	)
	assert.NoError(t, err)

	result, err := parser.ParseString("", sourceCode)
	assert.NoError(t, err)

	return result
}

func Test_Primary(t *testing.T) {

	testCases := []TestCase[Primary]{
		{
			Name:       "number",
			SourceCode: `1`,
			Expected: Primary{
				Number: ptr(1),
			},
		},
		{
			Name:       "string",
			SourceCode: `"s"`,
			Expected: Primary{
				String: ptr("s"),
			},
		},
		{
			Name:       "ident",
			SourceCode: `a`,
			Expected: Primary{
				Ident: ptr("a"),
			},
		},
		{
			Name:       "subexpr",
			SourceCode: `(1)`,
			Expected: Primary{
				Subexpr: intPrimaryExpr(1),
			},
		},
	}

	testAll(t, testCases)
}

func Test_Postfix(t *testing.T) {

	testCases := []TestCase[Postfix]{
		{
			Name:       "number",
			SourceCode: `1`,
			Expected: Postfix{
				Primary: &Primary{Number: ptr(1)},
			},
		},
		{
			Name:       "ident",
			SourceCode: `a`,
			Expected: Postfix{
				Primary: &Primary{Ident: ptr("a")},
			},
		},
		{
			Name:       "call no args",
			SourceCode: `foo()`,
			Expected: Postfix{
				Primary: &Primary{Ident: ptr("foo")},
				Calls: []*Call{
					{Args: nil},
				},
			},
		},
		{
			Name:       "call one arg",
			SourceCode: `foo(1)`,
			Expected: Postfix{
				Primary: &Primary{Ident: ptr("foo")},
				Calls: []*Call{
					{Args: []*Expr{intPrimaryExpr(1)}},
				},
			},
		},
		{
			Name:       "call two args",
			SourceCode: `foo(1, 2)`,
			Expected: Postfix{
				Primary: &Primary{Ident: ptr("foo")},
				Calls: []*Call{
					{Args: []*Expr{intPrimaryExpr(1), intPrimaryExpr(2)}},
				},
			},
		},
	}

	testAll(t, testCases)
}

func primaryExpr(p Primary) *Expr {
	return &Expr{
		Equality: &Equality{
			Left: &Additive{
				Left: &Multiplicative{
					Left: &Unary{
						Operand: &Postfix{Primary: &p},
					},
				},
			},
		},
	}
}

func intPrimaryExpr(n int) *Expr {
	return primaryExpr(Primary{Number: ptr(n)})
}

func stringPrimaryExpr(s string) *Expr {
	return primaryExpr(Primary{String: ptr(s)})
}

func identPrimaryExpr(name string) *Expr {
	return primaryExpr(Primary{Ident: ptr(name)})
}

func subexprPrimaryExpr(inner *Expr) *Expr {
	return primaryExpr(Primary{Subexpr: inner})
}

func ptr[T any](t T) *T {
	return &t
}
