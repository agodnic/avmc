//go:build ignore

package main

import (
	"fmt"
	"os"
	"strings"

	"github.com/agodnic/avmc/tealspec"
)

func main() {

	spec := tealspec.MustParse()

	// Add missing mnemonics that do not translate 1:1 to opcodes
	fakeOpcodes := []tealspec.Op{
		{
			Name: "byte",
			ImmediateNote: []tealspec.Immediate{
				{
					Name:     "I",
					Encoding: "[]byte",
				},
			},
		},
		{
			Name: "int",
			ImmediateNote: []tealspec.Immediate{
				{
					Name:     "I",
					Encoding: "uint64",
				},
			},
		},
		{
			Name: "label",
			ImmediateNote: []tealspec.Immediate{
				{
					Name:     "I",
					Encoding: "string",
				},
			},
		},
	}
	spec.Ops = append(spec.Ops, fakeOpcodes...)

	// Open the output file
	file, err := os.OpenFile("generated_mnemonics.go", os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0644)
	if err != nil {
		msg := fmt.Sprintf("failed to open file: %v", err)
		panic(msg)
	}
	defer file.Close()

	// Generate a struct for each opcode
	fmt.Fprintf(file, "package mnemonic\n\n")
	fmt.Fprintf(file, "type (\n")
	for _, op := range spec.Ops {

		if _, ok := opcodeAllowed[op.Name]; !ok {
			continue
		}

		// Print a comment above the struct definition
		fmt.Fprintf(file, "\n")
		fmt.Fprintf(file, "\t// %s", op.Name)
		for _, imm := range op.ImmediateNote {
			fmt.Fprintf(file, " %s", imm.Name)
		}
		fmt.Fprintf(file, "\n")

		// Print the struct definition
		fmt.Fprintf(file, "\t%s struct{\n", opcodeNameToIdentifierName(op.Name))
		for _, imm := range op.ImmediateNote {
			fmt.Fprintf(file, "\t\t%s %s\n",
				uppercaseFirstCharacter(imm.Name),
				mapImmEncoding(op.Name, imm.Encoding),
			)
		}
		fmt.Fprintf(file, "\t}\n")
	}
	fmt.Fprintf(file, ")\n")

	// Generate interface implementations for a little bit of extra type safety
	fmt.Fprintf(file, "\n")
	for _, op := range spec.Ops {

		if _, ok := opcodeAllowed[op.Name]; !ok {
			continue
		}

		fmt.Fprintf(file, "func (m %s) mnemonicTag() {}\n", opcodeNameToIdentifierName(op.Name))
	}

}

// uppercaseFirstCharacter sets the first character of the input string to uppercase.
func uppercaseFirstCharacter(s string) string {
	return strings.ToUpper(s[:1]) + strings.ToLower(s[1:])
}

// opcodeNameToIdentifierName creates an alphanumeric identifier name for a given opcode mnemonic
//
// For instance, the + opcode is mapped to Add, and the == opcode is mapped to Eq.
//
// TODO for the sake of correctness, this function could detect invalid opcode names and fail.
func opcodeNameToIdentifierName(name string) string {
	switch name {
	case "+":
		return "Add"
	case "-":
		return "Sub"
	case "/":
		return "Div"
	case "*":
		return "Mul"
	case "<":
		return "Lt"
	case ">":
		return "Gt"
	case "<=":
		return "Lte"
	case ">=":
		return "Gte"
	case "&&":
		return "LogicalAnd"
	case "||":
		return "LogicalOr"
	case "==":
		return "Eq"
	case "!=":
		return "Ne"
	case "!":
		return "LogicalNot"
	default:
		return uppercaseFirstCharacter(name)
	}
}

func mapImmEncoding(opcode string, encoding string) string {

	// Override operand types for jumps
	//
	// We have to do this because opcodes use an integer offset, but mnemonics use labels.
	switch opcode {
	case "b", "bnz", "bz":
		return "string"
	}

	// Translate operand types
	switch encoding {
	case "int16 (big-endian)":
		return "int16"
	case "uint8", "uint64", "string", "[]byte":
		return encoding
	default:
		msg := fmt.Sprintf("unknown encoding: %s", encoding)
		panic(msg)
	}
}

// opcodeAllowed determines which operands does the backend support
//
// We're usuing this allow-list approach in the early stages of the language
// to force a small set of opcodes we can control.
// We would not be able to deal with all opcodes at this stage.
var opcodeAllowed = map[string]bool{
	// fake opcodes
	"byte":  true,
	"int":   true,
	"label": true,

	// true opcodes
	"+":      true,
	"arg":    true,
	"b":      true,
	"bnz":    true,
	"bz":     true,
	"/":      true,
	"==":     true,
	">":      true,
	">=":     true,
	"len":    true,
	"<":      true,
	"<=":     true,
	"&&":     true,
	"!":      true,
	"||":     true,
	"*":      true,
	"!=":     true,
	"return": true,
	"sha256": true,
	"-":      true,
}
