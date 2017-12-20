#
# Makefile for the mgit project.
#
# Also known as: that rustdoc invocation is nasty, so it's time to
# make little shortcuts for common operations.
#

default :
	cargo build

release :
	cargo build --release

test :
	cargo test

doc :
	cargo rustdoc -- \
		--no-defaults \
		--passes collapse-docs \
		--passes unindent-comments \
		--passes strip-priv-imports
