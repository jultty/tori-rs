watch command="run" args="":
    DEBUG=${DEBUG:-} watchexec -c -w src -- cargo {{ command }} {{ args }}

alias w := watch
