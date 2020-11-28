# OpenStarscape server hacking guide
Here you'll find everything you need to know to get hacking on the OpenStarscape game engine/server

Tip: `cargo doc --open` will get you nice looking docs in a browser

## Structure
The project contains the following modules:
- `game`: components (like ships), systems (like physics) and logix to support actual gameplay
- `engine`: the core engine, which is basically an ECS fused with a reactive property system (see the next section for details)
- `connection`: the high level logic for talking to clients, such as encoding, decoding and dispatching messages
- `server`: lower level network code including the session implementation for WebRTC and other network protocols
- `helpers`: general helpers that may be useful anywhere

## Enhanced ECS
We use a custom ECS (entity component system) built on top of `slotmap` and `anymap`. Most of the interface is found on the `State` object. Due to the needs of the project, we have a reactive property system tightly integrated with the ECS. This allows, for example, a mutation of a value in the state to result in updates to multiple properties to be efficiently sent to multiple clients. Lets take a closer look at how this is put together:

### State
The `State` owns all entities and components. Most code that uses the state will be passed a reference to it. Entities and components inherit the state's mutability (if a function is passed an immutable state, it can not change anything).

### Entity
An entity is a collection of components. It's referred to by an `EntityKey`. Not every entity is exposed to every client, but every client-facing object maps to exactly one entity. Zero or one components of each component type may be attached to an entity.

### Object
An object (represented on the server by an `ObjectID`) is a client's view onto an entity. different clients may know the same entity by different object IDs, and be given a different set of properties or property values.

### Component
A component is a strongly typed piece of state attached to an entity. It's generally a struct that contains elements. It usually represents something about an entity. For example entities that are attached to a `Body` component, have a size and position in 3D space, entities that don't represent non-physical concepts.

### Element
`Element<T>` is an atomic unit of state. Components are generally made out of elements. An element can be subscribed to, in which case it will notify the subscriber when it is changed. These notifications are __not__ dispatched immediately. Instead, they are queued and processed later in the main game loop.

### Property
A property is a client's view onto some piece of state. It has a name and is attached to an object. A client can get, set, subscribe and unsubscribe from a property. A property *often* maps 1:1 with an element, but may calculate it's value based on an element, or based on multiple elements. It might even be calculated from information from multiple entities.

### Conduit
Conduits are what connect properties to elements. The way they are constructed and used by the game module hasn't been fully fleshed out yet.

## Code Style
### Documentation
Docs are important, and always encouraged. Write inline documentation with the standard [Rustdoc](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate). Ideally comments are wrapped at col 100 (like `cargo fmt` formats the code), but really whatever.

### Logging
We use the [log](https://docs.rs/log) crate for logging messages and [env_logger](https://docs.rs/env_logger) to print them. By default `error`, `warn` and `info` messages enabled and `debug` and `trace` messages are not, but this can be changed with the `RUST_LOG` environment variable (see env_logger docs for details). This is what the levels generally mean:
- `error!()`: an non-fatal problem that should be considered a bug in the server
- `warn!()`: a client bug, connection issue or other problem that may not be the server's fault
- `info!()`: information on server status and operation that may be relevant to whoever's running the server
- `debug!()`: generally only used in development
- `trace!()`: any gory details that may sometimes be useful but shouldn't normally clutter up the logs

### Error handling
The server going down is rather catastrophic for the game in progress, so we only want to panic on errors that are very unlikely to be intermittent or truly unrecoverable. If disconnecting a single connection or destroying an entity would solve the problem, that should be preferred. If there's a recoverable internal error, we log an `error!()` and otherwise ignore it.

In theory returning result is for expected errors, and should be properly handled. In practice I just slap `?` and `Result<_, Box<dyn Error>>` everywhere, and let problems bubble up to the top before logging them. At some point we will sort this out.

### Capitalization
Log messages and panic messages should start with a lower-case letter unless the word would be otherwise capitalized.

## Imports
Most files should `use super::*`, which will pull in all public names in the project, and the library and standard library names we commonly use (see [src/main.rs](src/main.rs)). Library and standard library names we use only a few places should be `use`d in those files only. Modules should only `pub use` what is needed outside of their module. All names should be unique within the scope of where they're used (that is two sibling modules can have private structs with conflicting names, but if either is public then one or both of the names should be changed).

### Clippy
Code should pass `cargo clippy` without warnings. If a warning is not useful, allow it with `#[allow(clippy::...)]`.

## Traits
We use a lot of boxed traits, even when there is only one "real" implementation. This makes it easier to write unit tests (you use a mock implementation of the trait, instead of having to use the real object) and increases modularity? Maybe? idk.

### Formatting
Code should be formatted with `cargo fmt` on it's default settings. The following are preferences that are not changed by `cargo fmt`.

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