package parser

import (
	"testing"

	"github.com/alecthomas/participle/v2"
	"github.com/stretchr/testify/assert"
)

const code = `
func main () {
	return 0
}
`

type CompulationUnit struct {
	// FIXME use an union to have different types of declarations
	FuncDeclarations []*FuncDeclaration `@@*`
}

type FuncDeclaration struct {
	Func   string `"func"`
	Name   string `@Ident`
	LParen string `"("`
	RParen string `")"`
	LBrace string `"{"`

	// TODO use an union to have different types of statements
	Stmts []*Stmt `@@*`

	RBrace string `"}"`
}

type Stmt struct {
	Return string `"return"`
	UInt   uint64 `@Int`
}

func TestExperiment(t *testing.T) {

	parser, err := participle.Build[CompulationUnit](
		participle.Unquote("String"),
		//participle.Union[Value](String{}, Number{}),
	)
	assert.NoError(t, err)

	compilationUnit, err := parser.ParseString("", code)
	assert.NoError(t, err)

	assert.Len(t, compilationUnit.FuncDeclarations, 1)

	decl := compilationUnit.FuncDeclarations[0]
	assert.Equal(t, "main", decl.Name)
	assert.Len(t, decl.Stmts, 1)

	stmt := decl.Stmts[0]
	assert.Equal(t, uint64(0), stmt.UInt)
}
