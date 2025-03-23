package codegen

import (
	"reflect"
	"testing"

	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/mnemonic"
)

func assertMnemonicsEqual(t *testing.T, actual []mnemonic.Mnemonic, expected []mnemonic.Mnemonic) {
	if !reflect.DeepEqual(actual, expected) {
		t.Errorf("expected %+v, got %+v", expected, actual)
	}
}

func TestGenerateFuncDecl(t *testing.T) {

	type TestCase struct {
		Input  ast.FuncDecl
		Output []mnemonic.Mnemonic
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
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 42},
				mnemonic.Return{},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateFuncDecl(tc.Input), tc.Output)
	}
}

func TestGenerateStmt(t *testing.T) {

	type TestCase struct {
		Input  ast.Stmt
		Output []mnemonic.Mnemonic
	}

	tcs := []TestCase{
		/*
			return 42
		*/
		{
			Input: ast.Return{
				Expr: ast.IntLit{V0: 42},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 42},
				mnemonic.Return{},
			},
		},
		/*
			if true:
				return 1
			else:
				return 0
		*/
		{
			Input: ast.If{
				BaseLabelsName: "L0",
				Cond:           ast.IntLit{V0: 1},
				Then: []ast.Stmt{
					ast.Return{
						Expr: ast.IntLit{V0: 1},
					},
				},
				Else: []ast.Stmt{
					ast.Return{
						Expr: ast.IntLit{V0: 0},
					},
				},
			},
			Output: []mnemonic.Mnemonic{
				// test block
				mnemonic.Int{V0: 1},
				mnemonic.Bnz{Label: "L0_else"},

				// then block
				mnemonic.Int{V0: 1},
				mnemonic.Return{},
				mnemonic.B{Label: "L0_end"},

				// else block
				mnemonic.Label{Name: "L0_else"},
				mnemonic.Int{V0: 0},
				mnemonic.Return{},

				// end block
				mnemonic.Label{Name: "L0_end"},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateStmt(tc.Input), tc.Output)
	}
}

func TestGenerateFunctionCall(t *testing.T) {

	type TestCase struct {
		Input  ast.FunctionCall
		Output []mnemonic.Mnemonic
	}

	tcs := []TestCase{
		/*
			len("\x01\x02\x03")
		*/
		{
			Input: ast.FunctionCall{
				FuncName: "len",
				Args: []ast.Expr{
					ast.BytesLit{V0: []byte{1, 2, 3}},
				},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Byte{V0: []byte{1, 2, 3}},
				mnemonic.Len{},
			},
		},
		/*
			sha256("\x00")
		*/
		{
			Input: ast.FunctionCall{
				FuncName: "sha256",
				Args: []ast.Expr{
					ast.BytesLit{V0: []byte{0}},
				},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Byte{V0: []byte{0}},
				mnemonic.Sha256{},
			},
		},
		/*
			arg(0)
		*/
		{
			Input: ast.FunctionCall{
				FuncName: "arg",
				Args: []ast.Expr{
					ast.IntLit{V0: 0},
				},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Arg{N: 0},
			},
		},
		/*
			txn.Sender()
		*/
		{
			Input: ast.FunctionCall{
				FuncName: "txn.Sender",
				Args:     []ast.Expr{},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Txn{Field: "Sender"},
			},
		},
		/*
			txn.CloseRemainderTo()
		*/
		{
			Input: ast.FunctionCall{
				FuncName: "txn.CloseRemainderTo",
				Args:     []ast.Expr{},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Txn{Field: "CloseRemainderTo"},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateExpr(tc.Input), tc.Output)
	}
}

func TestGenerateExpr(t *testing.T) {

	type TestCase struct {
		Input  ast.Expr
		Output []mnemonic.Mnemonic
	}

	tcs := []TestCase{
		/*
			1 + 2
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Add{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Add{},
			},
		},
		/*
			2 - 1
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Sub{},
				L:  ast.IntLit{V0: 2},
				R:  ast.IntLit{V0: 1},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 2},
				mnemonic.Int{V0: 1},
				mnemonic.Sub{},
			},
		},
		/*
			2 * 3
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Mul{},
				L:  ast.IntLit{V0: 2},
				R:  ast.IntLit{V0: 3},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 2},
				mnemonic.Int{V0: 3},
				mnemonic.Mul{},
			},
		},
		/*
			4 / 2
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Div{},
				L:  ast.IntLit{V0: 4},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 4},
				mnemonic.Int{V0: 2},
				mnemonic.Div{},
			},
		},
		/*
			!true
		*/
		{
			Input: ast.UnaryExpr{
				Op:   mnemonic.LogicalNot{},
				Expr: ast.IntLit{V0: 1},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.LogicalNot{},
			},
		},
		/*
			==
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Eq{},
				L:  ast.BytesLit{V0: []byte{1, 1}},
				R:  ast.BytesLit{V0: []byte{2, 2}},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Byte{V0: []byte{1, 1}},
				mnemonic.Byte{V0: []byte{2, 2}},
				mnemonic.Eq{},
			},
		},
		/*
			!=
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Ne{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Ne{},
			},
		},
		/*
			>
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Gt{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Gt{},
			},
		},
		/*
			>=
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Gte{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Gte{},
			},
		},
		/*
			<
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Lt{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Lt{},
			},
		},
		/*
			<=
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.Lte{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.Lte{},
			},
		},
		/*
			&&
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.LogicalAnd{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.LogicalAnd{},
			},
		},
		/*
			||
		*/
		{
			Input: ast.BinaryExpr{
				Op: mnemonic.LogicalOr{},
				L:  ast.IntLit{V0: 1},
				R:  ast.IntLit{V0: 2},
			},
			Output: []mnemonic.Mnemonic{
				mnemonic.Int{V0: 1},
				mnemonic.Int{V0: 2},
				mnemonic.LogicalOr{},
			},
		},
	}

	for _, tc := range tcs {
		assertMnemonicsEqual(t, generateExpr(tc.Input), tc.Output)
	}
}
