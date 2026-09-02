# Diagnostic codes

Codes are stable: once assigned, a code keeps its meaning.

| Code | Message | Stage |
|---|---|---|
| `E0001` | `unexpected character` | lexer |
| `E0002` | `expected {expected}, found {found}` | parser |
| `E0003` | `integer literal out of range` | parser |
| `E0004` | ``unknown type `{name}` `` | type checker |
| `E0005` | `missing return` | type checker |
| `E0006` | `unreachable statement` | type checker |
| `E0007` | ``duplicate function `{name}` `` | lowering |
| `E0008` | ``missing entry point `approval` `` | emitter |
| `E0009` | ``` `{opcode}` requires TEAL version {min}, target is {target} ``` | emitter |
