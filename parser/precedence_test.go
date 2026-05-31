package parser

import (
	"testing"

	"github.com/agodnic/avmc/parser/cst"
)

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
