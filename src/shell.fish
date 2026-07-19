function grove
    set -l directive (mktemp); or return
    set -lx GROVE_DIRECTIVE_CD_FILE $directive
    set -l executable grove
    if set -q GROVE_EXECUTABLE
        set executable $GROVE_EXECUTABLE
    end
    command $executable $argv
    set -l command_status $status
    if test -s $directive
        builtin cd -- (string collect < $directive)
    end
    rm -f -- $directive
    return $command_status
end
