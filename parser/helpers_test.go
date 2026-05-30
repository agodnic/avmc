package parser

import (
	"fmt"
	"testing"

	"github.com/agodnic/avmc/parser/cst"
	"github.com/agodnic/avmc/parser/generated/lexer"
	"github.com/agodnic/avmc/parser/generated/parser"
	"github.com/stretchr/testify/assert"
)

type TestCase struct {
	Name   string
	Input  string
	Output any
}

func parse(sourceCode []byte) (any, error) {

	lex := lexer.NewLexer([]byte(sourceCode))
	p := parser.NewParser()

	return p.Parse(lex)
}

func testAll(t *testing.T, tcs []TestCase) {
	for _, tc := range tcs {
		t.Run(tc.Name, func(t *testing.T) {

			tree, err := parse([]byte(tc.Input))
			if !assert.NoError(t, err) {
				return
			}

			assert.Equal(t, tc.Output, tree)
		})
	}
}

///////////////////////////////

type TestCaseInsideFunctionBody struct {
	Name   string
	Input  string
	Output any
}

func testAllInsideFunctionBody(t *testing.T, tcs []TestCaseInsideFunctionBody) {
	for _, tc := range tcs {
		t.Run(tc.Name, func(t *testing.T) {

			const functionName = "functionNameThatShouldNowShadowAnything"
			src := fmt.Sprintf(`func %s () { %s };`, functionName, tc.Input)

			actualTree, err := parse([]byte(src))
			if !assert.NoError(t, err) {
				return
			}

			expectedTree := cst.FuncDecl{
				Ident: functionName,
				Type:  cst.VoidType{},
				Block: cst.Block{
					Stmts: []any{
						tc.Output,
					},
				},
			}

			assert.Equal(t, expectedTree, actualTree)
		})
	}
}
