# Configuration
PROFILE ?= debug
TARGET_DIR := target/$(PROFILE)
PATCHELF := patchelf
CARGO := cargo

# The package name defined in examples/cuda-init-hook/Cargo.toml
EXAMPLE_PKG := cuda_init_hook
# The resulting library filename (Rust prepends 'lib' and appends '.so' for cdylibs)
HOOKS_SRC := lib$(EXAMPLE_PKG).so
# The folder name where we will store the masqueraded artifacts
HOOKS_OUT := $(EXAMPLE_PKG)

# Determine Cargo flags based on profile
CARGO_FLAGS :=
ifeq ($(PROFILE),release)
    CARGO_FLAGS += --release
endif

# The aliases the driver expects to see.
# libcuda.so.1 is usually the strict requirement for runtime linking.
ALIASES := libcuda.so libcuda.so.1

# Mark targets as phony
.PHONY: all hooks clean check-env cargo-build

# Default target: builds the rust crate and then masquerades it
all: check-env hooks

# Check for patchelf requirement
check-env:
	@which $(PATCHELF) > /dev/null || (echo "Error: 'patchelf' is required but not found." && exit 1)

# --- Reusable Recipe for Masquerading ---
# Args:
#   $1: Source .so filename (e.g., libkernelflow_hooks.so)
#   $2: Output folder name (e.g., kernelflow_hooks)
define generate_masquerade
	@echo "Generating masquerade artifacts for $2..."
	@# 1. Prepare Output Directory
	@rm -rf $(TARGET_DIR)/$2
	@mkdir -p $(TARGET_DIR)/$2

	@# 2. Generate ld.so.preload
	@# This file tells the interposer loader to load the files listed inside.
	@printf "/libcuda.so\n/libcuda.so.1" > $(TARGET_DIR)/$2/ld.so.preload

	@# 3. Copy source to aliases and patch SONAME
	@# CRITICAL: We use patchelf to change the internal SONAME.
	@# Even if we rename the file to libcuda.so, the dynamic linker looks at
	@# the internal binary header. If it still says "libkernelflow_hooks.so",
	@# strict applications might reject it.
	@src="$(TARGET_DIR)/$1"; \
	if [ -f "$$src" ]; then \
		for alias in $(ALIASES); do \
			dst="$(TARGET_DIR)/$2/$$alias"; \
			echo "  -> Creating $$alias from $$src"; \
			cp "$$src" "$$dst"; \
			$(PATCHELF) --set-soname "$$alias" "$$dst"; \
			echo "     [OK] SONAME patched to $$alias"; \
		done \
	else \
		echo "  [ERROR] Source artifact '$$src' not found." >&2; \
		echo "          Did cargo build fail?" >&2; \
		exit 1; \
	fi
endef

# Build the specific example library
cargo-build:
	@echo "Building package '$(EXAMPLE_PKG)' via Cargo ($(PROFILE))..."
	$(CARGO) build $(CARGO_FLAGS) -p $(EXAMPLE_PKG)

# Hooks depends on cargo-build to ensure artifacts exist
hooks: cargo-build
	$(call generate_masquerade,$(HOOKS_SRC),$(HOOKS_OUT))
	@echo "Build complete. Artifacts located in: $(TARGET_DIR)/$(HOOKS_OUT)"

clean:
	$(CARGO) clean
	rm -rf $(TARGET_DIR)/$(HOOKS_OUT)
