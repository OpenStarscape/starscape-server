# Reference for Deploying a Server

## Directory Layout
NOTE: this is all subject to change and may become outdated.

The layout I'm using is:
- `/root/starscape/devel`: server git repo
- `/root/starscape/devel/deploy/starscape.service`: service file, which is symlinked to `/etc/systemd/system/starscape.service`
- `/root/starscape/server/`: working directory of the service (relative paths specified in the server are relative to this)
- `/root/starscape/server/target/release/starscape-server`: server binary run by the service, copied from devel
- `/root/starscape/server/web/dist/`: the web frontend built for production (I haven't been able to build it on the server, so I'm building locally and transferring the tarball)
- `/root/starscape/server/ssl/cert.pem`: SSL certificate
- `/root/starscape/server/ssl/privkey.pem`: SSL private key

## SSL
- [useful guide](https://shirshak55.github.io/articles/deploying-warp-in-cloud/https://shirshak55.github.io/articles/deploying-warp-in-cloud/)

## Creating a Certificate
- NOTE: certbot spins up a webserver on port 80, so the starscape server must be stopped.
- `$ apt install certbot`
- `$ certbot certonly`
- Choose `1: Spin up a temporary webserver (standalone)`
- Agree to ToS
- Enter domain name (ex `starscape.wmww.sh`)
- It will tell you where it put the cert and key
- Create `starscape/ssl/` if needed and move them into it
- Example: `cp /etc/letsencrypt/live/starscape.wmww.sh/fullchain.pem starscape/ssl/cert.pem`
- Example: `cp /etc/letsencrypt/live/starscape.wmww.sh/privkey.pem starscape/ssl/privkey.pem`