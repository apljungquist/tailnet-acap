## Configuration
## =============

# Parameters
# ----------

# Name of package containing the app to be built.
# Rust does not enforce that the path to the package matches the package name, but
# this makefile does to keep things simple.
export AXIS_PACKAGE ?= tailnet

# The architecture that will be assumed when interacting with the device.
export AXIS_DEVICE_ARCH ?= aarch64

# The IP address of the device to interact with.
export AXIS_DEVICE_IP ?= 192.168.0.90

# The username to use when interacting with the device.
export AXIS_DEVICE_USER ?= root

# The password to use when interacting with the device.
export AXIS_DEVICE_PASS ?= pass

# Reproducible and stable results by default
export SOURCE_DATE_EPOCH ?= 0

.DEFAULT_GOAL := help
.DELETE_ON_ERROR: ;
.SECONDARY:
.SUFFIXES: ;
.PHONY: .FORCE

## Verbs
## =====

help:
	@mkhelp $(firstword $(MAKEFILE_LIST))

## Build <AXIS_PACKAGE> for <AXIS_DEVICE_ARCH>
##
## The EAP is placed in target/acap/.
build: crates/$(AXIS_PACKAGE)/LICENSE
	cargo-acap-build \
		--arch $(AXIS_DEVICE_ARCH) \
		-- \
		--package $(AXIS_PACKAGE) \
		--profile app

## Install <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS> and assuming architecture <AXIS_DEVICE_ARCH>
install:
	cargo-acap-sdk install \
	-- \
	--package $(AXIS_PACKAGE) \
	--profile app

## Remove <AXIS_PACKAGE> from <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
remove:
	cargo-acap-sdk remove

## Start <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
start:
	cargo-acap-sdk start

## Stop <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
stop:
	cargo-acap-sdk stop

## Restart <AXIS_PACKAGE> on <AXIS_DEVICE_IP> using password <AXIS_DEVICE_PASS>
restart:
	cargo-acap-sdk restart

## Checks
## ------

## _
check: check_build check_docs check_format check_generated_files check_lint check_tests
.PHONY: check

## _
check_build: check_build_host check_build_target
.PHONY: check_build

check_build_host:
	cargo build \
		--locked \
		--workspace
.PHONY: check_build_host

check_build_target: crates/$(AXIS_PACKAGE)/LICENSE
	cargo-acap-build \
		-- \
		--locked \
		--package $(AXIS_PACKAGE) \
		--profile app
	find target/acap/ -name '*.eap' | LC_ALL=C sort | xargs du --apparent-size
.PHONY: check_build_target

## _
check_docs:
	RUSTDOCFLAGS="-Dwarnings" cargo doc \
		--document-private-items \
		--locked \
		--no-deps \
		--workspace
.PHONY: check_docs

## _
check_format:
	cargo fmt \
		--check \
		-- \
		--config imports_granularity=Crate,group_imports=StdExternalCrate
.PHONY: check_format

## _
check_generated_files: Cargo.lock
	git update-index -q --refresh
	git --no-pager diff --exit-code HEAD -- $^
.PHONY: check_generated_files

## _
check_lint: check_lint_host check_lint_target
.PHONY: check_lint

check_lint_host:
	cargo clippy \
		--all-targets \
		--locked \
		--no-deps \
		--workspace \
		-- \
		-Dwarnings
.PHONY: check_lint_host

check_lint_target:
	cargo clippy \
		--all-targets \
		--locked \
		--no-deps \
		--target aarch64-unknown-linux-gnu \
		--workspace \
		-- \
		-Dwarnings
.PHONY: check_lint_target

## _
check_tests:
	cargo test \
		--all-targets \
		--locked \
		--workspace
.PHONY: check_tests

## Fixes
## -----

## _
fix_format:
	cargo fmt \
	-- \
	--config imports_granularity=Crate,group_imports=StdExternalCrate
.PHONY: fix_format

## _
fix_lint:
	cargo clippy --fix
.PHONY: fix_lint


## Nouns
## =====

Cargo.lock: Cargo.toml $(wildcard crates/*/Cargo.toml) $(wildcard crates/*/Cargo.toml)
	cargo metadata --format-version=1 > /dev/null

crates/$(AXIS_PACKAGE)/LICENSE: about.toml about.hbs Cargo.lock
	cargo about generate \
		--config about.toml \
		--fail \
		--manifest-path crates/$(AXIS_PACKAGE)/Cargo.toml \
		--output-file $@ \
		about.hbs
