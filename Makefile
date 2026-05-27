
# We're intentionally pinning the version here to avoid having
# surprise updates break our stuff in the middle of a pull request.
GOCC_VERSION := v1.0.2

all:
	@echo "Checking for gocc updates..."
	@latest_gocc_version="$$(go list -m -f '{{.Version}}' github.com/goccmack/gocc@latest 2>/dev/null || true)"; \
	if [ -n "$$latest_gocc_version" ] && [ "$$latest_gocc_version" != "$(GOCC_VERSION)" ]; then \
		echo "WARNING: gocc is pinned to $(GOCC_VERSION), latest is $$latest_gocc_version"; \
	elif [ -z "$$latest_gocc_version" ]; then \
		echo "WARNING: Could not check latest gocc version (continuing with pinned $(GOCC_VERSION))"; \
	fi
	go install github.com/goccmack/gocc@$(GOCC_VERSION)

	mkdir -p parser/generated
	cd parser/generated && gocc -p github.com/agodnic/avmc/parser/generated -v ../grammar.bnf && cd ..
	go generate ./...
	go test ./...
