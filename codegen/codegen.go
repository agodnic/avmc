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

func generateFn(fn ast.Func) []teal.Instruction {

	var instructions []teal.Instruction

	for _, stmt := range fn.Body {
		switch i := stmt.(type) {
		case (ast.Return):
			instructions = append(instructions, generateExpr(i.Expr)...)
			instructions = append(instructions, teal.Return{})
		default:
			//TODO msg := fmt(...)
			panic("not iplemented")
		}
	}

	return instructions
}

func generateExpr(expr ast.Expr) (instructions []teal.Instruction) {

	switch i := expr.(type) {
	case ast.Int:
		instructions = []teal.Instruction{
			teal.Int{V0: i.V0},
		}
		return instructions
	case ast.Add:
		instructions = append(instructions, generateExpr(i.L)...)
		instructions = append(instructions, generateExpr(i.R)...)
		instructions = append(instructions, teal.Add{})
		return instructions
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}
}
