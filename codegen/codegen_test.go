package codegen

import (
	"slices"
	"testing"

	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func TestGenerateFn(t *testing.T) {

	type TestCase struct {
		Input  ast.Func
		Output []teal.Instruction
	}

	tcs := []TestCase{
		/*
			func main():
				return 42
		*/
		{
			Input: ast.Func{
				Identifier: "main",
				Body: []ast.Stmt{
					ast.Return{
						Expr: ast.Int{V0: 42},
					},
				},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 42},
				teal.Return{},
			},
		},
	}

	for _, tc := range tcs {
		output := generateFn(tc.Input)

		if !slices.Equal(output, tc.Output) {
			t.Errorf("expected %+v, got %+v", tc.Output, output)
		}
	}
}

func TestGenerateExpr(t *testing.T) {

	type TestCase struct {
		Input  ast.Expr
		Output []teal.Instruction
	}

	tcs := []TestCase{
		/*
			1 + 2
		*/
		{
			Input: ast.Add{
				L: ast.Int{V0: 1},
				R: ast.Int{V0: 2},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Add{},
			},
		},
	}

	for _, tc := range tcs {
		output := generateExpr(tc.Input)

		if !slices.Equal(output, tc.Output) {
			t.Errorf("expected %+v, got %+v", tc.Output, output)
		}
	}
}
