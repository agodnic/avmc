package codegen

import (
	"github.com/agodnic/avmc/ir/ast"
	"github.com/agodnic/avmc/ir/mnemonic"
)

func generateFuncDecl(fn ast.FuncDecl) []mnemonic.Mnemonic {

	var mnemonics []mnemonic.Mnemonic

	for _, stmt := range fn.Body {
		mnemonics = append(mnemonics, generateStmt(stmt)...)
	}

	return mnemonics
}

func generateStmt(stmt ast.Stmt) (mnemonics []mnemonic.Mnemonic) {

	switch i := stmt.(type) {
	case ast.Return:
		mnemonics = append(mnemonics, generateExpr(i.Expr)...)
		mnemonics = append(mnemonics, mnemonic.Return{})
	case ast.If:
		elseLabel := i.BaseLabelsName + "_else"
		endLabel := i.BaseLabelsName + "_end"

		// test block
		mnemonics = append(mnemonics, mnemonic.Int{I: 1})
		mnemonics = append(mnemonics, mnemonic.Bnz{Target: elseLabel})

		// then block
		for j := range i.Then {
			mnemonics = append(mnemonics, generateStmt(i.Then[j])...)
		}
		mnemonics = append(mnemonics, mnemonic.B{Target: endLabel})

		// else block
		mnemonics = append(mnemonics, mnemonic.Label{I: elseLabel})
		for j := range i.Else {
			mnemonics = append(mnemonics, generateStmt(i.Else[j])...)
		}

		// end block
		mnemonics = append(mnemonics, mnemonic.Label{I: endLabel})
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}

	return mnemonics
}

func generateExpr(expr ast.Expr) (mnemonics []mnemonic.Mnemonic) {

	switch i := expr.(type) {
	case ast.IntLit:
		mnemonics = []mnemonic.Mnemonic{
			mnemonic.Int{I: i.V0},
		}
		return mnemonics
	case ast.BytesLit:
		mnemonics = []mnemonic.Mnemonic{
			mnemonic.Byte{I: i.V0},
		}
		return mnemonics
	case ast.BinaryExpr:
		mnemonics = append(mnemonics, generateExpr(i.L)...)
		mnemonics = append(mnemonics, generateExpr(i.R)...)
		mnemonics = append(mnemonics, i.Op)
		return mnemonics
	case ast.UnaryExpr:
		mnemonics = append(mnemonics, generateExpr(i.Expr)...)
		mnemonics = append(mnemonics, i.Op)
		return mnemonics
	case ast.FunctionCall:

		// Mnemonics with embedded arguments
		if i.FuncName == "arg" {
			if len(i.Args) != 1 {
				//TODO msg := fmt(...)
				panic("invalid number of arguments for arg")
			}
			n, ok := i.Args[0].(ast.IntLit)
			if !ok {
				//TODO msg := fmt(...)
				panic("invalid argument type for arg")
			}

			// FIXME hard cast. Probably the input structure should have the right type
			mnemonics = append(mnemonics, mnemonic.Arg{N: uint8(n.V0)})

			return mnemonics
		}

		// Mnemonics without embedded arguments
		opcode, ok := builtinFunctionToMnemonic[i.FuncName]
		if !ok {
			//TODO msg := fmt(...)
			panic("unknown function")
		}

		for j := range i.Args {
			mnemonics = append(mnemonics, generateExpr(i.Args[j])...)
		}

		mnemonics = append(mnemonics, opcode)
		return mnemonics
	default:
		//TODO msg := fmt(...)
		panic("not iplemented")
	}

}

