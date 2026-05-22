package parser

type CompilationUnit struct {
	// FIXME use an union to have different types of declarations
	FuncDeclarations []FuncDeclaration `@@*`
}

type FuncDeclaration struct {
	Func               string              `"func"`
	Name               string              `@Ident`
	LParen             string              `"("`
	FunctionParameters []FunctionParameter `@@! ("," @@)*`
	RParen             string              `")"`
	ReturnType         string              `@Ident`
	LBrace             string              `"{"`
	Stmts              []Stmt              `@@+`
	RBrace             string              `"}"`
}

type FunctionParameter struct {
	Ident string `@Ident`
	Type  string `@Ident`
}

type ReturnStmt struct {
	Return string `"return"`
	UInt   uint64 `@Int` //TODO should be an expr node
}

type VariableDeclarationStmt struct {
	Var   string `"var"`
	Ident string `@Ident`
	Eq    string `"="`
	Expr  string `@Int` //FIXME should be an expr node
}

type Stmt interface{ stmtTag() }

func (rs ReturnStmt) stmtTag()              {}
func (rs VariableDeclarationStmt) stmtTag() {}
