package ast

// https://go.dev/src/go/ast/ast.go?s=1405:1446#L29

// -----------------------------------------------------------------------------
// Interfaces that represent non-terminal nodes in the AST

// All statement nodes implement the Stmt interface
type Stmt interface {
	stmtTag()
}

// All expression nodes implement the Stmt interface
type Expr interface {
	exprTag()
}

// -----------------------------------------------------------------------------
// Structs that represent non-terminal nodes in the AST

type Program struct {
	MainFunction Func
}

type Func struct {
	Identifier string
	Body       []Stmt
}

type Return struct {
	Expr Expr
}

type Int struct {
	V0 uint64
}

type Add struct {
	L Expr
	R Expr
}

type Sub struct {
	L Expr
	R Expr
}

// -----------------------------------------------------------------------------
// Interface tags

// Stmt interface tags
func (s Return) stmtTag() {}

// Expr interface tags
func (e Add) exprTag() {}
func (e Int) exprTag() {}
func (e Sub) exprTag() {}
