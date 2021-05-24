# Reference for Deploying a Server

## Directory Layout
__NOTE:__: this is all subject to change, and much of it is completely arbitrary. If you set things up a different way they should and probably will work fine.

The layout I'm using is:
- `~` is `/root` (the service file uses the latter)
- `~/starscape/server`: server git repo
- `~/starscape/server/deploy/starscape.service`: service file, which is symlinked to `/etc/systemd/system/starscape.service`
- `~/starscape/server/`: working directory of the service (relative paths specified in the server are relative to this)
- `~/starscape/server/starscape-server`: server binary run by the service, copied from `~/starscape/server/target/release/starscape-server`
- `~/starscape/server/web-build`: the web frontend built for production (I haven't been able to build it on the server, so I'm building locally and transferring the tarball)
- `/root/starscape/server/ssl/cert.pem`: SSL certificate
- `/root/starscape/server/ssl/privkey.pem`: SSL private key

## Setting up the server
- `mkdir ~/starscape && cd ~/starscape`
- `git clone https://github.com/OpenStarscape/starscape-server.git server`
- `ln -sf ~/starscape/server/deploy/starscape.service /etc/systemd/system/starscape.service`
- `cp ~/starscape/server/deploy/starscape.toml ~/starscape/server/`
- `vim ~/starscape/server/starscape.toml`:
```
tcp = true
http_content = "../web-build"
```

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
- Create `starscape/ssl/` if needed and move them into it
- Example: `ln -s /etc/letsencrypt/live/starscape.wmww.sh/fullchain.pem ~/starscape/ssl/cert.pem`
- Example: `ln -s /etc/letsencrypt/live/starscape.wmww.sh/privkey.pem ~/starscape/ssl/privkey.pem`

## To update server
- `cd ~/starscape/server`
- `git pull`
- `cargo build --release`
- `systemctl stop starscape.service`
- Run `target/release/starscape-server` if you want
- `cp target/release/starscape-server .`
- `systemctl start starscape.service`

## To update frontend
- __on local machine__ (because server doesn't have enough memory to run babel 🙃)
- `cd starscape/web`
- `npm run build`
- `tar -czvf starscape-web.tar.gz ./build`
- `scp ./starscape-web.tar.gz root@0.0.0.0:/root/starscape` # with the real server IP substituted in
- __on server__
- `tar -xvf ~/starscape/starscape-web.tar.gz`
- `rm -Rf ~/starscape/web-build`
- `mv ./build/ ~/starscape/web-build`
