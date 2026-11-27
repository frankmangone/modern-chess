#!/bin/sh
# Prints the environment for the sign-in provider that `make dev` should use, for `eval`:
#
#   fake   (the default) a fake provider run by the web dev server, so sign-in works with no
#          outside account. Anyone can sign in as any email, on this machine only.
#   clerk  the real Clerk instance, from `.env.auth` at the repository root with the two lines
#          CLERK_PUBLISHABLE_KEY=pk_test_... and CLERK_ISSUER=https://<name>.clerk.accounts.dev
#          (neither value is a secret).
#   none   no sign-in: everyone plays as a guest.
#
# Both the social API (which checks the provider's tokens) and the web app (which asks the
# provider for them) must agree, so this is the one place that sets both.
set -eu

mode="${1:-fake}"
root="$(cd "$(dirname "$0")/.." && pwd)"

case "$mode" in
fake)
  echo "export VITE_AUTH_PROVIDER=fake"
  echo "export SOCIAL_PROVIDER_NAME=fake"
  echo "export SOCIAL_PROVIDER_ISSUER=http://localhost:3050/dev-idp"
  ;;
clerk)
  file="$root/.env.auth"
  if [ ! -f "$file" ]; then
    echo "make dev AUTH=clerk needs $file with CLERK_PUBLISHABLE_KEY and CLERK_ISSUER." >&2
    exit 1
  fi
  # Read as text, never run: a line is KEY=value and nothing else.
  key=$(sed -n 's/^CLERK_PUBLISHABLE_KEY=//p' "$file" | head -n 1)
  issuer=$(sed -n 's/^CLERK_ISSUER=//p' "$file" | head -n 1)
  for value in "$key" "$issuer"; do
    case "$value" in
    "" | *[!A-Za-z0-9_:/.=-]*)
      echo "$file: CLERK_PUBLISHABLE_KEY and CLERK_ISSUER must be set, with no spaces or quotes." >&2
      exit 1
      ;;
    esac
  done
  case "$key" in
  pk_*) ;;
  *)
    echo "$file: CLERK_PUBLISHABLE_KEY should start with pk_ (never use a sk_ secret key)." >&2
    exit 1
    ;;
  esac
  case "$issuer" in
  https://*) ;;
  *)
    echo "$file: CLERK_ISSUER must be an https address." >&2
    exit 1
    ;;
  esac
  echo "export VITE_AUTH_PROVIDER=clerk"
  echo "export VITE_CLERK_PUBLISHABLE_KEY='$key'"
  echo "export SOCIAL_PROVIDER_NAME=clerk"
  echo "export SOCIAL_PROVIDER_ISSUER='$issuer'"
  ;;
none)
  echo "unset VITE_AUTH_PROVIDER SOCIAL_PROVIDER_NAME SOCIAL_PROVIDER_ISSUER"
  ;;
*)
  echo "AUTH must be fake, clerk or none (got: $mode)." >&2
  exit 1
  ;;
esac
