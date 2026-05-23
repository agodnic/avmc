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

//func Test_VariableDeclarationStmt(t *testing.T) {
//
//	testCases := []TestCase[VariableDeclarationStmt]{
//		{
//			Name:       "naive case",
//			SourceCode: "var a int = 1",
//			Expected: VariableDeclarationStmt{
//				Ident: "a",
//				Type:  "int",
//				Expr:  IntegerExpr{Value: 1},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_Stmt(t *testing.T) {
//
//	testCases := []TestCase[Stmt]{
//		{
//			Name:       `assignment stmt`,
//			SourceCode: `a = 1`,
//			Expected: AssignmentStmt{
//				Ident: "a",
//				Value: IntegerExpr{Value: 1},
//			},
//		},
//		{
//			Name:       `return stmt`,
//			SourceCode: `return 1`,
//			Expected:   ReturnStmt{Value: IntegerExpr{Value: 1}},
//		},
//		{
//			Name:       `variable declaration stmt`,
//			SourceCode: `var s string = "foo"`,
//			Expected: VariableDeclarationStmt{
//				Ident: "s",
//				Type:  "string",
//				Expr:  StringExpr{Value: "foo"},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_AssignmentStmt(t *testing.T) {
//
//	testCases := []TestCase[AssignmentStmt]{
//		{
//			Name:       `assignment stmt`,
//			SourceCode: `a = 1`,
//			Expected: AssignmentStmt{
//				Ident: "a",
//				Value: IntegerExpr{Value: 1},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_ReturnStmt(t *testing.T) {
//
//	testCases := []TestCase[ReturnStmt]{
//		{
//			Name:       `just return 0`,
//			SourceCode: `return 0`,
//			Expected:   ReturnStmt{Value: IntegerExpr{Value: 0}},
//		},
//		{
//			Name:       `just return 1`,
//			SourceCode: `return 1`,
//			Expected:   ReturnStmt{Value: IntegerExpr{Value: 1}},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_FuncParam(t *testing.T) {
//
//	testCases := []TestCase[FuncParam]{
//		{
//			Name:       "Naive case",
//			SourceCode: "j int",
//			Expected: FuncParam{
//				Ident: "j",
//				Type:  "int",
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_FuncDeclaration(t *testing.T) {
//
//	testCases := []TestCase[FuncDeclaration]{
//		{
//			Name: "Naive case",
//			SourceCode: `
//				func main(i int) int {
//					return 0
//				}
//			`,
//			Expected: FuncDeclaration{
//				Name: "main",
//				FuncParams: []FuncParam{
//					{
//						Ident: "i",
//						Type:  "int",
//					},
//				},
//				ReturnType: "int",
//				Stmts: []Stmt{
//					ReturnStmt{
//						Value: IntegerExpr{Value: 0},
//					},
//				},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_CallExpr(t *testing.T) {
//
//	testCases := []TestCase[CallExpr]{
//		{
//			Name:       "call expression with one param",
//			SourceCode: `strlen("foo")`,
//			Expected: CallExpr{
//				Ident: "strlen",
//				Params: []Expr{
//					StringExpr{Value: "foo"},
//				},
//			},
//		},
//		{
//			Name:       "call expression with two params",
//			SourceCode: `printf("number: %d", 1)`,
//			Expected: CallExpr{
//				Ident: "printf",
//				Params: []Expr{
//					StringExpr{Value: "number: %d"},
//					IntegerExpr{Value: 1},
//				},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_IntegerExpr(t *testing.T) {
//
//	testCases := []TestCase[IntegerExpr]{
//		{
//			Name:       "int literal",
//			SourceCode: `42`,
//			Expected:   IntegerExpr{Value: 42},
//		},
//	}
//
//	testAll(t, testCases)
//}
//func Test_StringExpr(t *testing.T) {
//
//	testCases := []TestCase[StringExpr]{
//		{
//			Name:       "string literal",
//			SourceCode: `"this is a string literal"`,
//			Expected:   StringExpr{Value: "this is a string literal"},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_UnaryExpr(t *testing.T) {
//
//	testCases := []TestCase[UnaryExpr]{
//		{
//			Name:       "unary expression with variable expression",
//			SourceCode: `!a`,
//			Expected: UnaryExpr{
//				Operand: VarExpr{
//					Ident: "a",
//				},
//			},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_VarExpr(t *testing.T) {
//
//	testCases := []TestCase[VarExpr]{
//		{
//			Name:       "variable expression",
//			SourceCode: `a`,
//			Expected:   VarExpr{Ident: "a"},
//		},
//	}
//
//	testAll(t, testCases)
//}
//
//func Test_Expr(t *testing.T) {
//
//	testCases := []TestCase[Expr]{
//		{
//			Name:       "function call expression",
//			SourceCode: `printf("number: %d", 1)`,
//			Expected: CallExpr{
//				Ident: "printf",
//				Params: []Expr{
//					StringExpr{Value: "number: %d"},
//					IntegerExpr{Value: 1},
//				},
//			},
//		},
//		{
//			Name:       "int literal",
//			SourceCode: `42`,
//			Expected:   IntegerExpr{Value: 42},
//		},
//		{
//			Name:       "string literal",
//			SourceCode: `"this is a string literal"`,
//			Expected:   StringExpr{Value: "this is a string literal"},
//		},
//		{
//			Name:       "unary expression with variable expression",
//			SourceCode: `!a`,
//			Expected: UnaryExpr{
//				Operand: VarExpr{
//					Ident: "a",
//				},
//			},
//		},
//		{
//			Name:       "variable expression",
//			SourceCode: `a`,
//			Expected:   VarExpr{Ident: "a"},
//		},
//	}
//
//	testAll(t, testCases)
//}
//

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
				Subexpr: &Expr{
					Equality: &Equality{
						Left: &Additive{
							Left: &Multiplicative{
								Left: &Unary{
									Operand: &Postfix{
										Primary: &Primary{
											Number: ptr(1),
										},
									},
								},
							},
						},
					},
				},
			},
		},
	}

	testAll(t, testCases)
}

func ptr[T any](t T) *T {
	return &t
}
