package mnemonic

//go:generate go run gen.go > mnemonics_gen.go

// All mnemonics implement the Mnemonic interface
type Mnemonic interface {
	mnemonicTag()
}
