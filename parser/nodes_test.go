package parser

import (
	"testing"

	"github.com/alecthomas/participle/v2"
	"github.com/stretchr/testify/assert"
)

func mustParse[T any](t *testing.T, sourceCode string) *T {

	parser, err := participle.Build[T](
		participle.Unquote("String"),
	)
	assert.NoError(t, err)

	result, err := parser.ParseString("", sourceCode)
	assert.NoError(t, err)

	return result
}

type TestCase[T any] struct {
	Name       string
	SourceCode string
	Error      bool
	Expected   T
}

func testAll[T any](t *testing.T, testCases []TestCase[T]) {

	for _, testCase := range testCases {
		t.Run(testCase.Name, func(t *testing.T) {
			returnStmt := mustParse[T](t, testCase.SourceCode)
			assert.Equal(t, testCase.Expected, *returnStmt)
		})
	}
}

// TestReturnStmt exercises the parsing of the ReturnStmt grammar element
func TestReturnStmt(t *testing.T) {

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

// TestFunctionParameter exercises the parsing of the FunctionParameter grammar element
func TestFunctionParameter(t *testing.T) {

	testCases := []TestCase[FunctionParameter]{
		{
			Name:       "Naive case",
			SourceCode: "j int",
			Expected: FunctionParameter{
				Ident: "j",
				Type:  "int",
			},
		},
	}

	testAll(t, testCases)
}

// TODO write table-driven tests for each node
func TestExperiment(t *testing.T) {

	const code = `
func main (i int, j int) int {
	return 0
}
`

	compilationUnit := mustParse[CompilationUnit](t, code)

	assert.Len(t, compilationUnit.FuncDeclarations, 1)

	decl := compilationUnit.FuncDeclarations[0]
	assert.Equal(t, "main", decl.Name)
	assert.Equal(t, "int", decl.ReturnType)
	assert.Len(t, decl.Stmts, 1)

	stmt := decl.Stmts[0]
	assert.Equal(t, uint64(0), stmt.UInt)
}