var builtinFunctionToMnemonic = map[string]mnemonic.Mnemonic{
	"len":    mnemonic.Len{},
	"sha256": mnemonic.Sha256{},
	//"txn.Sender":                    mnemonic.Txn{Field: "Sender"},
	//"txn.Fee":                       mnemonic.Txn{Field: "Fee"},
	//"txn.FirstValid":                mnemonic.Txn{Field: "FirstValid"},
	//"txn.FirstValidTime":            mnemonic.Txn{Field: "FirstValidTime"},
	//"txn.LastValid":                 mnemonic.Txn{Field: "LastValid"},
	//"txn.Note":                      mnemonic.Txn{Field: "Note"},
	//"txn.Lease":                     mnemonic.Txn{Field: "Lease"},
	//"txn.Receiver":                  mnemonic.Txn{Field: "Receiver"},
	//"txn.Amount":                    mnemonic.Txn{Field: "Amount"},
	//"txn.CloseRemainderTo":          mnemonic.Txn{Field: "CloseRemainderTo"},
	//"txn.VotePK":                    mnemonic.Txn{Field: "VotePK"},
	//"txn.SelectionPK":               mnemonic.Txn{Field: "SelectionPK"},
	//"txn.VoteFirst":                 mnemonic.Txn{Field: "VoteFirst"},
	//"txn.VoteLast":                  mnemonic.Txn{Field: "VoteLast"},
	//"txn.VoteKeyDilution":           mnemonic.Txn{Field: "VoteKeyDilution"},
	//"txn.Type":                      mnemonic.Txn{Field: "Type"},
	//"txn.TypeEnum":                  mnemonic.Txn{Field: "TypeEnum"},
	//"txn.XferAsset":                 mnemonic.Txn{Field: "XferAsset"},
	//"txn.AssetAmount":               mnemonic.Txn{Field: "AssetAmount"},
	//"txn.AssetSender":               mnemonic.Txn{Field: "AssetSender"},
	//"txn.AssetReceiver":             mnemonic.Txn{Field: "AssetReceiver"},
	//"txn.AssetCloseTo":              mnemonic.Txn{Field: "AssetCloseTo"},
	//"txn.GroupIndex":                mnemonic.Txn{Field: "GroupIndex"},
	//"txn.TxID":                      mnemonic.Txn{Field: "TxID"},
	//"txn.ApplicationID":             mnemonic.Txn{Field: "ApplicationID"},
	//"txn.OnCompletion":              mnemonic.Txn{Field: "OnCompletion"},
	//"txn.NumAppArgs":                mnemonic.Txn{Field: "NumAppArgs"},
	//"txn.NumAccounts":               mnemonic.Txn{Field: "NumAccounts"},
	//"txn.ApprovalProgram":           mnemonic.Txn{Field: "ApprovalProgram"},
	//"txn.ClearStateProgram":         mnemonic.Txn{Field: "ClearStateProgram"},
	//"txn.RekeyTo":                   mnemonic.Txn{Field: "RekeyTo"},
	//"txn.ConfigAsset":               mnemonic.Txn{Field: "ConfigAsset"},
	//"txn.ConfigAssetTotal":          mnemonic.Txn{Field: "ConfigAssetTotal"},
	//"txn.ConfigAssetDecimals":       mnemonic.Txn{Field: "ConfigAssetDecimals"},
	//"txn.ConfigAssetDefaultFrozen":  mnemonic.Txn{Field: "ConfigAssetDefaultFrozen"},
	//"txn.ConfigAssetUnitName":       mnemonic.Txn{Field: "ConfigAssetUnitName"},
	//"txn.ConfigAssetName":           mnemonic.Txn{Field: "ConfigAssetName"},
	//"txn.ConfigAssetURL":            mnemonic.Txn{Field: "ConfigAssetURL"},
	//"txn.ConfigAssetMetadataHash":   mnemonic.Txn{Field: "ConfigAssetMetadataHash"},
	//"txn.ConfigAssetManager":        mnemonic.Txn{Field: "ConfigAssetManager"},
	//"txn.ConfigAssetReserve":        mnemonic.Txn{Field: "ConfigAssetReserve"},
	//"txn.ConfigAssetFreeze":         mnemonic.Txn{Field: "ConfigAssetFreeze"},
	//"txn.ConfigAssetClawback":       mnemonic.Txn{Field: "ConfigAssetClawback"},
	//"txn.FreezeAsset":               mnemonic.Txn{Field: "FreezeAsset"},
	//"txn.FreezeAssetAccount":        mnemonic.Txn{Field: "FreezeAssetAccount"},
	//"txn.FreezeAssetFrozen":         mnemonic.Txn{Field: "FreezeAssetFrozen"},
	//"txn.NumAssets":                 mnemonic.Txn{Field: "NumAssets"},
	//"txn.NumApplications":           mnemonic.Txn{Field: "NumApplications"},
	//"txn.GlobalNumUint":             mnemonic.Txn{Field: "GlobalNumUint"},
	//"txn.GlobalNumByteSlice":        mnemonic.Txn{Field: "GlobalNumByteSlice"},
	//"txn.LocalNumUint":              mnemonic.Txn{Field: "LocalNumUint"},
	//"txn.LocalNumByteSlice":         mnemonic.Txn{Field: "LocalNumByteSlice"},
	//"txn.ExtraProgramPages":         mnemonic.Txn{Field: "ExtraProgramPages"},
	//"txn.Nonparticipation":          mnemonic.Txn{Field: "Nonparticipation"},
	//"txn.NumLogs":                   mnemonic.Txn{Field: "NumLogs"},
	//"txn.CreatedAssetID":            mnemonic.Txn{Field: "CreatedAssetID"},
	//"txn.CreatedApplicationID":      mnemonic.Txn{Field: "CreatedApplicationID"},
	//"txn.LastLog":                   mnemonic.Txn{Field: "LastLog"},
	//"txn.StateProofPK":              mnemonic.Txn{Field: "StateProofPK"},
	//"txn.NumApprovalProgramPages":   mnemonic.Txn{Field: "NumApprovalProgramPages"},
	//"txn.NumClearStateProgramPages": mnemonic.Txn{Field: "NumClearStateProgramPages"},
}
