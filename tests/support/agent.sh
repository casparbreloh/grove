#!/bin/sh

printf '%s\n' "$$" >> "$GROVE_TEST_AGENT_PID"
printf 'cwd=%s\nsession=%s\ndirective=%s\n' "$PWD" "${ZMX_SESSION-}" "${GROVE_DIRECTIVE_CD_FILE-absent}" >> "$GROVE_TEST_AGENT_LOG"
for argument do
  printf 'arg=<%s>\n' "$argument" >> "$GROVE_TEST_AGENT_LOG"
done

session_file=
while test "$#" -gt 0; do
  if test "$1" = --session; then shift; session_file=$1; fi
  shift
done

if test -n "$session_file"; then
  : > "$session_file"
fi

sleep 0.1
printf 'grove-test-agent-ready\n'
if test -n "$session_file"; then
  IFS= read -r prompt || exit 0
  if test -n "${GROVE_EXECUTABLE-}"; then
    : > "$GROVE_NAMING_CLAIM"
    printf 'grove-test-prompt-received\n'
    (sleep 1; printf '%s' "$prompt" | "$GROVE_EXECUTABLE" __name) &
  fi
fi

exec sleep 30
