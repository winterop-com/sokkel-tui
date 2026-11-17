# The resolved tokens, the index (cells, color depth) and the glyphs are sokkel-design's, fetched
# from GitHub at the commit sokkel-design.json pins, so only committed contract work lands here.
# Read, never written.

TOKENS := crates/core/tokens
FILES := tokens/tokens.resolved.json index.json glyphs.json
PIN := $(shell python3 -c "import json;print(json.load(open('sokkel-design.json'))['commit'])")
REF ?= $(PIN)
FETCH = gh api -H "Accept: application/vnd.github.raw" "repos/winterop-com/sokkel-design/contents/$(1)?ref=$(2)"

.PHONY: help tokens tokens-check fmt fmt-check check test specimens all clean

help:
	@echo "make tokens        fetch sokkel-design's tokens, index and glyphs at the pin (REF=<commit> moves it)"
	@echo "make tokens-check  fail when the copy is not sokkel-design's"
	@echo "make fmt           format the workspace"
	@echo "make check         clippy with warnings denied"
	@echo "make test          every test"
	@echo "make specimens     draw the spike and the parts sheet into specimens/ as SVG, PNG and text"
	@echo "make all           fmt-check, check and test"
	@echo "make clean         remove the build and the specimens"

tokens:
	for f in $(FILES); do $(call FETCH,$$f,$(REF)) > $(TOKENS)/$$(basename $$f) || exit 1; done
	python3 -c "import json;p='sokkel-design.json';d=json.load(open(p));d['commit']='$$(gh api repos/winterop-com/sokkel-design/commits/$(REF) -q .sha)';open(p,'w').write(json.dumps(d,indent=4)+'\\n')"

tokens-check:
	@for f in $(FILES); do $(call FETCH,$$f,$(PIN)) | cmp -s - $(TOKENS)/$$(basename $$f) \
		|| { echo "$(TOKENS)/$$(basename $$f) is not sokkel-design's at $(PIN): run make tokens"; exit 1; }; done

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
	cargo build -q -p sokkel-tui --examples
	for s in "tidevann dark 120 30" "tidevann dark 80 24" "tidevann light 120 30" "tidevann light 80 24" \
		"terminal dark 120 30" "tidevann dark 80 24 drawer" "tidevann light 120 30 collapsed compact"; do \
		set -- $$s; t=$$1; m=$$2; c=$$3; r=$$4; shift 4; \
		out=specimens/$$t-$$m-$$c$$(for f in "$$@"; do printf -- '-%s' $$f; done); \
		target/debug/examples/06_shell $$t $$m $$c $$r $$out "$$@" || exit 1; \
		if command -v rsvg-convert >/dev/null; then rsvg-convert -z 2 $$out.svg -o $$out.png; fi; \
	done
	for s in "controls 44 66" "page 84 120" "table 90 90" "overlays 56 90" "palette 64 100"; do \
		set -- $$s; sheet=$$1; tall=$$2; narrow=$$3; \
		for v in "tidevann dark 120 $$tall" "tidevann dark 80 $$narrow" "tidevann light 120 $$tall" "tidevann light 80 $$narrow" "terminal dark 120 $$tall"; do \
			set -- $$v; \
			target/debug/examples/08_gallery $$sheet $$1 $$2 $$3 $$4 specimens/$$sheet-$$1-$$2-$$3 || exit 1; \
			if command -v rsvg-convert >/dev/null; then rsvg-convert -z 2 specimens/$$sheet-$$1-$$2-$$3.svg -o specimens/$$sheet-$$1-$$2-$$3.png; fi; \
		done; \
		target/debug/examples/08_gallery $$sheet tidevann dark 80 $$narrow specimens/$$sheet-ascii-nocolor-80 ascii nocolor || exit 1; \
	done
	for s in "dark 120 30" "dark 80 24" "light 120 30" "light 80 24"; do \
		set -- $$s; \
		for k in "rest" "palette ctrl-k"; do \
			set -- $$s $$k; m=$$1; c=$$2; r=$$3; name=$$4; shift 4; \
			out=specimens/runs-$$m-$$c-$$name; \
			target/debug/examples/07_runs $$m --frame $$c $$r $$out "$$@" || exit 1; \
			if command -v rsvg-convert >/dev/null; then rsvg-convert -z 2 $$out.svg -o $$out.png; fi; \
		done; \
	done

all: fmt-check check test

clean:
	cargo clean
	rm -rf specimens
