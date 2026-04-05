watch command="run" args="":
    DEBUG=${DEBUG:-} watchexec -c -w src -- cargo {{ command }} {{ args }}

alias w := watch

[script]
verify:
    for solver in cadical bitwuzla cvc5 kissat minisat z3; do
        printf '\n => trying solver %s\n\n' $solver; sleep 1
        timeout 20m \
            cargo kani --solver $solver \
            && return
    done

