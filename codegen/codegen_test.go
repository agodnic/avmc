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
				MainFunction: ast.Fn{
					Identifier: "main",
					Body: []any{
						ast.Return{V: 42},
					},
				},
			},
			Output: teal.Program{
				Instructions: []any{
					teal.Int{V0: 42},
					teal.Return{},
				},
			},
		},
	}

	for _, tc := range tcs {
		output := Generate(&tc.Input)
		if !slices.Equal(output.Instructions, tc.Output.Instructions) {
			t.Errorf("expected %v, got %v", tc.Output, output)
		}
	}

}
