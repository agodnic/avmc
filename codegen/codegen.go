package codegen

import (
	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func Generate(program *ast.Program) teal.Program {
	mnemonics := generateFn(program.MainFunction)
	return teal.Program{
		Mnemonics: mnemonics,
	}
}

func generateFn(fn ast.FuncDecl) []teal.Mnemonic {

	var mnemonics []teal.Mnemonic

	for _, stmt := range fn.Body {
		switch i := stmt.(type) {
		case (ast.Return):
			mnemonics = append(mnemonics, generateExpr(i.Expr)...)
			mnemonics = append(mnemonics, teal.Return{})
		default:
			//TODO msg := fmt(...)
			panic("not iplemented")
		}
	}

	return mnemonics
}

func generateExpr(expr ast.Expr) (mnemonics []teal.Mnemonic) {

	switch i := expr.(type) {
	case ast.IntLit:
		mnemonics = []teal.Mnemonic{
			teal.Int{V0: i.V0},
		}
		return mnemonics
	case ast.BytesLit:
		mnemonics = []teal.Mnemonic{
			teal.Byte{V0: i.V0},
		}
		return mnemonics
	case ast.BinaryExpr:
		mnemonics = append(mnemonics, generateExpr(i.L)...)
		mnemonics = append(mnemonics, generateExpr(i.R)...)
		mnemonics = append(mnemonics, i.Op)
		return mnemonics
	case ast.UnaryExpr:
		mnemonics = append(mnemonics, generateExpr(i.Expr)...)
		mnemonics = append(mnemonics, i.Op)
		return mnemonics
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}
}
