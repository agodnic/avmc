package teal

// Program is a high-level representation of a TEAL program
type Program struct {
	Instructions []Instruction
}

// All instructions implement the Instruction interface
type Instruction interface {
	instructionTag()
}

// Instructions
type Add struct{}
type Int struct {
	V0 uint64
}
type Return struct{}

// Instruction interface tags
func (i Add) instructionTag()    {}
func (i Int) instructionTag()    {}
func (i Return) instructionTag() {}
