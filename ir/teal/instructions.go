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

	// int <i>
	Int struct {
		V0 uint64
	}

	// !
	LogicalNot struct{}

	// *
	Mul struct{}

	// return
	Return struct{}

	// -
	Sub struct{}
)

// Mnemonic interface tags
func (i Add) mnemonicTag()        {}
func (i Div) mnemonicTag()        {}
func (i Int) mnemonicTag()        {}
func (i LogicalNot) mnemonicTag() {}
func (i Mul) mnemonicTag()        {}
func (i Return) mnemonicTag()     {}
func (i Sub) mnemonicTag()        {}
