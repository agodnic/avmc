package codegen

import (
	"reflect"
	"testing"

	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func assertMnemonicsEqual(t *testing.T, actual []teal.Mnemonic, expected []teal.Mnemonic) {
	if !reflect.DeepEqual(actual, expected) {
		t.Errorf("expected %+v, got %+v", expected, actual)
	}
}

func TestGenerateFn(t *testing.T) {

	type TestCase struct {
		Input  ast.FuncDecl
		Output []teal.Mnemonic
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
			Output: []teal.Mnemonic{
				teal.Int{V0: 42},
				teal.Return{},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateFn(tc.Input), tc.Output)
	}
}

func TestGenerateExpr(t *testing.T) {

	type TestCase struct {
		Input  ast.Expr
		Output []teal.Mnemonic
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
			Output: []teal.Mnemonic{
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
			Output: []teal.Mnemonic{
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
			Output: []teal.Mnemonic{
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
			Output: []teal.Mnemonic{
				teal.Int{V0: 4},
				teal.Int{V0: 2},
				teal.Div{},
			},
		},
		/*
			!true
		*/
		{
			Input: ast.UnaryExpr{
				Op:   teal.LogicalNot{},
				Expr: ast.IntLit{V0: 1},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.LogicalNot{},
			},
		},
		/*
			==
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Eq{},
				L:  ast.BytesLit{V0: []byte{1, 1}},
				R:  ast.BytesLit{V0: []byte{2, 2}},
			},
			Output: []teal.Mnemonic{
				teal.Byte{V0: []byte{1, 1}},
				teal.Byte{V0: []byte{2, 2}},
				teal.Eq{},
			},
		},
		/*
			!=
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Ne{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Ne{},
			},
		},
		/*
			>
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Gt{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Gt{},
			},
		},
		/*
			>=
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Gte{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Gte{},
			},
		},
		/*
			<
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Lt{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Lt{},
			},
		},
		/*
			<=
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.Lte{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.Lte{},
			},
		},
		/*
			&&
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.LogicalAnd{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.LogicalAnd{},
			},
		},
		/*
			||
		*/
		{
			Input: ast.BinaryExpr{
				Op: teal.LogicalOr{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []teal.Mnemonic{
				teal.Int{V0: 1},
				teal.Int{V0: 2},
				teal.LogicalOr{},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateExpr(tc.Input), tc.Output)
	}
}
