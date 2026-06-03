package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

func Test_Assignment(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "assignment from integer literal expression",
			Input: `a = 1;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr:  cst.UintLit{Value: 1},
			},
		},
		{
			Name:  "assignment from bool literal expression",
			Input: `a = true;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr:  cst.BoolLit{Value: true},
			},
		},
		{
			Name:  "assignment from bytes literal expression",
			Input: `a = hex"12ab";`,
			Output: cst.Assignment{
				Ident: "a",
				Expr:  cst.BytesLit{Value: []uint8{0x12, 0xab}},
			},
		},
		{
			Name:  "assignment from identifier expression",
			Input: `a = b;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr:  cst.QualifiedIdent{Ident: "b"},
			},
		},
		{
			Name:  "assignment from function call expression",
			Input: `a = f();`,
			Output: cst.Assignment{
				Ident: "a",
				Expr: cst.Call{
					QualifiedIdent: cst.QualifiedIdent{Ident: "f"},
				},
			},
		},
		{
			Name:  "assignment from unary operator expression",
			Input: `a = -2;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr: cst.UnaryOp{
					Op:   "-",
					Expr: cst.UintLit{Value: 2},
				},
			},
		},
		{
			Name:  "assignment from binary operator expression",
			Input: `a = 1 + 2;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr: cst.BinOp{
					R:  cst.UintLit{Value: 1},
					Op: "+",
					L:  cst.UintLit{Value: 2},
				},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_IfStmt(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "if statement without else part",
			Input: `if 1 { return; }`,
			Output: cst.If{
				Cond: cst.UintLit{Value: 1},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "if statement with else part",
			Input: `if 1 { return; } else { return; }`,
			Output: cst.If{
				Cond: cst.UintLit{Value: 1},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
				Else: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "if statement with else if part",
			Input: `if 1 { return; } else if 2 { return; }`,
			Output: cst.If{
				Cond: cst.UintLit{Value: 1},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
				Else: cst.If{
					Cond: cst.UintLit{Value: 2},
					Then: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_Block(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "empty block",
			Input: `{ }`,
			Output: cst.Block{
				Stmts: []any{},
			},
		},
		{
			Name:  "block with one statement",
			Input: `{ return; }`,
			Output: cst.Block{
				Stmts: []any{
					cst.Return{},
				},
			},
		},
		{
			Name:  "block with two statements",
			Input: `{ 1; return; }`,
			Output: cst.Block{
				Stmts: []any{
					cst.UintLit{Value: 1},
					cst.Return{},
				},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_Return(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:   "return without expression",
			Input:  `return;`,
			Output: cst.Return{},
		},
		{
			Name:  "return with expression",
			Input: `return 1;`,
			Output: cst.Return{
				Expr: cst.UintLit{Value: 1},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_ConstDecl(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "declare int constant",
			Input: `const a uint64 = 1;`,
			Output: cst.ConstDecl{
				Ident: "a",
				Type:  cst.Uint64Type{},
				Expr:  cst.UintLit{Value: 1},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_VarDecl(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "int literal expression",
			Input: `var a uint64 = 1;`,
			Output: cst.VarDecl{
				Ident: "a",
				Type:  cst.Uint64Type{},
				Expr:  cst.UintLit{Value: 1},
			},
		},
		{
			Name:  "function call expression",
			Input: `var b []uint8 = f();`,
			Output: cst.VarDecl{
				Ident: "b",
				Type: cst.SliceType{
					Type: cst.Uint8Type{},
				},
				Expr: cst.Call{
					QualifiedIdent: cst.QualifiedIdent{
						Ident: "f",
					},
				},
			},
		},
	}

	testStmts(t, tcs)
}
