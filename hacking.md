# OpenStarscape server hacking guide
Here you'll find everything you need to know to get hacking on the OpenStarscape game engine/server

## Structure
The project contains the following modules:
- `components`: the building blocks of the game, such as ships and bodies
- `systems`: contains functions that perform game logic to the state (such as physics)
- `engine`: the core engine, which is basically an ECS fused with a reactive property system (see the next section for details)
- `server`: all the code related to talking to clients including listening for connections, network protocols and serialization
- `util`: general helpers that may be useful anywhere

## Enhanced ECS
We use a custom ECS (entity component system) built on top of `slotmap` and `anymap`. Most of the interface is found on the `State` object. Due to the needs of the project, a reactive property system tightly integrated with the ECS. This allows, for example, mutating a value on a component to result in updates to multiple properties to be efficiently sent to multiple clients. Lets take a closer look at the peaces:

### State
The `State` owns all entities and components. Most code that uses the state will be passed a reference to it. Entities and components inherit the state's mutability (if a function is passed an immutable state, it can not change anything).

### Entity
An entity is a collection of components. It's referred to by an `EntityKey`. Not every entity is exposed to every client, but every client-facing object maps to exactly one entity.

### Object
An object (represented on the server by an `ObjectID`) is a client's view onto an entity. different clients may know the same entity by different object IDs, and be given a different set of properties or property values.

### Component
A component is a strongly typed piece of state attached to an entity. It's generally a struct that contains elements. It usually represents something about an entity. For example entities that are attached to a `Body` component, have a size and position in 3D space, entities that are not represent non-physical concepts.

### Element
`Element<T>` is an atomic unit of state. It can be subscribed to, in which case it will notify the subscriber when it is changed. These notifications are __not__ dispatched immediately. Instead, they are queued and processed later in the main game loop.

### Property
A property is a client's view onto some piece of state. It has a name and is attached to an object. A client can get, set, subscribe and unsubscribe from a property. A property *often* maps 1:1 with an element, but may calculate it's value based on an element, or based on multiple elements. It might even be calculated from information from multiple entities. Making sure the right properties get updated when they need to and not when they don't is rather complicated.

## Code Style
### Documentation
Docs are important, and always encouraged. Write inline documentation with the standard [Rustdoc](https://blog.guillaume-gomez.fr/articles/2020-03-12+Guide+on+how+to+write+documentation+for+a+Rust+crate).

### Logging
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

### Clippy
Code should pass `cargo clippy` without warnings. If a warning is not useful, allow it with `#[allow(clippy::...)]`.

### Formatting
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