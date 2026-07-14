grove() {
  local directive
  directive="$(mktemp)" || return
  GROVE_DIRECTIVE_CD_FILE="$directive" command grove "$@"
  local command_status=$?
  if [[ -s "$directive" ]]; then
    builtin cd -- "$(<"$directive")"
  fi
  rm -f -- "$directive"
  return $command_status
}
