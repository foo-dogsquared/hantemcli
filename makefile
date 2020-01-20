.PHONY: 'build create-docs'

build:
	cargo build --release

create-docs:
	asciidoctor --backend manpage docs/manual.adoc --out-file hantemcli.1
