# Reference for Deploying a Server

## Directory Layout
__NOTE:__: this is all subject to change, and much of it is completely arbitrary. If you set things up a different way they should and probably will work fine.

The layout I'm using is:
- `~` is `/root` (the service file uses the latter)
- `~/starscape/server`: server git repo
- `~/starscape/server/deploy/starscape.service`: service file, which is symlinked to `/etc/systemd/system/starscape.service`
- `~/starscape/server/`: working directory of the service (relative paths specified in the server are relative to this)
- `~/starscape/server/starscape-server`: server binary run by the service, copied from `~/starscape/server/target/release/starscape-server`
- `~/starscape/web/public`: the web frontend, built for production
- `/root/starscape/server/tls/cert.pem`: TLS certificate
- `/root/starscape/server/tls/privkey.pem`: TLS private key

## Setting up the server
- `mkdir ~/starscape && cd ~/starscape`
- `git clone https://github.com/OpenStarscape/starscape-server.git server`
- `ln -sf ~/starscape/server/deploy/starscape.service /etc/systemd/system/starscape.service`
- `cp ~/starscape/server/deploy/starscape.toml ~/starscape/server/`
- `./starscape-server` check if it works

## Setting up the frontend
- `cd ~/starscape`
- `git clone https://github.com/OpenStarscape/starscape-web.git web`
- `apt install npm && npm install --global yarn`
- `yarn`
- `yarn prod-build`

## Turning it on
- `systemctl enable --now starscape.service`
- `systemctl status starscape.service`

## TLS
- [useful guide](https://shirshak55.github.io/articles/deploying-warp-in-cloud/)

## Creating a Certificate
- NOTE: certbot spins up a webserver on port 80, so the starscape server must be stopped.
- `$ apt install certbot`
- `$ certbot certonly`
- Choose `1: Spin up a temporary webserver (standalone)`
- Agree to ToS
- Enter domain name (ex `starscape.wmww.sh`)
- It will tell you where it put the cert and key
- Create `starscape/tls/` if needed and move them into it
- Example: `ln -s /etc/letsencrypt/live/starscape.wmww.sh/fullchain.pem ~/starscape/tls/cert.pem`
- Example: `ln -s /etc/letsencrypt/live/starscape.wmww.sh/privkey.pem ~/starscape/tls/privkey.pem`

## Renew Certificate
- Because we roll our own webserver, certbot can't integrate with us
- Instead we had certbot run a standalone HTTP server, but we had to stop the Starscape server for that
- Autorenewal fails for the same reason while the Starscape server is running
- When the cert expires, log into the server, and:
- `$ systemctl stop starscape.service`
- `$ certbot renew`
- `$ systemctl start starscape.service`
- TODO: better solution, possibly using the [webroot](https://certbot.eff.org/docs/using.html#webroot) plugin?

## To update server
- `cd ~/starscape/server`
- `git pull`
- `cargo build --release`
- `systemctl stop starscape.service`
- Run `target/release/starscape-server` if you want
- `cp target/release/starscape-server .`
- `systemctl start starscape.service`

## To update frontend
- `cd ~/starscape/web`
- `git pull`
- `yarn`
- `yarn prod-build`
- `systemctl restart starscape.service`
