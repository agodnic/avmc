package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

func Test_UnaryOp(t *testing.T) {

	tcs := []TestCase{
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

	testAll(t, tcs)
}

func Test_BinOp(t *testing.T) {

	tcs := []TestCase{
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

	testAll(t, tcs)
}

func Test_IntLit(t *testing.T) {

	tcs := []TestCase{
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

	testAll(t, tcs)
}

func Test_BytesLit(t *testing.T) {

	tcs := []TestCase{
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

	testAll(t, tcs)
}

func Test_Ident(t *testing.T) {

	tcs := []TestCase{
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

	testAll(t, tcs)
}
