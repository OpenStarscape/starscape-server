## Documentation
Docs are important, and always encouraged. Write inline documentation with the standard [Rustdoc](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate).

## Warnings
Code should pass `cargo clippy` without warnings. If a warning is not useful, allow it with `#[allow(clippy::...)]`.

## Formatting
Code should be formatted with `cargo fmt`.

Prefer
```
use module::{
	Name,
	Other,
};
```
style imports over
```
use module::Name;
use module::Other;
```
(`cargo fmt` will not change this for you)

## Logging
We use the [log](https://docs.rs/log) crate for logging messages and [env_logger](https://docs.rs/env_logger) to print them. By default `error`, `warn` and `info` messages enabled and `debug` and `trace` messages are not, but this can be changed with the `RUST_LOG` environment variable (see env_logger docs for details). This is what the levels generally mean:
- `error!()`: an non-fatal server error that should be reported as a bug
- `warn!()`: a client bug, connection issue or other problem that may not be the server's fault
- `info!()`: information on server status and operation that may be relevant to whoever's running the server
- `debug!()`: generally only used in development
- `trace!()`: any gory details that may sometimes be useful but shouldn't normally clutter up the logs

### Capitalization
Log messages and panic messages should not be given initial capitalization like a sentence.

## Imports
Most files should `use super::*`, which will pull in all public names in the project, and the library and standard library names we commonly use (see [src/main.rs](src/main.rs)). Library and standard library names we use only a few places should be `use`d in those files only. Modules should only `pub use` what is needed outside of their module. All names should be unique within the scope of where they're used (that is two sibling modules can have private structs with conflicting names, but if either is public then one or both of the names should be changed).
