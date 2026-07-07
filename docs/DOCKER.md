# Docker Betrieb

## Start

~~~bash
cd /opt/xdcc-extractor
docker compose build
docker compose up -d
docker compose logs --tail=100
~~~

## WebUI

~~~text
http://<docker-host>:8099
~~~

Die WebUI ist per Basic Auth geschützt. Zugangsdaten werden über `.env` gesetzt.

## Healthcheck

Docker prüft:

~~~text
http://127.0.0.1:8099/health
~~~

Status prüfen:

~~~bash
docker inspect --format='{{.State.Health.Status}}' xdcc-extractor
~~~

## Neustart

~~~bash
docker compose restart
~~~

Oder über die WebUI:

~~~text
/settings/edit
~~~

## Config

Lokale produktive Config:

~~~text
config.docker.toml
~~~

Beispielconfig:

~~~text
config.docker.example.toml
~~~

Die produktive Config kann Secrets enthalten und darf nicht committed werden.

## Backup

Bei Änderungen über die WebUI wird ein Backup erstellt:

~~~text
state/config-backups/
~~~

## Logs

Docker Logs:

~~~bash
docker compose logs --tail=100
~~~

WebUI Logs:

~~~text
/logs
/api/logs
~~~
