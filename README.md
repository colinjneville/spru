# spru
[![Crates.io](https://img.shields.io/crates/v/spru.svg)](https://crates.io/crates/spru)
[![Docs](https://docs.rs/spru/badge.svg)](https://docs.rs/spru/latest/spru/)  

An experimental framework for building multiplayer strategy and digital board games.  

spru is designed to structuralize game logic in such a way that synchronization, saving, undo, simultaneous actions, and hidden information, can be managed automatically. It is made for portability and flexibility - it is WASM-compatible, not tied to any specific engine, and works using user-defined Rust types (which you can use to layer a scripting language on top, if desired).  
spru works similarly to a specialized in-memory database, providing atomicity, consistency, and isolation. As spru is in-memory only, it does not natively provide durability.

# spru-util
[![Crates.io](https://img.shields.io/crates/v/spru-util.svg)](https://crates.io/crates/spru-util)
[![Docs](https://docs.rs/spru-util/badge.svg)](https://docs.rs/spru-bevy/latest/spru-util/)  

A collection of reusable components for use in spru games.

# spru-bevy
[![Crates.io](https://img.shields.io/crates/v/spru-bevy.svg)](https://crates.io/crates/spru-bevy)
[![Docs](https://docs.rs/spru-bevy/badge.svg)](https://docs.rs/spru-bevy/latest/spru-bevy/)  

A spru implementation for the [bevy](https://bevyengine.org/) game engine.  

# spru-quibbler
An example word game built on spru-bevy.
