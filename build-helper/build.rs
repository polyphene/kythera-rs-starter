// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT

use anyhow::{bail, Context};
use convert_case::{Case, Casing};
use kythera_lib::{self, Abi, Method};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, thread};
use toml::Table;

use proc_macro2::TokenTree;
use syn::{Expr, Item};

const FILES_TO_WATCH: &[&str] = &["Cargo.toml", "src", "../actors", "../tests", "../artifacts"];

/// The Kind of actors to parse.
#[derive(Debug)]
enum Kind {
    Target,
    Test,
}

/// An actor crate with the name and its path.
struct ActorCrate {
    name: String,
    source: PathBuf,
}

impl ActorCrate {
    /// Create a new `ActorCrate` from an input location.
    fn new_from_path(path: &Path) -> Result<Self, anyhow::Error> {
        let root = fs::read_dir(path)
            .with_context(|| format!("Could not read path {}", path.display()))?;

        let cargo_path = root
            .filter_map(|e| e.ok())
            .map(|f| f.path())
            .find(|f| f.ends_with("Cargo.toml"))
            .with_context(|| {
                format!(
                    "path {} doesn't have a valid Cargo.toml file",
                    path.display()
                )
            })?;

        let mut cargo_file = File::open(&cargo_path)
            .with_context(|| format!("Could open {} file", cargo_path.display()))?;

        // Unfortunately `toml` doesn't have from_read() like `serde_json`
        // https://github.com/toml-rs/toml/issues/326
        let mut cargo_str = String::new();
        cargo_file
            .read_to_string(&mut cargo_str)
            .with_context(|| format!("Could not read  {} file", cargo_path.display()))?;

        let cargo = cargo_str
            .parse::<Table>()
            .with_context(|| format!("{} is not a valid TOML file", cargo_path.display()))?;

        let name = cargo
            .get("package")
            .and_then(|p| p.as_table())
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str())
            .with_context(|| {
                format!(
                    "{} is not a valid Rust Cargo.toml file, \"name\" is missing ",
                    cargo_path.display()
                )
            })?;

        Ok(Self {
            name: name.into(),
            source: path.join("src/actor.rs"),
        })
    }
}

