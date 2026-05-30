package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

func Test_FuncDecl(t *testing.T) {

	tcs := []TestForTopLevelStmt{
		{
			Name:  "bytes return value",
			Input: `func f() bytes { return; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.BytesType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
						},
					},
				},
			},
		},
		{
			Name:  "int return value",
			Input: `func f() uint64 { return; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.Uint64Type{},
					Block: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
		{
			Name:  "no parameters",
			Input: `func f() { return ; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.VoidType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
						},
					},
				},
			},
		},
		{
			Name:  "one parameter",
			Input: `func f(i uint64) { return ; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Params: []cst.Param{
						{Ident: "i", Type: cst.Uint64Type{}},
					},
					Type: cst.VoidType{},
					Block: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
		{
			Name:  "two parameters",
			Input: `func f(b bytes, i uint64) { return ; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Params: []cst.Param{
						{Ident: "b", Type: cst.BytesType{}},
						{Ident: "i", Type: cst.Uint64Type{}},
					},
					Type: cst.VoidType{},
					Block: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
		{
			Name:  "three parameters",
			Input: `func f(i uint64, j uint64, k uint64) { return ; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Params: []cst.Param{
						{Ident: "i", Type: cst.Uint64Type{}},
						{Ident: "j", Type: cst.Uint64Type{}},
						{Ident: "k", Type: cst.Uint64Type{}},
					},
					Type: cst.VoidType{},
					Block: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
		{
			Name:  "two statements in block",
			Input: `func f() { return; return ; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.VoidType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
							cst.Return{},
						},
					},
				},
			},
		},
	}

	testTopLevelStmts(t, tcs)
}

func Test_TopLevelStatements(t *testing.T) {

	tcs := []TestForTopLevelStmt{
		{
			Name:  "declare two functions",
			Input: `func f() bytes { return; } func g() bytes { return; }`,
			Output: []any{
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.BytesType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
						},
					},
				},
				cst.FuncDecl{
					Ident: "g",
					Type:  cst.BytesType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
						},
					},
				},
			},
		},
		{
			Name:  "declare two constants",
			Input: `const c uint64 = 1 const d uint64 = 2`,
			Output: []any{
				cst.ConstDecl{
					Ident: "c",
					Type:  cst.Uint64Type{},
					Expr: cst.UintLit{
						Value: 1,
					},
				},
				cst.ConstDecl{
					Ident: "d",
					Type:  cst.Uint64Type{},
					Expr: cst.UintLit{
						Value: 2,
					},
				},
			},
		},
		{
			Name:  "declare a function and a constant",
			Input: `const c uint64 = 1 func f() bytes { return; }`,
			Output: []any{
				cst.ConstDecl{
					Ident: "c",
					Type:  cst.Uint64Type{},
					Expr: cst.UintLit{
						Value: 1,
					},
				},
				cst.FuncDecl{
					Ident: "f",
					Type:  cst.BytesType{},
					Block: cst.Block{
						Stmts: []any{
							cst.Return{},
						},
					},
				},
			},
		},
	}

	testTopLevelStmts(t, tcs)
}
