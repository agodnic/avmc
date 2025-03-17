package codegen

import (
	"slices"
	"testing"

	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func TestGenerate(t *testing.T) {

	type TestCase struct {
		Input  ast.Program
		Output teal.Program
	}

	tcs := []TestCase{
		/*
			func main():
				return 42
		*/
		{
			Input: ast.Program{
				MainFunction: ast.Func{
					Identifier: "main",
					Body: []ast.Stmt{
						ast.Return{
							Expr: ast.Int{V0: 42},
						},
					},
				},
			},
			Output: teal.Program{
				Instructions: []teal.Instruction{
					teal.Int{V0: 42},
					teal.Return{},
				},
			},
		},
		/*
			func main():
				return 1 + 2
		*/
		{
			Input: ast.Program{
				MainFunction: ast.Func{
					Identifier: "main",
					Body: []ast.Stmt{
						ast.Return{
							Expr: ast.Add{
								L: ast.Int{V0: 1},
								R: ast.Int{V0: 2},
							},
						},
					},
				},
			},
			Output: teal.Program{
				Instructions: []teal.Instruction{
					teal.Int{V0: 1},
					teal.Int{V0: 2},
					teal.Add{},
					teal.Return{},
				},
			},
		},
	}

	for _, tc := range tcs {
		output := Generate(&tc.Input)
		if !slices.Equal(output.Instructions, tc.Output.Instructions) {
			t.Errorf("expected %+v, got %+v", tc.Output, output)
		}
	}

}