/// Generate wasm actors for the input dir.
fn generate_actors(kind: Kind, clean_artifacts_dir: bool) -> Result<(), anyhow::Error> {
    let out_dir = std::env::var_os("OUT_DIR")
        .as_ref()
        .map(Path::new)
        .map(|p| p.join("bundle"))
        .context("no OUT_DIR env var")?;
    println!("cargo:warning=out_dir: {:?}", &out_dir);

    let manifest_dir =
        Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").context("CARGO_MANIFEST_DIR unset")?)
            .to_path_buf();

    let workspace_dir = &manifest_dir
        .parent()
        .context("Workspace doesn't exist exist")?;

    let artifacts_dir = workspace_dir.join("artifacts");

    // Cargo executable location.
    let cargo = std::env::var_os("CARGO").context("no CARGO env var")?;

    let path = match kind {
        Kind::Target => workspace_dir.join("actors"),
        Kind::Test => workspace_dir.join("tests"),
    };

    let actors = fs::read_dir(&path)
        .with_context(|| format!("Could not read dir {}", path.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_dir())
        .filter(|e| {
            fs::read_dir(e)
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|f| f.path().ends_with("Cargo.toml"))
        })
        .filter_map(|p| ActorCrate::new_from_path(&p).ok())
        .collect::<Vec<ActorCrate>>();

    let actor_names = actors
        .iter()
        .map(|a| a.name.as_str())
        .collect::<Vec<&str>>();

    // Cargo build command for all test_actors at once.
    let mut cmd = Command::new(cargo);
    cmd.arg("build")
        .args(actor_names.iter().map(|pkg| "-p=".to_owned() + pkg))
        .arg("--target=wasm32-unknown-unknown")
        .arg("--profile=wasm")
        .arg("--locked")
        .arg("--manifest-path=".to_owned() + manifest_dir.join("../Cargo.toml").to_str().unwrap())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // We are supposed to only generate artifacts under OUT_DIR,
        // so set OUT_DIR as the target directory for this build.
        .env("CARGO_TARGET_DIR", &out_dir)
        // As we are being called inside a build-script, this env variable is set. However, we set
        // our own `RUSTFLAGS` and thus, we need to remove this. Otherwise cargo favors this
        // env variable.
        .env_remove("CARGO_ENCODED_RUSTFLAGS");

    // Print out the command line we're about to run.
    println!("cargo:warning=cmd={:?}", &cmd);

    // Launch the command.
    let mut child = cmd.spawn().context("failed to launch cargo build")?;

    // Pipe the output as cargo warnings. Unfortunately this is the only way to
    // get cargo build to print the output.
    let stdout = child.stdout.take().context("Stdout is not available")?;
    let stderr = child.stderr.take().context("Stderr is not available")?;
    let j1 = thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            println!("cargo:warning={:?}", line.unwrap());
        }
    });
    let j2 = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            println!("cargo:warning={:?}", line.unwrap());
        }
    });

    j1.join().unwrap();
    j2.join().unwrap();

    let result = child.wait().context("failed to wait for build to finish")?;
    if !result.success() {
        bail!("actor build failed");
    }

    // Create artifacts dir.
    if let Err(err) = fs::create_dir(&artifacts_dir) {
        match err.kind() {
            std::io::ErrorKind::AlreadyExists => {
                if clean_artifacts_dir {
                    fs::remove_dir_all(&artifacts_dir)
                        .context("Could not remove artifacts dir")?;
                    fs::create_dir(&artifacts_dir).context("Could not create artifacts dir")?;
                }
            }
            err => bail!("Could not create artifacts dir {err}"),
        }
    };

    // Create the Abi files and copy the wasm files to the artifacts dir.
    for actor in actors {
        let src = fs::read_to_string(&actor.source)
            .with_context(|| format!("Could not open {}", actor.source.display()))?;
        let syntax = syn::parse_file(&src)
            .with_context(|| format!("Could not parse {}", actor.source.display()))?;
        let invoke = syntax
            .items
            .into_iter()
            .find_map(|item| match item {
                Item::Fn(f) if f.sig.ident == "invoke" => Some(f),
                _ => None,
            })
            .with_context(|| {
                format!(
                    "Could not find invoke function on {}",
                    actor.source.display()
                )
            })?;

        let match_method = invoke
            .block
            .stmts
            .into_iter()
            .find_map(|s| match s {
                syn::Stmt::Expr(Expr::Macro(m), _)
                    if m.mac
                        .path
                        .get_ident()
                        .filter(|i| *i == "match_method")
                        .is_some() =>
                {
                    Some(m)
                }
                _ => None,
            })
            .with_context(|| {
                format!(
                    "Could not find match_method macro in the invoke function of {}",
                    actor.source.display(),
                )
            })?;
        let group = match_method
            .mac
            .tokens
            .into_iter()
            .find_map(|t| match t {
                TokenTree::Group(g) => Some(g),
                _ => None,
            })
            .with_context(|| {
                format!(
                    "Could not parse the match_method contents of {}",
                    actor.source.display(),
                )
            })?;

        let mut constructor = None;
        let mut set_up = None;
        let mut methods = vec![];
        for token in group.stream().into_iter() {
            match token {
                TokenTree::Literal(l) => match l.to_string().deref() {
                    "\"Constructor\"" => {
                        constructor = Some(Method::new_from_name("Constructor").unwrap());
                    }
                    "\"SetUp\"" => {
                        set_up = Some(Method::new_from_name("SetUp").unwrap());
                    }
                    m => {
                        let m = m.to_string();
                        let method = m.trim_matches('"');
                        methods.push(Method::new_from_name(method).with_context(|| {
                            format!(
                                "Could not generate Method for method {} of actor {}",
                                m,
                                actor.source.display()
                            )
                        })?);
                    }
                },
                _ => {}
            }
        }
        let abi = Abi {
            constructor,
            set_up,
            methods,
        };

        let actor_wasm_file = out_dir.join(format!(
            "wasm32-unknown-unknown/wasm/{}.wasm",
            // Cargo replaces -'s for _'s on compilation targets.
            &actor.name.replace("-", "_")
        ));

        // If the Actor is a test actor we rename the trailing `-test` to `.t`.
        let actor_dest_name = match kind {
            Kind::Target => actor.name,
            Kind::Test => match actor.name.rfind("test") {
                Some(index) => {
                    let (before, _after) = actor.name.split_at(index);
                    format!("{}.t", before)
                }
                None => bail!(format!(
                    "{} actor should be a test actor, but doesn't have test in its name",
                    actor.name
                )),
            },
        }
        .to_case(Case::Pascal);

        fs::copy(
            &actor_wasm_file,
            artifacts_dir.join(format!("{actor_dest_name}.wasm")),
        )
        .with_context(|| {
            format!(
                "Could not copy {} wasm file to artifacts dir",
                &actor_wasm_file.display()
            )
        })?;

        let mut abi_file =
            File::create(artifacts_dir.join(format!("{actor_dest_name}.cbor"))).unwrap();
        abi_file
            .write_all(&kythera_lib::to_vec(&abi).unwrap())
            .with_context(|| {
                format!(
                    "Could not generate Abi file for actor {}",
                    actor.source.display()
                )
            })?;
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    for file in FILES_TO_WATCH {
        println!("cargo:rerun-if-changed={}", file);
    }

    generate_actors(Kind::Target, true).expect("Could not generate target actors");
    generate_actors(Kind::Test, false).expect("Could not generate test actors");

    Ok(())
}
