
all:
	# We're intentionally pinning the version here to avoid having
	# surprise updates break our stuff in the middle of a pull request.
	#
	# The downside is that we'll have to update the dependency manually
	# or somehow find a way to get notified if there are newer versions.
	go install github.com/goccmack/gocc@v1.0.2

	mkdir -p parser/generated
	cd parser/generated && gocc -p github.com/agodnic/avmc/parser/generated -v ../grammar.bnf && cd ..
	go generate ./...
	go test ./...
