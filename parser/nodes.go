package parser

type (
	//TODO UnaryExpression

	//TODO BinaryExpression

	//TODO Figure out how to represent parenthesized expressions without
	//
	// Could probably define ParenthesizedExpr as "(" @@ ")"
	// and add it to the enum

	//TODO IfStmt

	//TODO WhileLoop

	CompilationUnit struct {
		// FIXME use an union to have different types of declarations
		FuncDeclarations []FuncDeclaration `@@+`
	}

	FuncDeclaration struct {
		Func       string      `"func"`
		Name       string      `@Ident`
		LParen     string      `"("`
		FuncParams []FuncParam `@@! ("," @@)*`
		RParen     string      `")"`
		ReturnType string      `@Ident`
		LBrace     string      `"{"`
		Stmts      []Stmt      `@@+`
		RBrace     string      `"}"`
	}

	FuncParam struct {
		Ident string `@Ident`
		Type  string `@Ident`
	}

	ReturnStmt struct {
		Return string `"return"`
		Value  Expr   `@@`
	}

	VariableDeclarationStmt struct {
		Var   string `"var"`
		Ident string `@Ident`
		Type  string `@Ident`
		Eq    string `"="`
		Expr  Expr   `@@`
	}

	AssignmentStmt struct {
		Ident string `@Ident`
		Eq    string `"="`
		Value Expr   `@@`
	}

	CallExpr struct {
		Ident  string `@Ident`
		LParen string `"("`
		Params []Expr `@@! ("," @@)*`
		RParen string `")"`
	}

	IntegerExpr struct {
		Value int64 `@Int`
	}

	StringExpr struct {
		Value string `@String`
	}

	UnaryExpr struct {
		Operator string `"!"`
		Operand  Expr   `@@`
	}

	VarExpr struct {
		Ident string `@Ident`
	}
)

type (
	Expr interface{ exprTag() }
	Stmt interface{ stmtTag() }
)

func (expr CallExpr) exprTag()    {}
func (expr IntegerExpr) exprTag() {}
func (expr StringExpr) exprTag()  {}
func (expr UnaryExpr) exprTag()   {}
func (expr VarExpr) exprTag()     {}

func (stmt AssignmentStmt) stmtTag()          {}
func (stmt ReturnStmt) stmtTag()              {}
func (stmt VariableDeclarationStmt) stmtTag() {}
