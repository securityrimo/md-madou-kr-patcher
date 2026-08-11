# md-madou-kr-patcher

Rust code for the Korean patch toolchain for *Madou Monogatari I* on Mega Drive.

This repository contains the ROM format handling, JP-native text pipeline, checked Motorola 68000 code generation, graphics compilers, validation logic, and BPS/IPS support. Motorola 68000 instructions are encoded and complete streams are verified through the pinned [`retro-typed-isa`](https://github.com/mcpads/retro-typed-isa) profile. This repository does not include ROM images, translated scripts, translated graphics, fonts, or other private build assets.

## Build and test

Rust 2024 edition and a current stable Rust toolchain are required.

```sh
cargo build
cargo test --all-targets
```

The test suite uses only repository code and small synthetic fixtures. ROM surveys and private localization integration checks are maintained outside this public repository.

Available commands can be listed with:

```sh
cargo run -- --help
```

The complete JP-to-KR ROM and BPS build requires the supported original Japanese ROM and private localization assets. This public repository alone does not reproduce the distributed patch.

## License

The source code in this repository is provided under the [MIT License](LICENSE). Third-party games, fonts, tools, and assets remain subject to their respective rights and licenses.
