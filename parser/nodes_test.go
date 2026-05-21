package parser

import (
	"testing"

	"github.com/alecthomas/participle/v2"
	"github.com/stretchr/testify/assert"
)

const code = `
func main
`

type CompulationUnit struct {
	FuncDeclarations []*FuncDeclaration `@@*`
}

type FuncDeclaration struct {
	Name string `"func" @Ident`
}

func TestExperiment(t *testing.T) {

	parser, err := participle.Build[CompulationUnit](
		participle.Unquote("String"),
		//participle.Union[Value](String{}, Number{}),
	)
	assert.NoError(t, err)

	compilationUnit, err := parser.ParseString("", code)
	assert.NoError(t, err)

	assert.Equal(t, 1, len(compilationUnit.FuncDeclarations))
	assert.Equal(t, "main", compilationUnit.FuncDeclarations[0].Name)
}
