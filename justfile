default: check

[doc("Everything that must be green before a commit.")]
check: fmt clippy stories comments wasm

[doc("SF-001 to SF-014: SpecForge's own stories, compiled, assessed and run by SpecForge; a stale docs/views/trace.md fails.")]
stories:
    cargo test -q --all-features --test stories

[doc("Redraw docs/views/trace.md from the specifications and the step definitions.")]
views:
    SPECFORGE_VIEW=draw cargo test -q --all-features --test stories

[doc("Formatting is checked, never applied, so a red is a diff you can read.")]
fmt:
    @cargo fmt --check || (echo 'fmt: run `cargo fmt` and commit the result' && false)

clippy:
    cargo clippy -q --all-targets --all-features -- -D warnings

[doc("No comments; names and types carry the meaning. Doc comments only.")]
comments:
    @python3 comments.py

[doc("The core, without the cucumber bridge, builds for the browser.")]
wasm:
    cargo check -q --target wasm32-unknown-unknown

[doc("Not in `check`: native mutation of the crate; a survivor is a clause no story pins.")]
mutants:
    cargo mutants --all-features --no-shuffle

[doc("Run `just <args>` in a bubblewrap sandbox: no network, an empty home, the project the only writable path besides /tmp, cargo offline.")]
isolated *args='check':
    #!/usr/bin/env bash
    set -euo pipefail
    project="$(pwd -P)"
    rustup_home="$(rustup show home)"
    exec bwrap \
        --unshare-all --die-with-parent --new-session --clearenv \
        --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/bin /sbin --symlink usr/lib /lib --symlink usr/lib /lib64 \
        --ro-bind /etc /etc --proc /proc --dev /dev --tmpfs /tmp --dir /tmp/home \
        --ro-bind "$rustup_home" /opt/rustup \
        --ro-bind "$HOME/.cargo/registry" /tmp/cargo/registry \
        --ro-bind "$HOME/.cargo/git" /tmp/cargo/git \
        --bind "$project" "$project" --chdir "$project" \
        --remount-ro / \
        --setenv HOME /tmp/home --setenv PATH /usr/bin \
        --setenv RUSTUP_HOME /opt/rustup --setenv CARGO_HOME /tmp/cargo \
        --setenv CARGO_NET_OFFLINE true --setenv CARGO_TARGET_DIR "$project/target/isolated" \
        --setenv TERM dumb --setenv LANG C.UTF-8 \
        just {{args}}
