package ast

type Program struct {
	MainFunction Func
}

type Func struct {
	Identifier string
	Body       []Stmt
}

// All statement nodes implement the Stmt interface
type Stmt interface {
	stmtTag()
}

// All expression nodes implement the Stmt interface
type Expr interface {
	exprTag()
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

// Stmt interface tags
func (s Return) stmtTag() {}

// Expr interface tags
func (e Add) exprTag() {}
func (e Int) exprTag() {}
