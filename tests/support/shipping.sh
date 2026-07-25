#!/bin/sh

case "$(basename "$0")" in
  ssh)
    case "$*" in
      *git-upload-pack*) exec git-upload-pack "$GROVE_TEST_REMOTE_PATH" ;;
      *git-receive-pack*) exec git-receive-pack "$GROVE_TEST_REMOTE_PATH" ;;
      *) echo "unsupported fake ssh invocation: $*" >&2; exit 1 ;;
    esac
    ;;
  gh|glab)
    printf 'program=%s args=%s\n' "$(basename "$0")" "$*" >> "$GROVE_TEST_SHIPPING_LOG"
    ;;
  *)
    echo "unsupported fake shipping program: $(basename "$0")" >&2
    exit 1
    ;;
esac

case " $* " in
  *" --version "*|*" auth status "*) exit 0 ;;
esac

case " $* " in
  *" api "*) ;;
  *) exit 0 ;;
esac

case " $* " in
  *" repos/"*"/pulls "*)
    case " $* " in
      *" --method POST "*|*" --method PATCH "*)
        cat >/dev/null
        printf '{"number":1,"html_url":"%s","title":"%s","body":"%s"}\n' \
          "${GROVE_TEST_RESULT_URL-https://github.com/example/repo/pull/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}"
        ;;
      *)
        if test -n "${GROVE_TEST_REVIEW_URL-}"; then
          printf '[{"number":1,"html_url":"%s","title":"%s","body":"%s"}]\n' \
            "$GROVE_TEST_REVIEW_URL" "${GROVE_TEST_REVIEW_TITLE-feat: existing}" \
            "${GROVE_TEST_REVIEW_BODY-Existing description.}"
        else
          printf '[]\n'
        fi
        ;;
    esac
    ;;
  *" repos/"*) printf '{"default_branch":"main"}\n' ;;
  *" projects/"*"/merge_requests"*)
    case " $* " in
      *" --method POST "*|*" --method PUT "*)
        cat >/dev/null
        printf '{"iid":1,"web_url":"%s","title":"%s","description":"%s","source_project_id":1}\n' \
          "${GROVE_TEST_RESULT_URL-https://gitlab.com/example/repo/-/merge_requests/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}"
        ;;
      *) printf '[]\n' ;;
    esac
    ;;
  *" projects/"*) printf '{"id":1,"default_branch":"main"}\n' ;;
esac
