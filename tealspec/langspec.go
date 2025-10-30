package tealspec

type LangSpec struct {
	Version         uint8       `json:"Version"`
	LogicSigVersion uint8       `json:"LogicSigVersion"`
	NamedTypes      []NamedType `json:"NamedTypes"`
	Ops             []Op        `json:"Ops"`
}

type NamedType struct {
	Name         string  `json:"Name"`
	Abbreviation string  `json:"Abbreviation"`
	Bound        [2]uint `json:"Bound"`
	AVMType      string  `json:"AVMType"`
}

type Op struct {
	Opcode            uint8       `json:"Opcode"`
	Name              string      `json:"Name"`
	Args              []string    `json:"Args"`
	Returns           []string    `json:"Returns"`
	Size              uint8       `json:"Size"`
	ArgEnum           []string    `json:"ArgEnum"`
	ArgEnumTypes      []string    `json:"ArgEnumTypes"`
	DocCost           string      `json:"DocCost"`
	Doc               string      `json:"Doc"`
	DocExtra          string      `json:"DocExtra"`
	ImmediateNote     []Immediate `json:"ImmediateNote"`
	IntroducedVersion uint8       `json:"IntroducedVersion"`
	Groups            []string    `json:"Groups"`
}

type Immediate struct {
	Comment   string `json:"Comment"`
	Encoding  string `json:"Encoding"`
	Name      string `json:"Name"`
	Reference string `json:"Reference"`
}
