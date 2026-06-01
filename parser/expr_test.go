package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

func Test_Call(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "function call with no args",
			Input: `f();`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					Ident: "f",
				},
			},
		},
		{
			Name:  "function call with one arg",
			Input: `f(1);`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					Ident: "f",
				},
				Args: []any{
					cst.UintLit{
						Value: 1,
					},
				},
			},
		},
		{
			Name:  "function call with two args",
			Input: `f(1, 2);`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					Ident: "f",
				},
				Args: []any{
					cst.UintLit{
						Value: 1,
					},
					cst.UintLit{
						Value: 2,
					},
				},
			},
		},
		{
			Name:  "function call with three args",
			Input: `f(1, 2, 3);`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					Ident: "f",
				},
				Args: []any{
					cst.UintLit{
						Value: 1,
					},
					cst.UintLit{
						Value: 2,
					},
					cst.UintLit{
						Value: 3,
					},
				},
			},
		},
		{
			Name:  "function call with package name",
			Input: `pkg.f();`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					PackageName: "pkg",
					Ident:       "f",
				},
			},
		},
		{
			Name:  "function call with a trailing comma in args",
			Input: `f(1,);`,
			Output: cst.Call{
				QualifiedIdent: cst.QualifiedIdent{
					Ident: "f",
				},
				Args: []any{
					cst.UintLit{Value: 1},
				},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_UnaryOp(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "unary op over an integer literal",
			Input: `-1;`,
			Output: cst.UnaryOp{
				Op:   "-",
				Expr: cst.UintLit{Value: 1},
			},
		},
		{
			Name:  "unary op over a variable",
			Input: `!a;`,
			Output: cst.UnaryOp{
				Op:   "!",
				Expr: cst.QualifiedIdent{Ident: "a"},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_BinOp(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "binary add between two integer literals",
			Input: `1 + 2;`,
			Output: cst.BinOp{
				R:  cst.UintLit{Value: 1},
				Op: "+",
				L:  cst.UintLit{Value: 2},
			},
		},
	}

	testStmts(t, tcs)
}
func Test_ParenthesizedExpr(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "parenthesized uint literal",
			Input: `(1);`,
			Output: cst.ParenthesizedExpr{
				Expr: cst.UintLit{Value: 1},
			},
		},
		{
			Name:  "parenthesized operand in binary operation",
			Input: `1 + (2);`,
			Output: cst.BinOp{
				R:  cst.UintLit{Value: 1},
				Op: "+",
				L: cst.ParenthesizedExpr{
					Expr: cst.UintLit{Value: 2},
				},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_IndexExpr(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "identifier indexed with uint literal",
			Input: `a[1];`,
			Output: cst.IndexExpr{
				BaseExpr:  cst.QualifiedIdent{Ident: "a"},
				IndexExpr: cst.UintLit{Value: 1},
			},
		},
		{
			Name:  "identifier indexed with identifier",
			Input: `a[i];`,
			Output: cst.IndexExpr{
				BaseExpr:  cst.QualifiedIdent{Ident: "a"},
				IndexExpr: cst.QualifiedIdent{Ident: "i"},
			},
		},
		{
			Name:  "function call indexed with uint literal",
			Input: `f()[1];`,
			Output: cst.IndexExpr{
				BaseExpr: cst.Call{
					QualifiedIdent: cst.QualifiedIdent{Ident: "f"},
				},
				IndexExpr: cst.UintLit{Value: 1},
			},
		},
	}

	testStmts(t, tcs)
}

func Test_IntLit(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:   "single digit number",
			Input:  `1;`,
			Output: cst.UintLit{Value: 1},
		},
		{
			Name:   "multiple digit number",
			Input:  `123;`,
			Output: cst.UintLit{Value: 123},
		},
	}

	testStmts(t, tcs)
}

func Test_BoolLit(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:   "true",
			Input:  `true;`,
			Output: cst.BoolLit{Value: true},
		},
		{
			Name:   "false",
			Input:  `false;`,
			Output: cst.BoolLit{Value: false},
		},
	}

	testStmts(t, tcs)
}

func Test_BytesLit(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:   "empty bytes",
			Input:  `hex"";`,
			Output: cst.BytesLit{},
		},
		{
			Name:   "one byte",
			Input:  `hex"1a";`,
			Output: cst.BytesLit{Value: []byte{0x1a}},
		},
		{
			Name:   "two bytes",
			Input:  `hex"12ab";`,
			Output: cst.BytesLit{Value: []byte{0x12, 0xab}},
		},
	}

	testStmts(t, tcs)
}

func Test_Ident(t *testing.T) {

	tcs := []TestForStmt{
		{
			Name:  "all lowercase",
			Input: `myvariable;`,
			Output: cst.QualifiedIdent{
				Ident: "myvariable",
			},
		},
		{
			Name:  "start with lowercase, then mixed case",
			Input: `myVariable;`,
			Output: cst.QualifiedIdent{
				Ident: "myVariable",
			},
		},
		{
			Name:  "start with uppercase",
			Input: `MyVariable;`,
			Output: cst.QualifiedIdent{
				Ident: "MyVariable",
			},
		},
		{
			Name:  "alphanumeric",
			Input: `a1;`,
			Output: cst.QualifiedIdent{
				Ident: "a1",
			},
		},
		{
			Name:  "variable with package name",
			Input: `mypackage.myvariable;`,
			Output: cst.QualifiedIdent{
				PackageName: "mypackage",
				Ident:       "myvariable",
			},
		},
		{
			Name:  `expression over variable with package name`,
			Input: `!pkg.myvar;`,
			Output: cst.UnaryOp{
				Op: "!",
				Expr: cst.QualifiedIdent{
					PackageName: "pkg",
					Ident:       "myvar",
				},
			},
		},
	}

	testStmts(t, tcs)
}
