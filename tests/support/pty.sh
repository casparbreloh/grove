#!/bin/sh

if script --version 2>&1 | grep -q util-linux; then
  command='stty rows "${GROVE_TEST_ROWS:-40}" cols "${GROVE_TEST_COLUMNS:-120}"; exec'
  for argument do
    escaped=$(printf '%s' "$argument" | sed "s/'/'\\\\''/g")
    command="$command '$escaped'"
  done
  exec script -q -e -c "$command" /dev/null
fi

exec script -q /dev/null /bin/sh -c \
  'stty rows "${GROVE_TEST_ROWS:-40}" cols "${GROVE_TEST_COLUMNS:-120}"; exec "$@"' \
  grove-test-pty "$@"
