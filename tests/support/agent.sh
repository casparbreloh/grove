#!/bin/sh

printf '%s\n' "$$" >> "$GROVE_TEST_AGENT_PID"
printf 'cwd=%s\ndirective=%s\n' "$PWD" "${GROVE_DIRECTIVE_CD_FILE-absent}" >> "$GROVE_TEST_AGENT_LOG"
for argument do
  printf 'arg=<%s>\n' "$argument" >> "$GROVE_TEST_AGENT_LOG"
done

session_file=
session_id=
fake_codex=false
new_pi_session=false
while test "$#" -gt 0; do
  if test "$1" = --session; then shift; session_file=$1; fi
  if test "$1" = --session-id; then shift; session_id=$1; fi
  if test "$1" = --fake-codex; then fake_codex=true; fi
  if test "$1" = --new-pi-session; then new_pi_session=true; fi
  shift
done

printf 'grove-test-agent-ready\n'
if test -n "$session_file"; then
  IFS= read -r prompt
  if test "$new_pi_session" = true; then
    transcript="${session_file%/*}/new-session.jsonl"
    printf '{"type":"session","cwd":"%s"}\n' "$PWD" > "$transcript"
    printf '{"type":"message","message":{"role":"user","content":[{"type":"text","text":"%s"}]}}\n' "$prompt" >> "$transcript"
    exit 0
  fi
  printf '{"type":"message","message":{"role":"user","content":"%s"}}\n' "$prompt" >> "$session_file"
elif test -n "$session_id"; then
  IFS= read -r prompt
  transcript="$HOME/.claude/projects/test/$session_id.jsonl"
  mkdir -p "${transcript%/*}"
  printf '{"type":"user","origin":{"kind":"human"},"promptSource":"typed","message":{"role":"user","content":"%s"}}\n' "$prompt" >> "$transcript"
elif test "$fake_codex" = true; then
  IFS= read -r prompt
  transcript="$HOME/.codex/sessions/2026/07/15/rollout-test.jsonl"
  mkdir -p "${transcript%/*}"
  printf '{"type":"session_meta","payload":{"cwd":"%s"}}\n' "$PWD" >> "$transcript"
  printf '{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"%s"}]}}\n' "$prompt" >> "$transcript"
  printf '{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"later prompt must not rename"}]}}\n' >> "$transcript"
fi

sleep 30
