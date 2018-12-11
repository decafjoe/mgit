#
# Makefile for the mgit project.
#
CARGO ?= cargo

ROOT := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))


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


fmt :
	cd $(ROOT); $(CARGO) fmt -- --check

lint :
	cd $(ROOT); $(CARGO) clippy -- -W 'clippy::pedantic'

test :
	cd $(ROOT); $(CARGO) test

check : test fmt lint


doc :
	cd $(ROOT); $(CARGO) rustdoc -- --document-private-items


check-update:
	cd $(ROOT); $(CARGO) outdated


dev :
	cd $(ROOT); $(CARGO) build

rel :
	cd $(ROOT); $(CARGO) build --release
