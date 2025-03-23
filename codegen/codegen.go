package codegen

import (
	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func Generate(program *ast.Program) teal.Program {
	mnemonics := generateFuncDecl(program.MainFunction)
	return teal.Program{
		Mnemonics: mnemonics,
	}
}

func generateFuncDecl(fn ast.FuncDecl) []teal.Mnemonic {

	var mnemonics []teal.Mnemonic

	for _, stmt := range fn.Body {
		mnemonics = append(mnemonics, generateStmt(stmt)...)
	}

	return mnemonics
}

func generateStmt(stmt ast.Stmt) (mnemonics []teal.Mnemonic) {

	switch i := stmt.(type) {
	case ast.Return:
		mnemonics = append(mnemonics, generateExpr(i.Expr)...)
		mnemonics = append(mnemonics, teal.Return{})
	case ast.If:
		elseLabel := i.BaseLabelsName + "_else"
		endLabel := i.BaseLabelsName + "_end"

		// test block
		mnemonics = append(mnemonics, teal.Int{V0: 1})
		mnemonics = append(mnemonics, teal.Bnz{Label: elseLabel})

		// then block
		for j := range i.Then {
			mnemonics = append(mnemonics, generateStmt(i.Then[j])...)
		}
		mnemonics = append(mnemonics, teal.B{Label: endLabel})

		// else block
		mnemonics = append(mnemonics, teal.Label{Name: elseLabel})
		for j := range i.Else {
			mnemonics = append(mnemonics, generateStmt(i.Else[j])...)
		}

		// end block
		mnemonics = append(mnemonics, teal.Label{Name: endLabel})
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
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
	case ast.FunctionCall:

		// Mnemonics with embedded arguments
		if i.FuncName == "arg" {
			if len(i.Args) != 1 {
				//TODO msg := fmt(...)
				panic("invalid number of arguments for arg")
			}
			n, ok := i.Args[0].(ast.IntLit)
			if !ok {
				//TODO msg := fmt(...)
				panic("invalid argument type for arg")
			}

			mnemonics = append(mnemonics, teal.Arg{N: n.V0})

			return mnemonics
		}

		// Mnemonics without embedded arguments
		opcode, ok := builtinFunctionToMnemonic[i.FuncName]
		if !ok {
			//TODO msg := fmt(...)
			panic("unknown function")
		}

		for j := range i.Args {
			mnemonics = append(mnemonics, generateExpr(i.Args[j])...)
		}

		mnemonics = append(mnemonics, opcode)
		return mnemonics
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}

}

var builtinFunctionToMnemonic = map[string]teal.Mnemonic{
	"len":    teal.Len{},
	"sha256": teal.Sha256{},
}
