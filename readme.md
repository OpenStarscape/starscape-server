# OpenStarscape Server

The OpenStarscape game engine and server

Currently in development

Implements [the OpenStarscape protocol](https://github.com/OpenStarscape/Protocol)

To run:
- Clone this repo and `cargo build`
- Clone the [web client](https://github.com/OpenStarscape/starscape-web), `yarn install` and `yarn build`
- Create `starscape.toml` the root directory of the server project with this content:
```
http_port = 56560
http_static_content = "../starscape-web/public"
```
- `cargo run -- --open-browser=true`

See [hacking.md](hacking.md) for architecture and coding guidelines

See [starscape-deploy](https://github.com/OpenStarscape/starscape-deploy) for documentation on deploying to a web server
