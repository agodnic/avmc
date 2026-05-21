package codegen

import (
	"fmt"

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
		msg := fmt.Sprintf("encountered unexpected stmt type while generating code: %#v", stmt)
		panic(msg)
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
		//
		// FIXME obviously we should not code this manually for every function.
		// We should define, for each function, number and types of parameters.
		// Then use a function to perform the type checking.
		// This is would be easy to unit test.
		if i.FuncName == "arg" {
			if len(i.Args) != 1 {
				msg := fmt.Sprintf("expected exactly 1 arguments on `arg` function call, but got %d", len(i.Args))
				panic(msg)
			}
			n, ok := i.Args[0].(ast.IntLit)
			if !ok {
				msg := fmt.Sprintf("unexpected argument type for `arg` function call: %#v", i.Args[0])
				panic(msg)
			}

			// FIXME hard cast to int8.
			// Maybe the input structure should have the int8 type?
			// The problem is that the parser has no way to know that at parse-time.
			// Hence the argument can only be an integer literal without detailed type information,
			// and the type-checking should be done at this later stage.
			mnemonics = append(mnemonics, mnemonic.Arg{N: uint8(n.V0)})

			return mnemonics
		}

		// Mnemonics without embedded arguments
		opcode, ok := builtinFunctionToMnemonic[i.FuncName]
		if !ok {
			msg := fmt.Sprintf("unknown function name: %s", i.FuncName)
			panic(msg)
		}

		for j := range i.Args {
			mnemonics = append(mnemonics, generateExpr(i.Args[j])...)
		}

		mnemonics = append(mnemonics, opcode)
		return mnemonics
	default:
		msg := fmt.Sprintf("encountered unexpected expression type while generating code: %#v", expr)
		panic(msg)
	}

}

// Some builtin functions translate directly to TEAL opcodes
//
// e.g. the builtin function `len()` translates into the `len` opcode.
//
// This map defines a table to perform those translations
var builtinFunctionToMnemonic = map[string]mnemonic.Mnemonic{
	"len":    mnemonic.Len{},
	"sha256": mnemonic.Sha256{},
}
