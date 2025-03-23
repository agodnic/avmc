package mnemonic

// Mnemonics
type (
	// +
	Add struct{}

	// arg <n>
	Arg struct {
		N uint64
	}

	// b <label>
	B struct {
		Label string
	}

	// bnz <label>
	Bnz struct {
		Label string
	}

	// bz <label>
	Bz struct {
		Label string
	}

	// byte <b>
	Byte struct {
		V0 []byte
	}

	// /
	Div struct{}

	// ==
	Eq struct{}

	// >
	Gt struct{}

	// >=
	Gte struct{}

	// int <i>
	Int struct {
		V0 uint64
	}

	// <label_name>:
	Label struct {
		Name string
	}

	// len
	Len struct{}

	// <
	Lt struct{}

	// <=
	Lte struct{}

	// &&
	LogicalAnd struct{}

	// !
	LogicalNot struct{}

	// ||
	LogicalOr struct{}

	// *
	Mul struct{}

	// !=
	Ne struct{}

	// return
	Return struct{}

	// sha256
	Sha256 struct{}

	// -
	Sub struct{}

	// txn <field>
	Txn struct {
		Field string
	}
)

// All mnemonics implement the Mnemonic interface
type Mnemonic interface {
	mnemonicTag()
}

func (i Add) mnemonicTag()        {}
func (i Arg) mnemonicTag()        {}
func (i B) mnemonicTag()          {}
func (i Bnz) mnemonicTag()        {}
func (i Bz) mnemonicTag()         {}
func (i Byte) mnemonicTag()       {}
func (i Div) mnemonicTag()        {}
func (i Eq) mnemonicTag()         {}
func (i Gt) mnemonicTag()         {}
func (i Gte) mnemonicTag()        {}
func (i Int) mnemonicTag()        {}
func (i Label) mnemonicTag()      {}
func (i Len) mnemonicTag()        {}
func (i Lt) mnemonicTag()         {}
func (i Lte) mnemonicTag()        {}
func (i LogicalAnd) mnemonicTag() {}
func (i LogicalNot) mnemonicTag() {}
func (i LogicalOr) mnemonicTag()  {}
func (i Mul) mnemonicTag()        {}
func (i Ne) mnemonicTag()         {}
func (i Return) mnemonicTag()     {}
func (i Sha256) mnemonicTag()     {}
func (i Sub) mnemonicTag()        {}
func (i Txn) mnemonicTag()        {}
