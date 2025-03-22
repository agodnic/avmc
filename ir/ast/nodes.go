package ast

import "github.com/agodnic/avmc/ir/teal"

// https://go.dev/src/go/ast/ast.go?s=1405:1446#L29

// -----------------------------------------------------------------------------
// Interfaces that represent non-terminal nodes in the AST
//
// These interfaces are used to emulate some sort of pattern-matching.

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

// Declaration nodes
type (
	FuncDecl struct {
		Identifier string
		Body       []Stmt
	}
)

// Expression nodes
type (
	// BinaryExpr represents a binary expression
	BinaryExpr struct {
		Op teal.Mnemonic
		L  Expr
		R  Expr
	}

	// BytesLit represents a literal byte array expression
	BytesLit struct {
		V0 []byte
	}

	// IntLit represents a literal integer expression
	IntLit struct {
		V0 uint64
	}

	// UnaryExpr represents a unary expression
	UnaryExpr struct {
		Op   teal.Mnemonic
		Expr Expr
	}
)

// Statement nodes
type (
	Return struct {
		Expr Expr
	}
)

// -----------------------------------------------------------------------------
// Interface tags

// Stmt interface tags
func (s Return) stmtTag() {}

// Expr interface tags
func (e BinaryExpr) exprTag() {}
func (e BytesLit) exprTag()   {}
func (e IntLit) exprTag()     {}
func (e UnaryExpr) exprTag()  {}
