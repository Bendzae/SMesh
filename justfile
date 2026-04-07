# Run an example with hot patching (sub-second reload on save)
# Requires: cargo install dioxus-cli@0.7.0-rc.1
dev example:
    dx serve --example {{example}} --hot-patch

# Run an example normally (no hot reload)
run example:
    cargo run --example {{example}}

# List all available examples
list:
    @ls examples/*.rs | sed 's|examples/||;s|\.rs||'
