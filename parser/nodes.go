package parser

/*
Expression:

	Equality        == !=
	Comparison      < > <= >=
	Additive        + -
	Multiplicative  * /
	Unary           ! -
	Postfix         function calls
	Primary         literals, variables, (...)
*/
type (
	Expr struct {
		Equality *Equality `@@`
	}

	Equality struct {
		Left  *Additive  `@@`
		Right []*EqOpRhs `@@*`
	}

	EqOpRhs struct {
		Op    string    `@( "==" | "!=" )`
		Right *Additive `@@`
	}

	Additive struct {
		Left  *Multiplicative `@@`
		Right []*AddOpRhs     `@@*`
	}

	AddOpRhs struct {
		Op    string          `@( "+" | "-" )`
		Right *Multiplicative `@@`
	}

	Multiplicative struct {
		Left  *Unary      `@@`
		Right []*MulOpRhs `@@*`
	}

	MulOpRhs struct {
		Op    string `@( "*" | "/" )`
		Right *Unary `@@`
	}

	Unary struct {
		Op      *string  `@( "!" | "-" )?`
		Operand *Postfix `@@`
	}

	Postfix struct {
		Primary *Primary `@@`
		Calls   []*Call  `@@*`
	}

	Call struct {
		Args []*Expr `"(" ( @@ ( "," @@ )* )? ")"`
	}

	Primary struct {
		Number  *int    `  @Int`
		String  *string `| @String`
		Ident   *string `| @Ident`
		Subexpr *Expr   `| "(" @@ ")"`
	}
)

/*
Statements:

	VarDecl
	FuncDecl
	IfStmt
	WhileStmt
	ReturnStmt
	Assignment
	ExprStmt
	BlockStmt
*/
type (
	Statement struct {
		VarDecl   *VarDecl    `  @@`
		FuncDecl  *FuncDecl   `| @@`
		WhileStmt *WhileStmt  `| @@`
		IfStmt    *IfStmt     `| @@`
		Return    *ReturnStmt `| @@`
		Assign    *Assignment `| @@`
		Expr      *ExprStmt   `| @@`
		Block     *BlockStmt  `| @@`
	}

	VarDecl struct {
		Name  string ` "let" @Ident`
		Value *Expr  `"=" @@`
	}

	Assignment struct {
		Name  string `@Ident`
		Value *Expr  `"=" @@`
	}

	FuncDecl struct {
		Name   string     `"fn" @Ident`
		Params []string   `"(" ( @Ident ( "," @Ident )* )? ")"`
		Body   *BlockStmt `@@`
	}

	WhileStmt struct {
		Condition *Expr      `"while" @@`
		Body      *BlockStmt `@@`
	}

	IfStmt struct {
		Condition *Expr      `"if" @@`
		Then      *BlockStmt `@@`
		Else      *BlockStmt `( "else" @@ )?`
	}

	ReturnStmt struct {
		Value *Expr `"return" @@?`
	}

	ExprStmt struct {
		Expr *Expr `@@`
	}

	BlockStmt struct {
		Statements []*Statement `"{" @@* "}"`
	}
)

type CompilationUnit struct {
	FuncDecls []FuncDecl `@@+`
}
