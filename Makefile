#
# Makefile for the mgit project.
#
CARGO ?= cargo

ROOT := $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))


help :
	@printf "\n"
	@printf "usage: make <target> where target is one of:\n"
	@printf "\n"
	@printf "  dev      Compile with dev profile to target/debug/\n"
	@printf "  release  Compile with release profile to target/release/\n"
	@printf "\n"


dev :
	cd $(ROOT); $(CARGO) build

release :
	cd $(ROOT); $(CARGO) build --release
