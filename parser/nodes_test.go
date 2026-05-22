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

// mustParse is a helper that parses a grammar element of type T from source code
func mustParse[T any](t *testing.T, sourceCode string) *T {

	parser, err := participle.Build[T](
		participle.Unquote("String"),
		participle.Union[Stmt](ReturnStmt{}, VariableDeclarationStmt{}),
	)
	assert.NoError(t, err)

	result, err := parser.ParseString("", sourceCode)
	assert.NoError(t, err)

	return result
}

func Test_VariableDeclarationStmt(t *testing.T) {

	testCases := []TestCase[VariableDeclarationStmt]{
		{
			Name:       "naive case",
			SourceCode: "var a = 1",
			Expected: VariableDeclarationStmt{
				Ident: "a",
				Expr:  "1",
			},
		},
	}

	testAll(t, testCases)
}

func Test_Stmt(t *testing.T) {

	testCases := []TestCase[Stmt]{
		{
			Name:       `return stmt`,
			SourceCode: `return 1`,
			Expected:   ReturnStmt{UInt: 1},
		},
		{
			Name:       `variable declaration stmt`,
			SourceCode: `var i = 1`,
			Expected:   VariableDeclarationStmt{Ident: "i", Expr: "1"},
		},
	}

	testAll(t, testCases)
}

func Test_ReturnStmt(t *testing.T) {

	testCases := []TestCase[ReturnStmt]{
		{
			Name:       `just return 0`,
			SourceCode: `return 0`,
			Expected:   ReturnStmt{UInt: 0},
		},
		{
			Name:       `just return 1`,
			SourceCode: `return 1`,
			Expected:   ReturnStmt{UInt: 1},
		},
	}

	testAll(t, testCases)
}

func Test_FuncParam(t *testing.T) {

	testCases := []TestCase[FuncParam]{
		{
			Name:       "Naive case",
			SourceCode: "j int",
			Expected: FuncParam{
				Ident: "j",
				Type:  "int",
			},
		},
	}

	testAll(t, testCases)
}

func Test_FuncDeclaration(t *testing.T) {

	testCases := []TestCase[FuncDeclaration]{
		{
			Name: "Naive case",
			SourceCode: `
				func main(i int) int {
					return 0
				}
			`,
			Expected: FuncDeclaration{
				Name: "main",
				FunctionParameters: []FuncParam{
					{
						Ident: "i",
						Type:  "int",
					},
				},
				ReturnType: "int",
				Stmts: []Stmt{
					ReturnStmt{
						UInt: 0,
					},
				},
			},
		},
	}

	testAll(t, testCases)
}
