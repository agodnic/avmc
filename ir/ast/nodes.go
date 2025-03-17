package ast

type Program struct {
	MainFunction Fn
}

type Fn struct {
	Identifier string
	Body       []any // Stmt | If | Expr
}

type Return struct {
	V uint64
}
