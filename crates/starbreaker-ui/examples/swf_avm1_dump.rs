//! AVM1 bytecode dumper for framework-constant truth mining (plan P2.1,
//! ledger item 27).
//!
//! Input: an SWF path (e.g. the extracted `BuildingBlocks_root.swf`). For
//! every `DoInitAction` / `DoAction` tag it walks the action stream —
//! recursing into `DefineFunction`/`DefineFunction2`/`With`/`Try` bodies,
//! where class methods keep their constants — and emits, per export name
//! (`DoInitAction.id` paired with the `ExportAssets` name):
//! the `ConstantPool` strings and every numeric/string `Push` with its
//! enclosing function path. Output is line-oriented and greppable:
//!
//! ```text
//! === DoInitAction id=290 export=__Packages.bhvr.ui.SomeView
//! pool[12] = "setSize"
//! push Int 44 in SomeView/<anon>/setSize
//! ```
//!
//! Usage: `cargo run -p starbreaker-ui --example swf_avm1_dump -- <file.swf>`

use std::collections::HashMap;

use swf::avm1::read::Reader;
use swf::avm1::types::{Action, Value};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: swf_avm1_dump <file.swf>");
        std::process::exit(64);
    };
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("error: read {path}: {e}");
        std::process::exit(1);
    });
    let buf = swf::decompress_swf(std::io::Cursor::new(bytes.as_slice()))
        .expect("decompress SWF");
    let parsed = swf::parse_swf(&buf).expect("parse SWF");
    let version = parsed.header.version();
    let encoding = swf::SwfStr::encoding_for_version(version);

    // ExportAssets pairs character ids with the linkage/export name
    // (`__Packages.<class>` for AS2 class initialisers).
    let mut export_names: HashMap<u16, String> = HashMap::new();
    for tag in &parsed.tags {
        if let swf::Tag::ExportAssets(assets) = tag {
            for asset in assets {
                export_names.insert(asset.id, asset.name.to_str_lossy(encoding).into_owned());
            }
        }
    }

    let mut init_count = 0usize;
    for tag in &parsed.tags {
        match tag {
            swf::Tag::DoInitAction { id, action_data } => {
                init_count += 1;
                let export = export_names
                    .get(id)
                    .map(String::as_str)
                    .unwrap_or("?");
                println!("=== DoInitAction id={id} export={export}");
                dump_block(action_data, version, encoding, "");
            }
            swf::Tag::DoAction(action_data) => {
                println!("=== DoAction (timeline)");
                dump_block(action_data, version, encoding, "");
            }
            _ => {}
        }
    }
    eprintln!("{init_count} DoInitAction tags");
}

/// Walk one action block, printing pools and pushes; recurse into bodies.
/// `fn_path` is the enclosing function chain (for locating a constant).
fn dump_block(data: &[u8], version: u8, encoding: &'static swf::Encoding, fn_path: &str) {
    let mut reader = Reader::new(data, version);
    loop {
        if reader.get_ref().is_empty() {
            break;
        }
        let action = match reader.read_action() {
            Ok(action) => action,
            Err(e) => {
                println!("!! parse error in {fn_path:?}: {e}");
                break;
            }
        };
        match action {
            Action::End => break,
            Action::ConstantPool(pool) => {
                for (index, string) in pool.strings.iter().enumerate() {
                    println!("pool[{index}] = {:?}", string.to_str_lossy(encoding));
                }
            }
            Action::Push(push) => {
                for value in &push.values {
                    let formatted = match value {
                        Value::Int(v) => Some(format!("Int {v}")),
                        Value::Float(v) => Some(format!("Float {v}")),
                        Value::Double(v) => Some(format!("Double {v}")),
                        Value::Str(s) => {
                            Some(format!("Str {:?}", s.to_str_lossy(encoding)))
                        }
                        _ => None,
                    };
                    if let Some(formatted) = formatted {
                        println!("push {formatted} in {}", display_path(fn_path));
                    }
                }
            }
            Action::DefineFunction(function) => {
                let name = function.name.to_str_lossy(encoding);
                let child = join_path(fn_path, if name.is_empty() { "<anon>" } else { &name });
                dump_block(function.actions, version, encoding, &child);
            }
            Action::DefineFunction2(function) => {
                let name = function.name.to_str_lossy(encoding);
                let child = join_path(fn_path, if name.is_empty() { "<anon>" } else { &name });
                dump_block(function.actions, version, encoding, &child);
            }
            Action::With(with) => {
                dump_block(with.actions, version, encoding, &join_path(fn_path, "<with>"));
            }
            Action::Try(try_block) => {
                dump_block(try_block.try_body, version, encoding, &join_path(fn_path, "<try>"));
                if let Some((_, catch_body)) = try_block.catch_body {
                    dump_block(catch_body, version, encoding, &join_path(fn_path, "<catch>"));
                }
                if let Some(finally_body) = try_block.finally_body {
                    dump_block(finally_body, version, encoding, &join_path(fn_path, "<finally>"));
                }
            }
            _ => {}
        }
    }
}

fn join_path(base: &str, leaf: &str) -> String {
    if base.is_empty() {
        leaf.to_string()
    } else {
        format!("{base}/{leaf}")
    }
}

fn display_path(fn_path: &str) -> &str {
    if fn_path.is_empty() { "<top>" } else { fn_path }
}
