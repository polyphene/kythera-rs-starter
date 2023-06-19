# Kythera Rust Starter Kit

Create wasm target and test actors to be tested with [Kythera].
This project contains the necessary logic to build actors for our [Kythera] toolset.

It was heavily inspired and copied from the implementation [over the `ref-fvm`](https://github.com/filecoin-project/ref-fvm/tree/37643fc02f0342256afecff5158c43693b5ee4f0/testing/test_actors)
done by @fridrik01.

## Instructions

Clone this repository and look into the [actors](./actors/) and [tests](./tests/) dirs for examples on how to create both `Target` and `Test` actors.

A new actor and test actor templates can be created with the `create-actor.sh` script.

```shell
./create-actor.sh <actor-name> 
```

## Caveats

Currently there are some shortcommings with the Starter kit:

- User can only have one `test` actor per `target` actor.
- `build-helper` directory needs to be mantained, it's where `build.rs` script resides and it is responsible for artifacts generation.
- User has to have the actor source file named `actor.rs` in the `/src` dir.
- Not directly because of the project structure but because of Rust itself, `build.rs` will only run if a source file changes.

## License

This project is licensed under the [MIT License](LICENSE).

## Contribute

Contributions are welcome! If you'd like to contribute to the  [Kythera] Rust Starter Kit, please follow these steps:

1. Fork the repository on GitHub.
2. Create a new branch with a descriptive name.
3. Make your desired changes.
4. Commit your changes and push the branch to your forked repository.
5. Open a pull request on the main repository, describing the changes you made.
6. THANKS!

Please ensure your contributions adhere to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md).

[Kythera]: https://github.com/polyphene/kythera
