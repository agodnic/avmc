package parser

import (
	"testing"

	"github.com/alecthomas/participle/v2"
	"github.com/stretchr/testify/assert"
)

const code = `
func main (i int, j int) int {
	return 0
}
`

type CompilationUnit struct {
	// FIXME use an union to have different types of declarations
	FuncDeclarations []FuncDeclaration `@@*`
}

type FuncDeclaration struct {
	Func               string              `"func"`
	Name               string              `@Ident`
	LParen             string              `"("`
	FunctionParameters []FunctionParameter `@@! ("," @@)*`
	RParen             string              `")"`
	ReturnType         string              `@Ident`
	LBrace             string              `"{"`
	Stmts              []*ReturnStmt       `@@+` // TODO use an union to have different types of statements
	RBrace             string              `"}"`
}

type FunctionParameter struct {
	Ident string `@Ident`
	Type  string `@Ident`
}

// TODO add different types of statements
type ReturnStmt struct {
	Return string `"return"`
	UInt   uint64 `@Int` //TODO this should be an expr node
}

// TestReturnStmt exercises the parsing of the ReturnStmt grammar element
func TestReturnStmt(t *testing.T) {

	type TestCase struct {
		Name       string
		SourceCode string
		Error      bool
		Expected   ReturnStmt
	}

	tcs := []TestCase{
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

	for _, tc := range tcs {
		t.Run(tc.Name, func(t *testing.T) {
			//FIXME deduplicate
			parser, err := participle.Build[ReturnStmt](
				participle.Unquote("String"),
			)
			assert.NoError(t, err)

			returnStmt, err := parser.ParseString("", tc.SourceCode)
			assert.NoError(t, err)

			assert.Equal(t, tc.Expected, *returnStmt)
		})
	}

}

// TODO write table-driven tests for each node
func TestExperiment(t *testing.T) {

	parser, err := participle.Build[CompilationUnit](
		participle.Unquote("String"),
		//participle.Union[Value](String{}, Number{}),
	)
	assert.NoError(t, err)

	compilationUnit, err := parser.ParseString("", code)
	assert.NoError(t, err)

	assert.Len(t, compilationUnit.FuncDeclarations, 1)

	decl := compilationUnit.FuncDeclarations[0]
	assert.Equal(t, "main", decl.Name)
	assert.Equal(t, "int", decl.ReturnType)
	assert.Len(t, decl.Stmts, 1)

	stmt := decl.Stmts[0]
	assert.Equal(t, uint64(0), stmt.UInt)

}
