#
# Makefile for the mgit project.
#
CARGO ?= cargo

ROOT := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))
FLAME_FEATURE_NAME = flame_mgit
FLAME_TARGET_DIR = $(ROOT)/target/flame


help :
	@printf "\n"
	@printf "usage: make <target> where target is one of:\n"
	@printf "\n"
	@printf "          fmt  Check formatting, show diff to project style\n"
	@printf "         lint  Check code with linter\n"
	@printf "         test  Run unit, integration, and doc tests\n"
	@printf "        check  Run all test/QA checks\n"
	@printf "\n"
	@printf "          doc  Generate internal and dep docs "
	@printf                 "to target/doc/\n"
	@printf "\n"
	@printf " check-update  Check for updates for deps\n"
	@printf "\n"
	@printf "          dev  Compile with dev profile to target/debug/\n"
	@printf "          rel  Compile with release profile "
	@printf                 "to target/release/\n"
	@printf "\n"
	@printf "        flame  Compile with $(FLAME_FEATURE_NAME) and dev "
	@printf                 "profile to target/flame/debug/\n"
	@printf "    flame-rel  Compile with $(FLAME_FEATURE_NAME) and "
	@printf                 "release profile to target/flame/release/\n"
	@printf "\n"


fmt :
	cd $(ROOT); $(CARGO) +nightly fmt -- --write-mode=diff

lint :
	cd $(ROOT); $(CARGO) +nightly clippy -- -Wclippy-pedantic

test :
	cd $(ROOT); $(CARGO) test

check : test fmt lint


doc :
	cd $(ROOT); $(CARGO) rustdoc -- --document-private-items


check-update:
	cd $(ROOT); $(CARGO) outdated


dev :
	cd $(ROOT); $(CARGO) build

flame :
	cd $(ROOT); CARGO_TARGET_DIR=$(FLAME_TARGET_DIR) \
		$(CARGO) +nightly build --features $(FLAME_FEATURE_NAME)

rel :
	cd $(ROOT); $(CARGO) build --release

flame-rel :
	cd $(ROOT); CARGO_TARGET_DIR=$(FLAME_TARGET_DIR) \
		$(CARGO) +nightly build \
			--features $(FLAME_FEATURE_NAME) \
			--release
