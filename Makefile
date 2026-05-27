
all:
	go install github.com/goccmack/gocc@latest
	mkdir -p parser/generated
	cd parser/generated && gocc -p github.com/agodnic/avmc/parser/generated -v ../grammar.bnf && cd ..
	go generate ./...
	go test ./...
