#!/bin/sh

mode=interactive
for argument do
  if test "$argument" = --print; then mode=print; fi
done

printf 'mode=%s\ncwd=%s\ndirective=%s\n' "$mode" "$PWD" "${GROVE_DIRECTIVE_CD_FILE-absent}" >> "$GROVE_TEST_AGENT_LOG"
for argument do
  printf 'arg=<%s>\n' "$argument" >> "$GROVE_TEST_AGENT_LOG"
done

if test "$mode" = print; then
  if test -n "${GROVE_TEST_TITLE_BLOCK-}"; then
    while test -e "$GROVE_TEST_TITLE_BLOCK"; do sleep 0.05; done
  fi
  if test -n "${GROVE_TEST_TITLE-}"; then
    printf '%s\n' "$GROVE_TEST_TITLE"
  fi
  exit "${GROVE_TEST_TITLE_EXIT-0}"
fi

session_dir=
while test "$#" -gt 0; do
  if test "$1" = --session-dir; then shift; session_dir=$1; fi
  shift
done

if test -n "$session_dir"; then
  mkdir -p "$session_dir"
  session_id=${GROVE_TEST_SESSION_ID-test-session}
  session_file="$session_dir/2026-01-01T00-00-00-000Z_${session_id}.jsonl"
  if test -n "${GROVE_TEST_SESSION_ID-}" || ! find "$session_dir" -maxdepth 1 -name '*.jsonl' -print -quit | grep -q .; then
    printf '{"type":"session","version":3,"id":"%s","timestamp":"2026-01-01T00:00:00.000Z","cwd":"%s"}\n' "$session_id" "$PWD" > "$session_file"
  else
    session_file=$(find "$session_dir" -maxdepth 1 -name '*.jsonl' -print -quit)
    session_id=$(sed -n '1s/.*"id":"\([^"]*\)".*/\1/p' "$session_file")
  fi

  if test -n "${GROVE_TEST_AGENT_PROMPT-}"; then
    if ! grep -q '"customType":"grove"' "$session_file"; then
      printf '{"type":"custom","id":"grove-link","parentId":null,"timestamp":"2026-01-01T00:00:00.001Z","customType":"grove","data":{"schema":1,"changeId":"%s"}}\n' "$GROVE_CHANGE_ID" >> "$session_file"
    fi
    title_file="$session_dir/.title-$session_id"
    (
      if printf '%s' "$GROVE_TEST_AGENT_PROMPT" | "$GROVE_EXECUTABLE" __title --change "$GROVE_CHANGE_ID" --session "$session_id" > "$title_file" 2>/dev/null; then
        title=$(tr -d '\r\n' < "$title_file")
        printf '{"type":"session_info","id":"grove-title","parentId":"grove-link","timestamp":"2026-01-01T00:00:00.002Z","name":"%s"}\n' "$title" >> "$session_file"
      fi
      rm -f "$title_file"
    ) </dev/null >/dev/null 2>/dev/null &
  fi
fi

printf 'grove-test-agent-ready\n'
if test -n "${GROVE_TEST_AGENT_BLOCK-}"; then
  while test -e "$GROVE_TEST_AGENT_BLOCK"; do sleep 0.05; done
fi
exit "${GROVE_TEST_AGENT_EXIT-0}"
