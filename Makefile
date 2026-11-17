# The resolved tokens are sokkel-design's, fetched from GitHub at the commit sokkel-design.json pins,
# so only committed contract work lands here. Read, never written.

RESOLVED := crates/core/tokens/tokens.resolved.json
PIN := $(shell python3 -c "import json;print(json.load(open('sokkel-design.json'))['commit'])")
REF ?= $(PIN)
FETCH = gh api -H "Accept: application/vnd.github.raw" "repos/winterop-com/sokkel-design/contents/tokens/tokens.resolved.json?ref=$(1)"

.PHONY: help tokens tokens-check fmt fmt-check check test specimens all clean

help:
	@echo "make tokens        fetch sokkel-design's resolved tokens at the pin (REF=<commit> moves it)"
	@echo "make tokens-check  fail when the copy is not sokkel-design's"
	@echo "make fmt           format the workspace"
	@echo "make check         clippy with warnings denied"
	@echo "make test          every test"
	@echo "make specimens     draw the spike and the parts sheet into specimens/ as SVG, PNG and text"
	@echo "make all           fmt-check, check and test"
	@echo "make clean         remove the build and the specimens"

tokens:
	$(call FETCH,$(REF)) > $(RESOLVED)
	python3 -c "import json;p='sokkel-design.json';d=json.load(open(p));d['commit']='$$(gh api repos/winterop-com/sokkel-design/commits/$(REF) -q .sha)';open(p,'w').write(json.dumps(d,indent=4)+'\\n')"

tokens-check:
	@$(call FETCH,$(PIN)) | cmp -s - $(RESOLVED) \
		|| (echo "$(RESOLVED) is not sokkel-design's at $(PIN): run make tokens"; exit 1)

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

check:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

specimens:
	mkdir -p specimens
	for s in "tidevann dark 120 30" "tidevann dark 80 24" "tidevann light 120 30" "terminal dark 120 30"; do \
		set -- $$s; \
		cargo run -q -p sokkel-tui-spike -- $$1 $$2 $$3 $$4 specimens/$$1-$$2-$$3 || exit 1; \
		if command -v rsvg-convert >/dev/null; then rsvg-convert -z 2 specimens/$$1-$$2-$$3.svg -o specimens/$$1-$$2-$$3.png; fi; \
	done
	for s in "controls 40 52" "page 84 120"; do \
		set -- $$s; sheet=$$1; tall=$$2; narrow=$$3; \
		for v in "tidevann dark 120 $$tall" "tidevann dark 80 $$narrow" "tidevann light 120 $$tall" "tidevann light 80 $$narrow" "terminal dark 120 $$tall"; do \
			set -- $$v; \
			cargo run -q -p sokkel-tui-parts -- $$sheet $$1 $$2 $$3 $$4 specimens/$$sheet-$$1-$$2-$$3 || exit 1; \
			if command -v rsvg-convert >/dev/null; then rsvg-convert -z 2 specimens/$$sheet-$$1-$$2-$$3.svg -o specimens/$$sheet-$$1-$$2-$$3.png; fi; \
		done; \
		cargo run -q -p sokkel-tui-parts -- $$sheet tidevann dark 80 $$narrow specimens/$$sheet-ascii-nocolor-80 ascii nocolor || exit 1; \
	done

all: fmt-check check test

clean:
	cargo clean
	rm -rf specimens
