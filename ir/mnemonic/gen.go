//go:build ignore

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/agodnic/avmc/tealspec"
)

func main() {

	// Read file contents
	bs, err := os.ReadFile("../../tealspec/langspec_v12.json")
	if err != nil {
		msg := fmt.Sprintf("failed to read file: %v", err)
		panic(msg)
	}

	// Parse as JSON
	var spec tealspec.LangSpec
	err = json.Unmarshal(bs, &spec)
	if err != nil {
		msg := fmt.Sprintf("failed to parse JSON: %v", err)
		panic(msg)
	}

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

		if _, ok := allowList[op.Name]; !ok {
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
		fmt.Fprintf(file, "\t%s struct{\n", mapOpcodeName(op.Name))
		for _, imm := range op.ImmediateNote {
			fmt.Fprintf(file, "\t\t%s %s\n",
				mapCase(imm.Name),
				mapImmEncoding(op.Name, imm.Encoding),
			)
		}
		fmt.Fprintf(file, "\t}\n")
	}
	fmt.Fprintf(file, ")\n")

	// Generate interface implementations for a little bit of extra type safety
	fmt.Fprintf(file, "\n")
	for _, op := range spec.Ops {

		if _, ok := allowList[op.Name]; !ok {
			continue
		}

		fmt.Fprintf(file, "func (m %s) mnemonicTag() {}\n", mapOpcodeName(op.Name))
	}

}

func mapCase(s string) string {
	return strings.ToUpper(s[:1]) + strings.ToLower(s[1:])
}

func mapOpcodeName(name string) string {
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
		return mapCase(name)
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

var allowList = map[string]bool{
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
