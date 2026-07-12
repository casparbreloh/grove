grove() {
  case "$1" in
    switch)
      local target
      target="$(command grove "$@")" || return
      builtin cd -- "$target"
      ;;
    remove)
      local target
      target="$(command grove "$@")" || return
      if [[ -n "$target" ]]; then
        builtin cd -- "$target"
      fi
      ;;
    *) command grove "$@" ;;
  esac
}
