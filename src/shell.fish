function grove
    set -l directive (mktemp); or return
    set -lx GROVE_DIRECTIVE_CD_FILE $directive
    command grove $argv
    set -l command_status $status
    if test $command_status -eq 0; and test -s $directive
        builtin cd -- (string collect < $directive)
    end
    rm -f -- $directive
    return $command_status
end
