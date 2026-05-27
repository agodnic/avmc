package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
	"github.com/agodnic/avmc/parser/generated/lexer"
	"github.com/agodnic/avmc/parser/generated/parser"
	"github.com/stretchr/testify/assert"
)

type TestCase struct {
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
		tree, err := parse([]byte(tc.Input))
		assert.NoError(t, err)
		assert.Equal(t, tc.Output, tree)
	}
}

func Test_Assignment(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `a = 1;`,
			Output: cst.Assignment{
				Ident: "a",
				Expr:  cst.IntLit{Value: "1"},
			},
		},
	}

	testAll(t, tcs)
}

func Test_FuncDecl(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `func f() string { a; return ; } ;`,
			Output: cst.FuncDecl{
				Ident:  "f",
				Params: []cst.Param{},
				Type:   cst.Type{TypeEnum: cst.TypeEnum_String},
				Block: cst.Block{
					Stmts: []any{
						cst.Ident{Ident: "a"},
						cst.Return{},
					},
				},
			},
		},
		{
			Input: `func f() { a; return ; } ;`,
			Output: cst.FuncDecl{
				Ident:  "f",
				Params: []cst.Param{},
				Type:   cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{
						cst.Ident{Ident: "a"},
						cst.Return{},
					},
				},
			},
		},
		{
			Input: `func f(i uint64) uint64 { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Input: `func f(i uint64) { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Input: `func f(s string, i uint64) uint64 { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "s", Type: cst.Type{TypeEnum: cst.TypeEnum_String}},
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Input: `func f(i uint64, j uint64, k uint64) uint64 { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
					{Ident: "j", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
					{Ident: "k", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_IfStmt(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `if 1 { return; }`,
			Output: cst.If{
				Cond: cst.IntLit{Value: "1"},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Input: `if 1 { return; } else { return; }`,
			Output: cst.If{
				Cond: cst.IntLit{Value: "1"},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
				Else: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Input: `if 1 { return; } else if 2 { return; }`,
			Output: cst.If{
				Cond: cst.IntLit{Value: "1"},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
				Else: cst.If{
					Cond: cst.IntLit{Value: "2"},
					Then: cst.Block{
						Stmts: []any{cst.Return{}},
					},
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_Block(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `{ return; }`,
			Output: cst.Block{
				Stmts: []any{
					cst.Return{},
				},
			},
		},
		{
			Input: `{ 1; return; }`,
			Output: cst.Block{
				Stmts: []any{
					cst.IntLit{Value: "1"},
					cst.Return{},
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_Return(t *testing.T) {

	tcs := []TestCase{
		{
			Input:  `return;`,
			Output: cst.Return{},
		},
		{
			Input: `return 1;`,
			Output: cst.Return{
				Expr: cst.IntLit{Value: "1"},
			},
		},
	}

	testAll(t, tcs)
}

func Test_ConstDecl(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `const a uint64 = 1;`,
			Output: cst.ConstDecl{
				Ident: "a",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Expr:  cst.IntLit{Value: "1"},
			},
		},
	}

	testAll(t, tcs)
}

func Test_VarDecl(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `var a uint64 = 1;`,
			Output: cst.VarDecl{
				Ident: "a",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Expr:  cst.IntLit{Value: "1"},
			},
		},
		{
			Input: `var b string = strcat("a", "b");`,
			Output: cst.VarDecl{
				Ident: "b",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_String},
				Expr: cst.Call{
					Ident: cst.Ident{
						Ident: "strcat",
					},
					Args: []any{
						cst.StrLit{Value: "a"},
						cst.StrLit{Value: "b"},
					},
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_Call(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `f();`,
			Output: cst.Call{
				Ident: cst.Ident{
					Ident: "f",
				},
				Args: []any{},
			},
		},
		{
			Input: `f(1);`,
			Output: cst.Call{
				Ident: cst.Ident{
					Ident: "f",
				},
				Args: []any{
					cst.IntLit{
						Value: "1",
					},
				},
			},
		},
		{
			Input: `f(1, 2);`,
			Output: cst.Call{
				Ident: cst.Ident{
					Ident: "f",
				},
				Args: []any{
					cst.IntLit{
						Value: "1",
					},
					cst.IntLit{
						Value: "2",
					},
				},
			},
		},
		{
			Input: `f(1, 2, 3);`,
			Output: cst.Call{
				Ident: cst.Ident{
					Ident: "f",
				},
				Args: []any{
					cst.IntLit{
						Value: "1",
					},
					cst.IntLit{
						Value: "2",
					},
					cst.IntLit{
						Value: "3",
					},
				},
			},
		},
		{
			Input: `pkg.f();`,
			Output: cst.Call{
				Ident: cst.Ident{
					PackageName: "pkg",
					Ident:       "f",
				},
				Args: []any{},
			},
		},
	}

	testAll(t, tcs)
}

func Test_Ident(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `myvariable;`,
			Output: cst.Ident{
				Ident: "myvariable",
			},
		},
		{
			Input: `myVariable;`,
			Output: cst.Ident{
				Ident: "myVariable",
			},
		},
		{
			Input: `MyVariable;`,
			Output: cst.Ident{
				Ident: "MyVariable",
			},
		},
		{
			Input: `a1;`,
			Output: cst.Ident{
				Ident: "a1",
			},
		},
		{
			Input: `mypackage.myvariable;`,
			Output: cst.Ident{
				PackageName: "mypackage",
				Ident:       "myvariable",
			},
		},
		{
			Input: `!pkg.myvar;`,
			Output: cst.UnaryOp{
				Op: "!",
				Expr: cst.Ident{
					PackageName: "pkg",
					Ident:       "myvar",
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_OperatorPrecedence(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `1 + 2 * 3;`,
			Output: cst.BinOp{
				R:  cst.IntLit{Value: "1"},
				Op: "+",
				L: cst.BinOp{
					R:  cst.IntLit{Value: "2"},
					Op: "*",
					L:  cst.IntLit{Value: "3"},
				},
			},
		},
		{
			Input: `1 * 2 + 3;`,
			Output: cst.BinOp{
				R: cst.BinOp{
					R:  cst.IntLit{Value: "1"},
					Op: "*",
					L:  cst.IntLit{Value: "2"},
				},
				Op: "+",
				L:  cst.IntLit{Value: "3"},
			},
		},
		{
			Input: `1 * (2 + 3);`,
			Output: cst.BinOp{
				R:  cst.IntLit{Value: "1"},
				Op: "*",
				L: cst.BinOp{
					R:  cst.IntLit{Value: "2"},
					Op: "+",
					L:  cst.IntLit{Value: "3"},
				},
			},
		},
		{
			Input: `-1 + 2;`,
			Output: cst.BinOp{
				R: cst.UnaryOp{
					Op:   "-",
					Expr: cst.IntLit{Value: "1"},
				},
				Op: "+",
				L:  cst.IntLit{Value: "2"},
			},
		},
	}

	testAll(t, tcs)
}

func Test_BinOp(t *testing.T) {

	tcs := []TestCase{
		{
			Input: `1 + 2;`,
			Output: cst.BinOp{
				R:  cst.IntLit{Value: "1"},
				Op: "+",
				L:  cst.IntLit{Value: "2"},
			},
		},
	}

	testAll(t, tcs)
}

func Test_IntLit(t *testing.T) {

	tcs := []TestCase{
		{
			Input:  `1;`,
			Output: cst.IntLit{Value: "1"},
		},
		{
			Input:  `123;`,
			Output: cst.IntLit{Value: "123"},
		},
	}

	testAll(t, tcs)
}

func Test_StrLit(t *testing.T) {

	tcs := []TestCase{
		{
			Input:  `"";`,
			Output: cst.StrLit{Value: ""},
		},
		{
			Input:  `"abc";`,
			Output: cst.StrLit{Value: "abc"},
		},
	}

	testAll(t, tcs)
}
