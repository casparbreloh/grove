#!/bin/sh

case "$*" in
  *git-upload-pack*) exec git-upload-pack "$GROVE_TEST_REMOTE_PATH" ;;
  *git-receive-pack*) exec git-receive-pack "$GROVE_TEST_REMOTE_PATH" ;;
  *) echo "unsupported fake ssh invocation: $*" >&2; exit 1 ;;
esac
