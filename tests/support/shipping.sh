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
  *" repos/"*"/pulls"*)
    case " $* " in
      *" --method POST "*)
        payload=$(cat)
        printf 'payload=%s\n' "$payload" >> "$GROVE_TEST_SHIPPING_LOG"
        if test "${GROVE_TEST_CREATE_EXIT-0}" -ne 0; then
          echo "injected create failure" >&2
          exit "$GROVE_TEST_CREATE_EXIT"
        fi
        printf '{"number":1,"html_url":"%s","title":"%s","body":"%s","base":{"ref":"%s"}}\n' \
          "${GROVE_TEST_RESULT_URL-https://github.com/example/repo/pull/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}" \
          "${GROVE_TEST_RESULT_TARGET-${GROVE_TEST_REVIEW_TARGET-main}}"
        ;;
      *" --method PATCH "*)
        payload=$(cat)
        printf 'payload=%s\n' "$payload" >> "$GROVE_TEST_SHIPPING_LOG"
        if test "${GROVE_TEST_UPDATE_EXIT-0}" -ne 0; then
          echo "injected update failure" >&2
          exit "$GROVE_TEST_UPDATE_EXIT"
        fi
        printf '{"number":1,"html_url":"%s","title":"%s","body":"%s","base":{"ref":"%s"}}\n' \
          "${GROVE_TEST_RESULT_URL-https://github.com/example/repo/pull/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}" \
          "${GROVE_TEST_RESULT_TARGET-${GROVE_TEST_REVIEW_TARGET-main}}"
        ;;
      *)
        if test -n "${GROVE_TEST_REVIEW_URL-}"; then
          target=${GROVE_TEST_REVIEW_TARGET-main}
          requests=$(grep -c 'program=gh args=.*repos/.*/pulls' "$GROVE_TEST_SHIPPING_LOG")
          if test "$requests" -gt 1 && test -n "${GROVE_TEST_RECHECK_TARGET-}"; then
            target=$GROVE_TEST_RECHECK_TARGET
          fi
          printf '[{"number":1,"html_url":"%s","title":"%s","body":"%s","base":{"ref":"%s"}}]\n' \
            "$GROVE_TEST_REVIEW_URL" "${GROVE_TEST_REVIEW_TITLE-feat: existing}" \
            "${GROVE_TEST_REVIEW_BODY-Existing description.}" "$target"
        else
          printf '[]\n'
        fi
        ;;
    esac
    ;;
  *" repos/"*) printf '{"default_branch":"%s"}\n' "${GROVE_TEST_DEFAULT_BRANCH-main}" ;;
  *" projects/"*"/merge_requests"*)
    case " $* " in
      *" --method POST "*)
        payload=$(cat)
        printf 'payload=%s\n' "$payload" >> "$GROVE_TEST_SHIPPING_LOG"
        if test "${GROVE_TEST_CREATE_EXIT-0}" -ne 0; then
          echo "injected create failure" >&2
          exit "$GROVE_TEST_CREATE_EXIT"
        fi
        printf '{"iid":1,"web_url":"%s","title":"%s","description":"%s","source_project_id":1,"target_branch":"%s"}\n' \
          "${GROVE_TEST_RESULT_URL-https://gitlab.com/example/repo/-/merge_requests/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}" \
          "${GROVE_TEST_RESULT_TARGET-${GROVE_TEST_REVIEW_TARGET-main}}"
        ;;
      *" --method PUT "*)
        payload=$(cat)
        printf 'payload=%s\n' "$payload" >> "$GROVE_TEST_SHIPPING_LOG"
        if test "${GROVE_TEST_UPDATE_EXIT-0}" -ne 0; then
          echo "injected update failure" >&2
          exit "$GROVE_TEST_UPDATE_EXIT"
        fi
        printf '{"iid":1,"web_url":"%s","title":"%s","description":"%s","source_project_id":1,"target_branch":"%s"}\n' \
          "${GROVE_TEST_RESULT_URL-https://gitlab.com/example/repo/-/merge_requests/1}" \
          "${GROVE_TEST_RESULT_TITLE-feat: ship change}" \
          "${GROVE_TEST_RESULT_BODY-Ships the Change.}" \
          "${GROVE_TEST_RESULT_TARGET-${GROVE_TEST_REVIEW_TARGET-main}}"
        ;;
      *) printf '[]\n' ;;
    esac
    ;;
  *" projects/"*) printf '{"id":1,"default_branch":"%s"}\n' "${GROVE_TEST_DEFAULT_BRANCH-main}" ;;
esac
