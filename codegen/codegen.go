package codegen

import (
	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/mnemonic"
)

func generateFuncDecl(fn ast.FuncDecl) []mnemonic.Mnemonic {

	var mnemonics []mnemonic.Mnemonic

	for _, stmt := range fn.Body {
		mnemonics = append(mnemonics, generateStmt(stmt)...)
	}

	return mnemonics
}

func generateStmt(stmt ast.Stmt) (mnemonics []mnemonic.Mnemonic) {

	switch i := stmt.(type) {
	case ast.Return:
		mnemonics = append(mnemonics, generateExpr(i.Expr)...)
		mnemonics = append(mnemonics, mnemonic.Return{})
	case ast.If:
		elseLabel := i.BaseLabelsName + "_else"
		endLabel := i.BaseLabelsName + "_end"

		// test block
		mnemonics = append(mnemonics, mnemonic.Int{I: 1})
		mnemonics = append(mnemonics, mnemonic.Bnz{Target: elseLabel})

		// then block
		for j := range i.Then {
			mnemonics = append(mnemonics, generateStmt(i.Then[j])...)
		}
		mnemonics = append(mnemonics, mnemonic.B{Target: endLabel})

		// else block
		mnemonics = append(mnemonics, mnemonic.Label{I: elseLabel})
		for j := range i.Else {
			mnemonics = append(mnemonics, generateStmt(i.Else[j])...)
		}

		// end block
		mnemonics = append(mnemonics, mnemonic.Label{I: endLabel})
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}

	return mnemonics
}

func generateExpr(expr ast.Expr) (mnemonics []mnemonic.Mnemonic) {

	switch i := expr.(type) {
	case ast.IntLit:
		mnemonics = []mnemonic.Mnemonic{
			mnemonic.Int{I: i.V0},
		}
		return mnemonics
	case ast.BytesLit:
		mnemonics = []mnemonic.Mnemonic{
			mnemonic.Byte{I: i.V0},
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

			// FIXME hard cast. Probably the input structure should have the right type
			mnemonics = append(mnemonics, mnemonic.Arg{N: uint8(n.V0)})

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

var builtinFunctionToMnemonic = map[string]mnemonic.Mnemonic{
	"len":    mnemonic.Len{},
	"sha256": mnemonic.Sha256{},
}
