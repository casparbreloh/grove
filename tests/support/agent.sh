#!/bin/sh

mode=interactive
schema=
previous=
for argument do
  if test "$previous" = --mode; then mode=$argument; fi
  if test "$previous" = --structured-output-schema; then schema=$argument; fi
  previous=$argument
done

printf 'mode=%s\ncwd=%s\ndirective=%s\n' "$mode" "$PWD" "${GROVE_DIRECTIVE_CD_FILE-absent}" >> "$GROVE_TEST_AGENT_LOG"
for argument do
  printf 'arg=<%s>\n' "$argument" >> "$GROVE_TEST_AGENT_LOG"
done

if test "$mode" = rpc; then
  while IFS= read -r request; do
    printf 'rpc=%s\n' "$request" >> "$GROVE_TEST_AGENT_LOG"
    if test -n "${GROVE_TEST_RPC_BLOCK-}"; then
      while test -e "$GROVE_TEST_RPC_BLOCK"; do sleep 0.05; done
    fi
    if test "${GROVE_TEST_TITLE_EXIT-0}" -ne 0; then
      exit "$GROVE_TEST_TITLE_EXIT"
    fi
    printf '{"type":"response","command":"prompt","success":true}\n'
    case "$schema" in
      *'"change"'*)
        if test -n "${GROVE_TEST_TITLE-}"; then
          printf '{"type":"tool_execution_end","toolName":"structured_output","result":{"content":[],"details":{"change":"%s"}},"isError":false}\n' "$GROVE_TEST_TITLE"
        fi
        ;;
      *)
        if test -n "${GROVE_TEST_SHIP_OUTPUT-}"; then
          printf '{"type":"tool_execution_end","toolName":"structured_output","result":{"content":[],"details":%s},"isError":false}\n' "$GROVE_TEST_SHIP_OUTPUT"
        fi
        ;;
    esac
    printf '{"type":"agent_settled"}\n'
  done
  exit 0
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
    if ! grep -q '"customType":"grove.change"' "$session_file"; then
      printf '{"type":"custom","id":"grove-link","parentId":null,"timestamp":"2026-01-01T00:00:00.001Z","customType":"grove.change","data":{"changeId":"%s"}}\n' "$GROVE_CHANGE_ID" >> "$session_file"
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
