watch *args:
    watchexec -c -w src -w Cargo.toml -- just {{ args }}

alias w := watch

[env("PROPTEST_CASES", "16640")]
test pattern="":
    cargo test {{ pattern}} --timings -- --test-threads=1 'serial_tests::'
    cargo test {{ pattern}} --timings --bin tori
    cargo test {{ pattern}} --timings --doc
    cargo test {{ pattern}} --timings --lib -- --skip 'serial_tests::'

alias t:= test

mutate:
    -just mutate-single -- --test-threads=1 serial_tests::
    -just mutate-single -- --skip serial_tests::

alias m := mutate

[private]
mutate-single *cargo_test_args:
    cargo mutants --iterate \
        -E '<impl Debug<' \
        -E '<impl From<' \
        -E '<impl std::fmt::Display for ' \
        -E 'print_help -> bool' \
        --output target/mutants \
        -- {{ cargo_test_args }}

cover:
    cargo llvm-cov --no-report
    cargo llvm-cov report --html
    cargo llvm-cov report \
        | tail -1 | awk \
        '{ print " [ Regions:", $4, "• Functions:", $7, "• Lines:", $10, "]" }'

alias c := cover

cover-open:
    cargo llvm-cov report --open

vet:
    cargo vet

deny:
    cargo deny check

check: && test vet deny
    cargo check
