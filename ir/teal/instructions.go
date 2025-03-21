package teal

// Program is a high-level representation of a TEAL program
type Program struct {
	Mnemonics []Mnemonic
}

// All mnemonics implement the Mnemonic interface
type Mnemonic interface {
	mnemonicTag()
}

// Mnemonics
type (
	// +
	Add struct{}

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

	// -
	Sub struct{}
)

// Mnemonic interface tags
func (i Add) mnemonicTag()        {}
func (i Div) mnemonicTag()        {}
func (i Eq) mnemonicTag()         {}
func (i Gt) mnemonicTag()         {}
func (i Gte) mnemonicTag()        {}
func (i Int) mnemonicTag()        {}
func (i Lt) mnemonicTag()         {}
func (i Lte) mnemonicTag()        {}
func (i LogicalAnd) mnemonicTag() {}
func (i LogicalNot) mnemonicTag() {}
func (i LogicalOr) mnemonicTag()  {}
func (i Mul) mnemonicTag()        {}
func (i Ne) mnemonicTag()         {}
func (i Return) mnemonicTag()     {}
func (i Sub) mnemonicTag()        {}
