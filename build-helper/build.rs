use convert_case::{Case, Casing};
// Copyright 2023 Polyphene.
// SPDX-License-Identifier: Apache-2.0, MIT
use kythera_lib::{self, Abi, Method};
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{fs, thread};

use proc_macro2::TokenTree;
use syn::{Expr, Item};

const FILES_TO_WATCH: &[&str] = &["Cargo.toml", "src", "../actors", "../tests"];

/// The Kind of actor.
#[derive(Debug)]
enum Kind {
    Target,
    Test,
}

/// Generate wasm actors for the input dir.
fn generate_actors(kind: Kind) -> Result<(), Box<dyn Error>> {
    let out_dir = std::env::var_os("OUT_DIR")
        .as_ref()
        .map(Path::new)
        .map(|p| p.join("bundle"))
        .expect("no OUT_DIR env var");
    println!("cargo:warning=out_dir: {:?}", &out_dir);

    let manifest_dir =
        Path::new(&std::env::var_os("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR unset"))
            .to_path_buf();

    let workspace_dir = &manifest_dir.parent().expect("Workspace dir should exist");

    let artifacts_dir = workspace_dir.join("artifacts");

    // Cargo executable location.
    let cargo = std::env::var_os("CARGO").expect("no CARGO env var");

    let path = match kind {
        Kind::Target => workspace_dir.join("actors"),
        Kind::Test => workspace_dir.join("tests"),
    };

    let actor_dirs = fs::read_dir(&path)
        .expect(&format!("Could not read dir {}", path.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_dir())
        .filter(|e| {
            fs::read_dir(e)
                .unwrap()
                .filter_map(|e| e.ok())
                .any(|f| f.path().ends_with("Cargo.toml"))
        })
        .collect::<Vec<PathBuf>>();

    let actor_names = actor_dirs
        .iter()
        .filter_map(|a| a.file_name())
        .filter_map(|a| a.to_str())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

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
    let mut child = cmd.spawn().expect("failed to launch cargo build");

    // Pipe the output as cargo warnings. Unfortunately this is the only way to
    // get cargo build to print the output.
    let stdout = child.stdout.take().expect("no stdout");
    let stderr = child.stderr.take().expect("no stderr");
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

    let result = child.wait().expect("failed to wait for build to finish");
    if !result.success() {
        return Err("actor build failed".into());
    }

    // Create the Abi files and copy the wasm files to the artifacts dir.
    let actor_source_files = actor_dirs.iter().map(|a| (a, a.join("src/actor.rs")));
    for (actor_dir, actor_path) in actor_source_files {
        let src = fs::read_to_string(&actor_path)
            .expect(&format!("Could not open {}", actor_path.display()));
        let syntax =
            syn::parse_file(&src).expect(&format!("Could not parse {}", actor_path.display()));
        let invoke = syntax
            .items
            .into_iter()
            .find_map(|item| match item {
                Item::Fn(f) if f.sig.ident == "invoke" => Some(f),
                _ => None,
            })
            .expect(&format!(
                "Could not find invoke function on {}",
                actor_path.display()
            ));

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
            .expect(&format!(
                "Could not find match_method macro in the invoke function of {}",
                actor_path.display(),
            ));
        let group = match_method
            .mac
            .tokens
            .into_iter()
            .find_map(|t| match t {
                TokenTree::Group(g) => Some(g),
                _ => None,
            })
            .expect(&format!(
                "Could not parse the match_method contents of {}",
                actor_path.display(),
            ));

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
                        methods.push(Method::new_from_name(method).expect(&format!(
                            "Could not generate Method for method {} of actor {}",
                            m,
                            actor_path.display()
                        )));
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

        // Create artifacts dir.
        if let Err(err) = fs::create_dir(&artifacts_dir) {
            match err.kind() {
                std::io::ErrorKind::AlreadyExists => {}
                err => panic!("Could not create artifacts dir {err}"),
            }
        };

        // Safe to `unwrap()` as at this time we know the file exists.
        let actor_name = actor_dir
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            // Rust replaces the -'s in the crate name with _'s,
            // so we replace it back.
            .replace("-", "_");

        let actor_wasm_file =
            out_dir.join(format!("wasm32-unknown-unknown/wasm/{actor_name}.wasm"));

        let actor_dest_name = actor_name.replace("test", ".t").to_case(Case::Pascal);

        fs::copy(
            &actor_wasm_file,
            artifacts_dir.join(format!("{actor_dest_name}.wasm")),
        )
        .expect("Could not copy wasm file to artifacts dir");

        let mut abi_file =
            File::create(artifacts_dir.join(format!("{actor_dest_name}.cbor"))).unwrap();
        abi_file
            .write_all(&kythera_lib::to_vec(&abi).unwrap())
            .expect(&format!(
                "Could not generate Abi file for actor {}",
                actor_path.display()
            ));
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    for file in FILES_TO_WATCH {
        println!("cargo:rerun-if-changed={}", file);
    }

    generate_actors(Kind::Target).expect("Could not generate target actors");
    generate_actors(Kind::Test).expect("Could not generate test actors");

    Ok(())
}
