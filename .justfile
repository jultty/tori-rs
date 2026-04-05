watch *args:
    watchexec -c -w src -w Cargo.toml -- just {{ args }}

alias w := watch

[script]
verify:
    for solver in cadical bitwuzla cvc5 kissat minisat z3; do
        printf '\n => trying solver %s\n\n' $solver; sleep 1
        timeout 20m \
            cargo kani --solver $solver \
            && return
    done

[env("PROPTEST_CASES", "16640")]
test pattern="":
    cargo test {{ pattern}} --timings -- --test-threads=1 'serial_tests::'
    cargo test {{ pattern}} --timings --bin tori
    cargo test {{ pattern}} --timings --doc
    cargo test {{ pattern}} --timings --lib -- --skip 'serial_tests::'



mutate:
    cargo mutants --iterate \
        -E '<impl Debug<' \
        -E '<impl From<' \
        -E '<impl std::fmt::Display for ' \
        -E 'print_help -> bool' \
        --output target/mutants


cover:
    cargo llvm-cov --no-report
    cargo llvm-cov report --html
    cargo llvm-cov report \
        | tail -1 | awk \
        '{ print " [ Regions:", $4, "• Functions:", $7, "• Lines:", $10, "]" }'

cover-open:
    cargo llvm-cov report --open

