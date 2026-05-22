package parser

type (
	CompilationUnit struct {
		// FIXME use an union to have different types of declarations
		FuncDeclarations []FuncDeclaration `@@*`
	}

	FuncDeclaration struct {
		Func               string      `"func"`
		Name               string      `@Ident`
		LParen             string      `"("`
		FunctionParameters []FuncParam `@@! ("," @@)*`
		RParen             string      `")"`
		ReturnType         string      `@Ident`
		LBrace             string      `"{"`
		Stmts              []Stmt      `@@+`
		RBrace             string      `"}"`
	}

	FuncParam struct {
		Ident string `@Ident`
		Type  string `@Ident`
	}

	ReturnStmt struct {
		Return string `"return"`
		UInt   uint64 `@Int` //TODO should be an expr node
	}

	VariableDeclarationStmt struct {
		Var   string `"var"`
		Ident string `@Ident`
		Eq    string `"="`
		Expr  Expr   `@@`
	}

	IntegerExpr struct {
		Value int64 `@Int`
	}
)

type (
	Expr interface{ exprTag() }
	Stmt interface{ stmtTag() }
)

func (expr IntegerExpr) exprTag() {}

func (stmt ReturnStmt) stmtTag()              {}
func (stmt VariableDeclarationStmt) stmtTag() {}
