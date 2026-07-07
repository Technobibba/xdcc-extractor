#!/usr/bin/env bash
set -euo pipefail

echo "== XDCC Extractor Publication Check =="

echo
echo "== Git Status =="
git status --short

echo
echo "== Sensible Dateien im Git-Index =="
sensitive_files="$(
  git ls-files | grep -Ei '(^|/)(\.env|config\.toml|config\.docker\.toml|config\.env|passwords\.txt)$|(^|/)(state|logs|target)/' || true
)"

if [ -n "$sensitive_files" ]; then
  echo "FEHLER: Sensible Runtime-Dateien sind getrackt:"
  echo "$sensitive_files"
  exit 1
fi

echo "OK: Keine sensiblen Runtime-Dateien getrackt."

echo
echo "== Secret-/URL-Scan in getrackten Dateien =="

python3 <<'PY'
from pathlib import Path
import re
import subprocess
import sys

tracked = subprocess.check_output(["git", "ls-files"], text=True).splitlines()

# Dieses Script enthält selbst die Suchmuster und würde sich sonst selbst melden.
skip_files = {
    "scripts/publication-check.sh",
}

allow_files = {
    ".env.example",
    "config.docker.example.toml",
    "config.example.toml",
    "passwords.example.txt",
}

patterns = [
    ("private duckdns URL", re.compile(r"duckdns\.org", re.IGNORECASE)),
    ("private host name", re.compile(r"technobibba", re.IGNORECASE)),
    ("non-example auth password", re.compile(r"XDCC_WEB_AUTH_PASSWORD=(?!change-me\s*$).+", re.IGNORECASE)),
    ("possible token assignment", re.compile(r'(?im)^\s*(token|secret|password)\s*=\s*"[A-Za-z0-9_.:/+=-]{8,}"')),
    ("possible bearer token", re.compile(r"Bearer\s+[A-Za-z0-9_.:/+=-]{8,}", re.IGNORECASE)),
]

hits = []

for file in tracked:
    if file in skip_files:
        continue

    path = Path(file)

    if not path.is_file():
        continue

    try:
        text = path.read_text(errors="ignore")
    except Exception:
        continue

    for lineno, line in enumerate(text.splitlines(), start=1):
        for label, pattern in patterns:
            if file in allow_files and label in {"non-example auth password", "possible token assignment"}:
                continue

            if pattern.search(line):
                redacted = re.sub(r"https?://\S+", "<URL_REDACTED>", line)
                redacted = re.sub(r'(?i)(token\s*=\s*)".*"', r'\1"<REDACTED>"', redacted)
                redacted = re.sub(r'(?i)(password\s*=\s*)".*"', r'\1"<REDACTED>"', redacted)
                redacted = re.sub(r'(?i)(XDCC_WEB_AUTH_PASSWORD=).*', r'\1<REDACTED>', redacted)
                hits.append(f"{file}:{lineno}: {label}: {redacted}")
                break

if hits:
    print("FEHLER: Mögliche private Daten gefunden:")
    print("\n".join(hits))
    sys.exit(1)

print("OK: Keine offensichtlichen Secrets oder privaten URLs gefunden.")
PY

echo
echo "== Dockerignore vorhanden =="
test -f .dockerignore
echo "OK: .dockerignore vorhanden."

echo
echo "== CI Workflow vorhanden =="
test -f .github/workflows/ci.yml
echo "OK: CI Workflow vorhanden."

echo
echo "== Rust Checks =="
cargo fmt --check
cargo test
cargo build --locked

echo
echo "OK: Publication Check erfolgreich."
