package ast

import "github.com/agodnic/avmc/ir/teal"

// https://go.dev/src/go/ast/ast.go?s=1405:1446#L29

// -----------------------------------------------------------------------------
// Interfaces that represent non-terminal nodes in the AST

// All statement nodes implement the Stmt interface
type Stmt interface {
	stmtTag()
}

// All expression nodes implement the Expr interface
type Expr interface {
	exprTag()
}

// -----------------------------------------------------------------------------
// Structs that represent non-terminal nodes in the AST

type Program struct {
	MainFunction FuncDecl
}

type FuncDecl struct {
	Identifier string
	Body       []Stmt
}

type Return struct {
	Expr Expr
}

type IntLit struct {
	V0 uint64
}

type BinaryExpr struct {
	Op teal.Instruction
	L  Expr
	R  Expr
}

// -----------------------------------------------------------------------------
// Interface tags

// Stmt interface tags
func (s Return) stmtTag() {}

// Expr interface tags
func (e BinaryExpr) exprTag() {}
func (e IntLit) exprTag()     {}
