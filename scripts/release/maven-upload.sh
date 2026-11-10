#!/bin/sh
# Uploads a Maven Central bundle (scripts/release/maven-bundle.sh) to the
# Central Portal as USER_MANAGED, waits until the Portal has validated it, and
# stops there: the deployment is published by a click in the Portal
# (https://central.sonatype.com/publishing/deployments), which cannot be
# undone and so stays a person's act.
#
#   CENTRAL_API_TOKEN=<user>:<token> scripts/release/maven-upload.sh <bundle.zip> <name>
#
# The token is a Portal user token as `user:password`. The Portal refuses a
# version that is already published, so uploading a bundle again may leave a
# deployment that fails validation (and one that is validated, still waiting
# for the click, is not replaced by the second): do not upload a version twice,
# and leave `maven` out of the registries of the next run.
set -eu

if [ $# -ne 2 ]; then
    echo "usage: $0 <bundle.zip> <deployment name>" >&2
    exit 2
fi
bundle=$1
name=$2
api=https://central.sonatype.com/api/v1/publisher
# A secret pasted with a trailing newline would give a token the Portal refuses.
auth=$(printf '%s' "$CENTRAL_API_TOKEN" | tr -d '\r\n' | base64 | tr -d '\n')

out=$(mktemp)
# Not piped into another command: a pipe reports the last command's status,
# which hid a failed upload as an empty deployment id.
code=$(curl -sS --fail-with-body -X POST -H "Authorization: Bearer $auth" \
    -F "bundle=@$bundle" "$api/upload?publishingType=USER_MANAGED&name=$name" \
    -o "$out" -w '%{http_code}') || {
    echo "maven-upload: the Portal refused the upload (HTTP $code)" >&2
    cat "$out" >&2
    exit 1
}
id=$(tr -d '\r\n' < "$out")
[ -n "$id" ] || { echo "maven-upload: the Portal returned no deployment id (HTTP $code)" >&2; exit 1; }
echo "maven-upload: deployment $id uploaded (HTTP $code)"

# PENDING and VALIDATING end in VALIDATED (USER_MANAGED waits for the click) or
# FAILED. Ten minutes at most.
# The deployment exists in the Portal from here on, so a failed status request
# is tried again, and the id is in every message.
state=
for attempt in $(seq 1 60); do
    if ! curl -sS --fail-with-body -X POST -H "Authorization: Bearer $auth" \
        "$api/status?id=$id" -o "$out"; then
        echo "maven-upload: the status of deployment $id could not be read (attempt $attempt)"
        sleep 10
        continue
    fi
    state=$(sed -n 's/.*"deploymentState" *: *"\([A-Z_]*\)".*/\1/p' "$out")
    echo "maven-upload: $state"
    case "$state" in
        VALIDATED)
            echo "maven-upload: validated; publish deployment $id in the Portal"
            exit 0 ;;
        FAILED)
            cat "$out" >&2
            echo "maven-upload: deployment $id failed validation" >&2
            exit 1 ;;
    esac
    sleep 10
done
echo "maven-upload: deployment $id is "${state:-unknown}" after ten minutes; look at it in the Portal" >&2
exit 1
