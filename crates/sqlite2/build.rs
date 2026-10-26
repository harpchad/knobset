// Builds the vendored SQLite 2.8.17 engine (public domain).
use std::env;
use std::fs;
use std::path::PathBuf;

const SOURCES: &[&str] = &[
    "attach", "auth", "btree", "btree_rb", "build", "copy", "date", "delete", "encode", "expr", "func",
    "hash", "insert", "main", "opcodes", "os", "pager", "parse", "pragma", "printf", "random", "select",
    "table", "tokenize", "trigger", "update", "util", "vacuum", "vdbe", "vdbeaux", "where",
];

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let ptr = env::var("CARGO_CFG_TARGET_POINTER_WIDTH").unwrap();
    let ptr_bytes: u32 = ptr.parse::<u32>().unwrap() / 8;
    // sqliteInt.h includes "config.h"; configure normally writes just this.
    fs::write(out.join("config.h"), format!("#define SQLITE_PTR_SZ {ptr_bytes}\n")).unwrap();

    let windows = env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows";
    let mut b = cc::Build::new();
    b.include(&out).include("vendor").warnings(false);
    for s in SOURCES {
        b.file(format!("vendor/{s}.c"));
    }
    if windows {
        b.define("OS_WIN", "1").define("OS_UNIX", "0");
    } else {
        b.define("OS_UNIX", "1").define("OS_WIN", "0").define("HAVE_USLEEP", "1");
    }
    b.define("NDEBUG", None);
    // Required: the app and test harness open connections on several threads at once.
    // Without it SQLite 2's file-lock bookkeeping is unsynchronized and can deadlock.
    b.define("THREADSAFE", "1");
    // 2005-era C: tolerate implicit declarations and int/pointer conversions on modern clang/gcc.
    // MSVC accepts the code as is and only warns about these flags.
    if env::var("CARGO_CFG_TARGET_ENV").unwrap() != "msvc" {
        for f in ["-std=gnu89", "-Wno-implicit-function-declaration", "-Wno-int-conversion", "-Wno-implicit-int"] {
            b.flag_if_supported(f);
        }
    }
    b.compile("sqlite2");

    println!("cargo:rerun-if-changed=vendor");
    println!("cargo:rerun-if-changed=build.rs");
}
