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
	UInt   uint64 `@Int` //TODO this should be an expr node
}

func (rs ReturnStmt) stmtTag() {}

// TODO add different types of statements
type Stmt interface{ stmtTag() }
