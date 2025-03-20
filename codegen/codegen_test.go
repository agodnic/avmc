package codegen

import (
	"slices"
	"testing"

	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func assertInstructionsEqual(t *testing.T, actual []teal.Instruction, expected []teal.Instruction) {
	if !slices.Equal(actual, expected) {
		t.Errorf("expected %+v, got %+v", expected, actual)
	}
}

func TestGenerateFn(t *testing.T) {

	type TestCase struct {
		Input  ast.FuncDecl
		Output []teal.Instruction
	}

	tcs := []TestCase{
		/*
			func main():
				return 42
		*/
		{
			Input: ast.FuncDecl{
				Identifier: "main",
				Body: []ast.Stmt{
					ast.Return{
						Expr: ast.IntLit{V0: 42},
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
		assertInstructionsEqual(t, generateFn(tc.Input), tc.Output)
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
			Input: ast.BinaryExpr{
				Op: teal.Add{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Add{},
			},
		},
		/*
			2 - 1
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Sub{},
				L:  ast.IntLit{V0: 2},
				R:  ast.IntLit{V0: 1},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 2},
				teal.Int{V0: 1},
				teal.Sub{},
			},
		},
		/*
			2 * 3
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Mul{},
				L:  ast.IntLit{V0: 2},
				R:  ast.IntLit{V0: 3},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 2},
				teal.Int{V0: 3},
				teal.Mul{},
			},
		},
		/*
			4 / 2
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Div{},
				L:  ast.IntLit{V0: 4},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Instruction{
				teal.Int{V0: 4},
				teal.Int{V0: 2},
				teal.Div{},
			},
		},
	}

	for _, tc := range tcs {
		assertInstructionsEqual(t, generateExpr(tc.Input), tc.Output)
	}
}
