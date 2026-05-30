package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

func Test_OperatorPrecedence(t *testing.T) {

	tcs := []TestCase{
		{
			Name:  "+ has lower precedence than *",
			Input: `1 + 2 * 3;`,
			Output: cst.BinOp{
				R:  cst.UintLit{Value: 1},
				Op: "+",
				L: cst.BinOp{
					R:  cst.UintLit{Value: 2},
					Op: "*",
					L:  cst.UintLit{Value: 3},
				},
			},
		},
		{
			Name:  "* has higher precedence than +",
			Input: `1 * 2 + 3;`,
			Output: cst.BinOp{
				R: cst.BinOp{
					R:  cst.UintLit{Value: 1},
					Op: "*",
					L:  cst.UintLit{Value: 2},
				},
				Op: "+",
				L:  cst.UintLit{Value: 3},
			},
		},
		{
			Name:  "parenthesized expressions have higher precedence than *",
			Input: `1 * (2 + 3);`,
			Output: cst.BinOp{
				R:  cst.UintLit{Value: 1},
				Op: "*",
				L: cst.BinOp{
					R:  cst.UintLit{Value: 2},
					Op: "+",
					L:  cst.UintLit{Value: 3},
				},
			},
		},
		{
			Name:  "unary - has higher precedence than +",
			Input: `-1 + 2;`,
			Output: cst.BinOp{
				R: cst.UnaryOp{
					Op:   "-",
					Expr: cst.UintLit{Value: 1},
				},
				Op: "+",
				L:  cst.UintLit{Value: 2},
			},
		},
	}

	testAll(t, tcs)
}
