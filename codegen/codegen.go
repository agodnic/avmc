package codegen

import (
	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/teal"
)

func Generate(program *ast.Program) teal.Program {
	instructions := generateFn(program.MainFunction)
	return teal.Program{
		Instructions: instructions,
	}
}

func generateFn(fn ast.Fn) []any {

	var instructions []any

	for _, stmt := range fn.Body {
		switch i := stmt.(type) {
		case ast.Return:
			instructions = append(instructions, teal.Int{V0: i.V})
			instructions = append(instructions, teal.Return{})
		default:
			//TODO msg := fmt(...)
			panic("not iplemented")
		}
	}

	return instructions
}
