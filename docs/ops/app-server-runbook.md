# App-server Runbook (code-app-server)

This runbook covers operating `code-app-server` as a supervised stdio JSON-RPC
service for local clients. It assumes trusted local IPC (Unix sockets) and a
single writer per `CODE_HOME`.

## Baseline
- Keep `CODE_HOME` on persistent storage.
- Set `CODE_SECURE_MODE=1` and `RUST_LOG=info` (or your preferred filter).
- `docs/ops/production.env.example` includes a baseline env template.
- Do not expose `code-app-server` on a network socket without authn/z.
- Run one `code-app-server` process per `CODE_HOME` at a time. If you need
  parallel instances, use distinct `CODE_HOME` paths.

## Systemd (Unix socket via socat)
This is the simplest supervised deployment. It listens on a Unix socket and
spawns a new `code-app-server` process per connection.

1. Install `socat` on the host.
2. Create a system user and data directory, for example:
   - user/group: `code`
   - data dir: `/var/lib/code`

Example unit: `/etc/systemd/system/code-app-server.service`

```ini
[Unit]
Description=Beacon code-app-server (socat wrapper)
After=network.target

[Service]
Type=simple
User=code
Group=code
Environment=CODE_HOME=/var/lib/code
Environment=CODE_SECURE_MODE=1
Environment=RUST_LOG=info
ExecStart=/usr/bin/socat -d -d UNIX-LISTEN=/run/code-app-server/app-server.sock,mode=0660,fork EXEC:/usr/local/bin/code-app-server,stderr
RuntimeDirectory=code-app-server
RuntimeDirectoryMode=0750
UMask=0077
Restart=on-failure
RestartSec=2
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
```

Notes:
- Remove `fork` if you want to allow only one client at a time.
- If you keep `fork`, ensure only one client uses a given `CODE_HOME`, or set
  a distinct `CODE_HOME` per client.
- Adjust the socket path and permissions to fit your access control model.

Enable the service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now code-app-server.service
```

## Systemd socket activation (alternative)
Use this if you prefer systemd-managed sockets instead of `socat`. Socket
activation requires `Accept=yes` so systemd passes a connected socket to the
service as stdio.

`/etc/systemd/system/code-app-server.socket`:

```ini
[Socket]
ListenStream=/run/code-app-server/app-server.sock
SocketMode=0660
SocketUser=code
SocketGroup=code
Accept=yes

[Install]
WantedBy=sockets.target
```

`/etc/systemd/system/code-app-server@.service`:

```ini
[Unit]
Description=Beacon code-app-server (per connection)

[Service]
StandardInput=socket
StandardOutput=socket
StandardError=journal
User=code
Group=code
Environment=CODE_HOME=/var/lib/code
Environment=CODE_SECURE_MODE=1
Environment=RUST_LOG=info
ExecStart=/usr/local/bin/code-app-server
UMask=0077
```

Enable the socket unit:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now code-app-server.socket
```

## Health check
Send an initialize handshake to verify the server responds. Example for a Unix
socket:

```bash
printf '{"id":0,"method":"initialize","params":{"clientInfo":{"name":"healthcheck","title":"Healthcheck","version":"0.0.0"}}}\n{"method":"initialized","params":{}}\n' \
  | socat - UNIX-CONNECT:/run/code-app-server/app-server.sock \
  | head -n 1
```

Optional automation:
```
scripts/ops/healthcheck-app-server.sh /run/code-app-server/app-server.sock --timeout 5
```

## Observability
- `journalctl -u code-app-server.service` for logs (or the socket unit name).
- App logs on disk: `$CODE_HOME/log/` and `$CODE_HOME/logs/`.
- For telemetry, configure OTEL in `config.toml` (see `docs/production_checklist.md`).

## Backup and rollback
- Back up `CODE_HOME` using `scripts/code-home-backup.sh` (see
  `docs/production_checklist.md`).
- Roll back by swapping the binary and restarting the service; restore the last
  known good `CODE_HOME` if needed.

## Common failures
- `"Not initialized"` errors: the client did not send `initialize` + `initialized`.
- `"Already initialized"` errors: the client attempted to initialize twice.
