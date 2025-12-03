This project is a fully custom 1.12.1 server that aims to provide a fully separate experience instead of what was provided initially in classic World of Warcraft. It uses completely custom quests, creatures and other data that is not connected with original World of Warcraft in any way.

To use this server you must have obtained a valid copy of World of Warcraft's client.
Logging into the server without one is **violating [Blizzard Entertainment, Inc.][1] copyright**.

Building & Running
--------
To build the project you'll need to install Rust from [rust-lang.org](https://rust-lang.org/tools/install/), after which you'll be able to run the following commands.

To compile the server into binaries, simply run `cargo build [--release]` in the root directory where you downloaded the project. (--release) flag is optional to enable all optimizations, otherwise it'll compile in debug mode.

After compilation, your binaries will be located in `target/release/` or `target/debug/` in case of a debug build.

To run the server you'll need to run __both__ the authserver (being the server that you authenticate on when you first connect) and the gameserver (the server which handles all in-game interactions).

__If you're using this method__, copy `authserver/authserver_config.toml`, `authserver/log4rs.yaml`, `gameserver/gameserver_config.toml` and `gameserver/log4rs.yaml` into the folders where your binaries are located, otherwise they will immediately crash with an error due to the configs missing.

--------
Alternatively, you can run `cargo run [--release]` inside `authserver/` or `gameserver/` directories to run the servers without messing with binaries.

Contact
--------
Development Discord server: [https://discord.gg/4WgSHzr9z4](https://discord.gg/4WgSHzr9z4)

Acknowledgements
--------
While we aim to create a custom experience, the further acknowledgement still holds:
World of Warcraft, and all World of Warcraft or Warcraft art, images, and lore are copyrighted by [Blizzard Entertainment, Inc.][1]

[1]: http://blizzard.com/ "Blizzard Entertainment Inc."
