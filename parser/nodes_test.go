package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
	"github.com/agodnic/avmc/parser/generated/lexer"
	"github.com/agodnic/avmc/parser/generated/parser"
	"github.com/stretchr/testify/assert"
)

type TestCase struct {
	Name   string
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
		t.Run(tc.Name, func(t *testing.T) {

			tree, err := parse([]byte(tc.Input))
			if !assert.NoError(t, err) {
				return
			}

			assert.Equal(t, tc.Output, tree)
		})
	}
}

func Test_Assignment(t *testing.T) {

	tcs := []TestCase{
		{
			Name:  "int literal expression",
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
			Name:  "string return value",
			Input: `func f() string { return; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_String},
				Block: cst.Block{
					Stmts: []any{
						cst.Return{},
					},
				},
			},
		},
		{
			Name:  "int return value",
			Input: `func f() uint64 { return; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "no parameters",
			Input: `func f() { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{
						cst.Return{},
					},
				},
			},
		},
		{
			Name:  "one parameter",
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
			Name:  "two parameters",
			Input: `func f(s string, i uint64) { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "s", Type: cst.Type{TypeEnum: cst.TypeEnum_String}},
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "three parameters",
			Input: `func f(i uint64, j uint64, k uint64) { return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Params: []cst.Param{
					{Ident: "i", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
					{Ident: "j", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
					{Ident: "k", Type: cst.Type{TypeEnum: cst.TypeEnum_Uint64}},
				},
				Type: cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "two statements in block",
			Input: `func f() { return; return ; } ;`,
			Output: cst.FuncDecl{
				Ident: "f",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Void},
				Block: cst.Block{
					Stmts: []any{
						cst.Return{},
						cst.Return{},
					},
				},
			},
		},
	}

	testAll(t, tcs)
}

func Test_IfStmt(t *testing.T) {

	tcs := []TestCase{
		{
			Name:  "if statement without else part",
			Input: `if 1 { return; }`,
			Output: cst.If{
				Cond: cst.IntLit{Value: "1"},
				Then: cst.Block{
					Stmts: []any{cst.Return{}},
				},
			},
		},
		{
			Name:  "if statement with else part",
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
			Name:  "if statement with else if part",
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
			Name:   "return without expression",
			Input:  `return;`,
			Output: cst.Return{},
		},
		{
			Name:  "return with expression",
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
			Name:  "declare int constant",
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
			Name:  "int literal expression",
			Input: `var a uint64 = 1;`,
			Output: cst.VarDecl{
				Ident: "a",
				Type:  cst.Type{TypeEnum: cst.TypeEnum_Uint64},
				Expr:  cst.IntLit{Value: "1"},
			},
		},
		{
			Name:  "function call expression",
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
			Name:  "function call with no args",
			Input: `f();`,
			Output: cst.Call{
				Ident: cst.Ident{
					Ident: "f",
				},
				Args: []any{},
			},
		},
		{
			Name:  "function call with one arg",
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
			Name:  "function call with two args",
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
			Name:  "function call with three args",
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
			Name:  "function call with package name",
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
			Name:  "all lowercase",
			Input: `myvariable;`,
			Output: cst.Ident{
				Ident: "myvariable",
			},
		},
		{
			Name:  "start with lowercase, then mixed case",
			Input: `myVariable;`,
			Output: cst.Ident{
				Ident: "myVariable",
			},
		},
		{
			Name:  "start with uppercase",
			Input: `MyVariable;`,
			Output: cst.Ident{
				Ident: "MyVariable",
			},
		},
		{
			Name:  "alphanumeric",
			Input: `a1;`,
			Output: cst.Ident{
				Ident: "a1",
			},
		},
		{
			Name:  "variable with package name",
			Input: `mypackage.myvariable;`,
			Output: cst.Ident{
				PackageName: "mypackage",
				Ident:       "myvariable",
			},
		},
		{
			Name:  `expression over variable with package name`,
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
			Name:  "+ has lower precedence than *",
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
			Name:  "* has higher precedence than +",
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
			Name:  "parenthesized expressions have higher precedence than *",
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
			Name:  "unary - has higher precedence than +",
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
			Name:  "binary add between two integer literals",
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
			Name:   "single digit number",
			Input:  `1;`,
			Output: cst.IntLit{Value: "1"},
		},
		{
			Name:   "multiple digit number",
			Input:  `123;`,
			Output: cst.IntLit{Value: "123"},
		},
	}

	testAll(t, tcs)
}

func Test_StrLit(t *testing.T) {

	tcs := []TestCase{
		{
			Name:   "Empty string",
			Input:  `"";`,
			Output: cst.StrLit{Value: ""},
		},
		{
			Name:   "lowercase string",
			Input:  `"abc";`,
			Output: cst.StrLit{Value: "abc"},
		},
	}

	testAll(t, tcs)
}
