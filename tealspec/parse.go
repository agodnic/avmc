package tealspec

import (
	_ "embed"
	"encoding/json"
	"fmt"
)

//go:embed langspec_v12.json
var langspecFileContents string

func MustParse() LangSpec {

	var result LangSpec

	err := json.Unmarshal([]byte(langspecFileContents), &result)
	if err != nil {
		msg := fmt.Sprintf("failed to parse langspec from JSON: %v", err)
		panic(msg)
	}

	return result
}
