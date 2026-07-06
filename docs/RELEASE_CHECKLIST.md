# Release-Checkliste

Diese Checkliste wird vor jedem neuen Release oder größeren Update genutzt.

---

## 1. Arbeitsstand prüfen

```bash
git status --short
git log --oneline --decorate -8

Es sollten keine unerwarteten lokalen Änderungen offen sein.

Secrets dürfen nie committed werden:

git check-ignore -v config.docker.toml
git check-ignore -v config/passwords.txt
2. Rust prüfen
cargo fmt
cargo test
cargo build

Alle Tests müssen grün sein.

3. Version prüfen
./target/debug/xdcc-extractor --version
./target/debug/xdcc-extractor --help
./target/debug/xdcc-extractor --status --config config.docker.toml
4. Docker neu bauen
docker compose down
docker compose build
docker compose up -d
5. Container prüfen
docker ps --format "table {{.Names}}\t{{.Status}}"
docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --version
docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --status
docker compose logs --tail=100

Erwartung:

xdcc-extractor   Up ... (healthy)
6. Gotify prüfen

Eine Testmeldung senden:

curl -sS -X POST "https://gotify.technobibba.duckdns.org/message?token=DEIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"XDCC Release Test","message":"Gotify funktioniert nach Release.","priority":5}'

Token nicht committen und nicht in Logs posten.

7. Testarchiv prüfen
rm -rf /tmp/xdcc-release-test
mkdir -p /tmp/xdcc-release-test

echo "XDCC Release Test" > /tmp/xdcc-release-test/test.txt

rm -f /media/HDD3/XDCC/Release.Check.Test.1.zip
rm -rf /media/HDD3/XDCC/_extracted/Release.Check.Test.1

7z a /media/HDD3/XDCC/Release.Check.Test.1.zip /tmp/xdcc-release-test/test.txt

rm -rf /tmp/xdcc-release-test

Logs prüfen:

docker compose logs -f

Erwartung:

Archivprüfung erfolgreich
Entpackung abgeschlossen
Entpackung validiert

Bei dry_run=true darf das Archiv nicht gelöscht werden.

8. Status nach Test prüfen
docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --status
9. Commit und Tag
git status --short
git add .
git restore --staged config.docker.toml 2>/dev/null || true
git restore --staged config/passwords.txt 2>/dev/null || true
git commit -m "Release preparation"

Tag setzen, Beispiel:

git tag v0.4.1
git log --oneline --decorate -8
git tag
10. Rollback-Hinweis

Letzten Stand anzeigen:

git log --oneline --decorate -10

Auf einen älteren Stand zurückgehen:

git checkout <commit-oder-tag>
docker compose down
docker compose build
docker compose up -d

Danach wieder Status prüfen:

docker exec -it xdcc-extractor /usr/local/bin/xdcc-extractor --status
docker compose logs --tail=100

