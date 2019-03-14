#![allow(unused_imports)]

#[cfg(feature = "bench")]
extern crate test;

use serde_json;
use std::{fs, ffi, process};
use std::io::{self, Write};
use std::path::Path;
use walkdir::WalkDir;
use super::*;

#[test]
fn test_count_lines() {
    assert_eq!(count_lines(""), 1);
    assert_eq!(count_lines("this is a line"), 1);
    assert_eq!(count_lines("this is a line\n"), 2);
    assert_eq!(count_lines("\nthis is a line"), 2);
    assert_eq!(count_lines("\n\n\nthis is a line"), 4);
    assert_eq!(count_lines("this is a line\n\n\n"), 4);
    assert_eq!(count_lines("these\nare\nlines"), 3);
    assert_eq!(count_lines("\r\n"), 2);
    assert_eq!(count_lines("this is a line\r\n"), 2);
    assert_eq!(count_lines("\r\nthis is a line"), 2);
    assert_eq!(count_lines("these\nare\r\nlines"), 3);
}

#[test]
fn test_vlq() {
    // 0000000000000000111111111111111122222222222222223333333333333333
    // 0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
    // ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/
    let mut vlq = Vlq::new();
    assert_eq!(vlq.enc(0), "A");
    assert_eq!(vlq.enc(1), "C");
    assert_eq!(vlq.enc(-1), "D");
    assert_eq!(vlq.enc(5), "K");
    assert_eq!(vlq.enc(-5), "L");
    assert_eq!(vlq.enc(15), "e");
    assert_eq!(vlq.enc(-15), "f");
    assert_eq!(vlq.enc(16), "gB");
    assert_eq!(vlq.enc(1876), "o1D"); // 11 10101 0100
    assert_eq!(vlq.enc(-485223), "v2zd"); // 11101 10011 10110 0111
}

#[test]
fn test_deserialize_browser_subst() {
    let parse = serde_json::from_str::<BrowserSubstitution<String>>;
    assert_matches!(parse("null"), Err(_));
    assert_matches!(parse("100"), Err(_));
    assert_matches!(parse("[1, 2, 3]"), Err(_));
    assert_matches!(parse("false"), Ok(BrowserSubstitution::Ignore));
    assert_matches!(parse("true"), Err(_));
    assert_eq!(parse(r#""asdf""#).unwrap(), BrowserSubstitution::Replace("asdf".to_owned()));
    assert_eq!(parse(r#""""#).unwrap(), BrowserSubstitution::Replace("".to_owned()));
}

#[test]
fn test_deserialize_browser() {
    let parse = serde_json::from_str::<BrowserSubstitutionMap>;
    assert_matches!(parse(r#"null"#), Err(_));
    assert_matches!(parse(r#""simple.browser.js""#), Err(_));
    assert_eq!(parse(r#"{}"#).unwrap(), BrowserSubstitutionMap(map!{}));
    assert_eq!(parse(r#"{"mod": "dom"}"#).unwrap(), BrowserSubstitutionMap(map!{
        PathBuf::from("mod") => BrowserSubstitution::Replace(PathBuf::from("dom")),
    }));
    assert_eq!(parse(r#"{"./file.js": "./file.browser.js"}"#).unwrap(), BrowserSubstitutionMap(map!{
        PathBuf::from("./file.js") => BrowserSubstitution::Replace(PathBuf::from("./file.browser.js")),
    }));
    assert_eq!(parse(r#"{"ignore": false}"#).unwrap(), BrowserSubstitutionMap(map!{
        PathBuf::from("ignore") => BrowserSubstitution::Ignore,
    }));
    assert_eq!(parse(r#"{
        "ignore": false,
        "mod": "dom",
        "mod2file": "./modfile.js",
        "mod2up": "../up.js",
        "mod2dir": "./moddir",
        "mod2abs": "/z/y/x",
        "./fileignore.js": false,
        "./file2mod.js": "mod",
        "./file2file.js": "./file.js",
        "./file2dir.js": "./dir",
        "./file2up.js": "../up.js",
        "./file2abs.js": "/x/y/z"
    }"#).unwrap(), BrowserSubstitutionMap(map!{
        PathBuf::from("ignore") => BrowserSubstitution::Ignore,
        PathBuf::from("mod") => BrowserSubstitution::Replace(PathBuf::from("dom")),
        PathBuf::from("mod2file") => BrowserSubstitution::Replace(PathBuf::from("./modfile.js")),
        PathBuf::from("mod2up") => BrowserSubstitution::Replace(PathBuf::from("../up.js")),
        PathBuf::from("mod2dir") => BrowserSubstitution::Replace(PathBuf::from("./moddir")),
        PathBuf::from("mod2abs") => BrowserSubstitution::Replace(PathBuf::from("/z/y/x")),
        PathBuf::from("./fileignore.js") => BrowserSubstitution::Ignore,
        PathBuf::from("./file2mod.js") => BrowserSubstitution::Replace(PathBuf::from("mod")),
        PathBuf::from("./file2file.js") => BrowserSubstitution::Replace(PathBuf::from("./file.js")),
        PathBuf::from("./file2dir.js") => BrowserSubstitution::Replace(PathBuf::from("./dir")),
        PathBuf::from("./file2up.js") => BrowserSubstitution::Replace(PathBuf::from("../up.js")),
        PathBuf::from("./file2abs.js") => BrowserSubstitution::Replace(PathBuf::from("/x/y/z")),
    }));
}

#[test]
fn test_deserialize_package_info() {
    let parse = serde_json::from_str::<PackageInfo>;
    assert_matches!(parse("null"), Err(_));
    assert_matches!(parse("100"), Err(_));
    assert_matches!(parse("[1, 2, 3]"), Err(_));
    assert_eq!(parse(r#"{}"#).unwrap(), PackageInfo {
        main: PathBuf::from("./index"),
        browser_substitutions: BrowserSubstitutionMap(map!{}),
    });
    assert_eq!(parse(r#"{"browser": null}"#).unwrap(), PackageInfo {
        main: PathBuf::from("./index"),
        browser_substitutions: BrowserSubstitutionMap(map!{}),
    });
    assert_eq!(parse(r#"{"browser": "simple"}"#).unwrap(), PackageInfo {
        main: PathBuf::from("./index"),
        browser_substitutions: BrowserSubstitutionMap(map!{
            PathBuf::from("./index") => BrowserSubstitution::Replace(PathBuf::from("./simple")),
        }),
    });
    assert_eq!(parse(r#"{"browser": {}}"#).unwrap(), PackageInfo {
        main: PathBuf::from("./index"),
        browser_substitutions: BrowserSubstitutionMap(map!{}),
    });
    assert_eq!(parse(r#"{"browser": {"mod": false}}"#).unwrap(), PackageInfo {
        main: PathBuf::from("./index"),
        browser_substitutions: BrowserSubstitutionMap(map!{
            PathBuf::from("mod") => BrowserSubstitution::Ignore,
        }),
    });
}

fn fixture_path() -> PathBuf {
    // let mut path = PathBuf::from(file!());
    // path.append_resolving("../../../fixtures");
    let mut path = std::env::current_dir().unwrap();
    path.push("fixtures");
    path
}

enum Resolution<'a> {
    Y(&'a str),
    Ignore,
    External,
    Fail,
}
use self::Resolution::*;

#[test]
fn test_resolve_path_or_module() {
    fn path_resolves(from: &str, to: Option<&str>, input_options: &InputOptions) {
        let base_path = fixture_path();
        let to_path = to.map(|to| {
            let mut to_path = base_path.clone();
            to_path.append_resolving(to);
            to_path
        });
        let mut from_path = base_path;
        from_path.append_resolving(from);

        let resolver = Resolver::new(input_options.clone());
        let expected = to_path.map(Resolved::Normal);
        // resolves with an empty cache...
        assert_eq!(resolver.resolve_path_or_module(None, from_path.clone(), false, false).unwrap(), expected);
        // ...and with everything cached
        assert_eq!(resolver.resolve_path_or_module(None, from_path, false, false).unwrap(), expected);
    }
    let cjs = InputOptions {
        for_browser: false,
        es6_syntax: false,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };
    let esm = InputOptions {
        for_browser: false,
        es6_syntax: true,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };
    path_resolves("resolve/named-noext",
             Some("resolve/named-noext"), &cjs);
    path_resolves("resolve/named-js.js",
             Some("resolve/named-js.js"), &cjs);
    path_resolves("resolve/named-json.json",
             Some("resolve/named-json.json"), &cjs);
    path_resolves("resolve/named-mjs.mjs",
             Some("resolve/named-mjs.mjs"), &esm);
    path_resolves("resolve/named-jsz.jsz",
             Some("resolve/named-jsz.jsz"), &cjs);

    path_resolves("resolve/named-js",
             Some("resolve/named-js.js"), &cjs);
    path_resolves("resolve/named-json",
             Some("resolve/named-json.json"), &cjs);
    path_resolves("resolve/named-mjs",
             Some("resolve/named-mjs.mjs"), &esm);

    path_resolves("resolve/dir-js",
             Some("resolve/dir-js/index.js"), &cjs);
    path_resolves("resolve/dir-js/index",
             Some("resolve/dir-js/index.js"), &cjs);
    path_resolves("resolve/dir-json",
             Some("resolve/dir-json/index.json"), &cjs);
    path_resolves("resolve/dir-json/index",
             Some("resolve/dir-json/index.json"), &cjs);
    path_resolves("resolve/dir-mjs",
             Some("resolve/dir-mjs/index.mjs"), &esm);
    path_resolves("resolve/dir-mjs/index",
             Some("resolve/dir-mjs/index.mjs"), &esm);

    path_resolves("resolve/mod-noext-bare",
             Some("resolve/mod-noext-bare/main-noext"), &cjs);
    path_resolves("resolve/mod-noext-rel",
             Some("resolve/mod-noext-rel/main-noext"), &cjs);

    path_resolves("resolve/mod-main-nesting-bare",
             Some("resolve/mod-main-nesting-bare/subdir/index.js"), &cjs);
    path_resolves("resolve/mod-main-nesting-bare/subdir",
             Some("resolve/mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    path_resolves("resolve/mod-main-nesting-rel",
             Some("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    path_resolves("resolve/mod-main-nesting-rel/subdir",
             Some("resolve/mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    path_resolves("resolve/mod-js-ext-bare",
             Some("resolve/mod-js-ext-bare/main-js.js"), &cjs);
    path_resolves("resolve/mod-js-ext-rel",
             Some("resolve/mod-js-ext-rel/main-js.js"), &cjs);
    path_resolves("resolve/mod-js-noext-bare",
             Some("resolve/mod-js-noext-bare/main-js.js"), &cjs);
    path_resolves("resolve/mod-js-noext-rel",
             Some("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    path_resolves("resolve/mod-js-dir-bare",
             Some("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    path_resolves("resolve/mod-js-dir-rel",
             Some("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);

    path_resolves("resolve/mod-json-ext-bare",
             Some("resolve/mod-json-ext-bare/main-json.json"), &cjs);
    path_resolves("resolve/mod-json-ext-rel",
             Some("resolve/mod-json-ext-rel/main-json.json"), &cjs);
    path_resolves("resolve/mod-json-noext-bare",
             Some("resolve/mod-json-noext-bare/main-json.json"), &cjs);
    path_resolves("resolve/mod-json-noext-rel",
             Some("resolve/mod-json-noext-rel/main-json.json"), &cjs);
    path_resolves("resolve/mod-json-dir-bare",
             Some("resolve/mod-json-dir-bare/main-json/index.json"), &cjs);
    path_resolves("resolve/mod-json-dir-rel",
             Some("resolve/mod-json-dir-rel/main-json/index.json"), &cjs);

    path_resolves("resolve/mod-mjs-ext-bare",
             Some("resolve/mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    path_resolves("resolve/mod-mjs-ext-rel",
             Some("resolve/mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    path_resolves("resolve/mod-mjs-noext-bare",
             Some("resolve/mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    path_resolves("resolve/mod-mjs-noext-rel",
             Some("resolve/mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    path_resolves("resolve/mod-mjs-dir-bare",
             Some("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    path_resolves("resolve/mod-mjs-dir-rel",
             Some("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    path_resolves("resolve/named-jsz", None, &cjs);
}

fn assert_resolves(context: &str, from: &str, to: Resolution, input_options: &InputOptions) {
    let base_path = fixture_path();
    let expected = match to {
        Y(to) => {
            let mut to_path = base_path.clone();
            to_path.append_resolving(to);
            Some(Resolved::Normal(to_path))
        }
        Ignore => Some(Resolved::Ignore),
        External => Some(Resolved::External),
        Fail => None,
    };
    let mut context_path = base_path;
    context_path.append_resolving(context);

    let resolver = Resolver::new(input_options.clone());
    if let Some(expected) = expected {
        // resolves with an empty cache...
        assert_eq!(resolver.resolve(&context_path, from).unwrap(), expected);
        // ...and with everything cached
        assert_eq!(resolver.resolve(&context_path, from).unwrap(), expected);
    } else {
        // resolves with an empty cache...
        assert_matches!(resolver.resolve(&context_path, from), Err(_));
        // ...and with everything cached
        assert_matches!(resolver.resolve(&context_path, from), Err(_));
    }
}

#[test]
fn test_resolve() {
  test_resolve_with(assert_resolves);
}
#[test]
fn test_resolve_unicode() {
  test_resolve_unicode_with(assert_resolves);
}
fn test_resolve_with<F>(mut assert_resolves: F)
where F: FnMut(&str, &str, Resolution<'static>, &InputOptions) {
    let cjs = InputOptions {
        for_browser: false,
        es6_syntax: false,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };
    let esm = InputOptions {
        for_browser: false,
        es6_syntax: true,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };

    // relative paths

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx, "./named-noext",
                 Y("resolve/named-noext"), &cjs);
    assert_resolves(ctx, "./named-js.js",
                 Y("resolve/named-js.js"), &cjs);
    assert_resolves(ctx, "./named-json.json",
                 Y("resolve/named-json.json"), &cjs);
    assert_resolves(ctx, "./named-mjs.mjs",
                 Y("resolve/named-mjs.mjs"), &esm);
    assert_resolves(ctx, "./named-jsz.jsz",
                 Y("resolve/named-jsz.jsz"), &cjs);

    assert_resolves(ctx, "./named-js",
                 Y("resolve/named-js.js"), &cjs);
    assert_resolves(ctx, "./named-json",
                 Y("resolve/named-json.json"), &cjs);
    assert_resolves(ctx, "./named-mjs",
                 Y("resolve/named-mjs.mjs"), &esm);

    assert_resolves(ctx, "./dir-js",
                 Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "./dir-js/index",
                 Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "./dir-json",
                 Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves(ctx, "./dir-json/index",
                 Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves(ctx, "./dir-mjs",
                 Y("resolve/dir-mjs/index.mjs"), &esm);
    assert_resolves(ctx, "./dir-mjs/index",
                 Y("resolve/dir-mjs/index.mjs"), &esm);

    assert_resolves(ctx, "./mod-noext-bare",
                 Y("resolve/mod-noext-bare/main-noext"), &cjs);
    assert_resolves(ctx, "./mod-noext-rel",
                 Y("resolve/mod-noext-rel/main-noext"), &cjs);

    assert_resolves(ctx, "./mod-main-nesting-bare",
                 Y("resolve/mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves(ctx, "./mod-main-nesting-bare/subdir",
                 Y("resolve/mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves(ctx, "./mod-main-nesting-rel",
                 Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves(ctx, "./mod-main-nesting-rel/subdir",
                 Y("resolve/mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves(ctx, "./mod-js-ext-bare",
                 Y("resolve/mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves(ctx, "./mod-js-ext-rel",
                 Y("resolve/mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "./mod-js-noext-bare",
                 Y("resolve/mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves(ctx, "./mod-js-noext-rel",
                 Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "./mod-js-dir-bare",
                 Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves(ctx, "./mod-js-dir-rel",
                 Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves(ctx, "./mod-json-ext-bare",
                 Y("resolve/mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves(ctx, "./mod-json-ext-rel",
                 Y("resolve/mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves(ctx, "./mod-json-noext-bare",
                 Y("resolve/mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves(ctx, "./mod-json-noext-rel",
                 Y("resolve/mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves(ctx, "./mod-json-dir-bare",
                 Y("resolve/mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves(ctx, "./mod-json-dir-rel",
                 Y("resolve/mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves(ctx, "./mod-mjs-ext-bare",
                 Y("resolve/mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "./mod-mjs-ext-rel",
                 Y("resolve/mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "./mod-mjs-noext-bare",
                 Y("resolve/mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "./mod-mjs-noext-rel",
                 Y("resolve/mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "./mod-mjs-dir-bare",
                 Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves(ctx, "./mod-mjs-dir-rel",
                 Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves(ctx, "./mod-js-slash-bare",
                 Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves(ctx, "./mod-js-slash-rel",
                 Y("resolve/mod-js-slash-rel/main.js"), &cjs);

    assert_resolves(ctx, "./named-jsz", Fail, &cjs);

    assert_resolves(ctx, "./file-and-dir",
                 Y("resolve/file-and-dir.js"), &cjs);
    assert_resolves(ctx, "./file-and-dir/",
                 Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves(ctx, "./file-and-mod",
                 Y("resolve/file-and-mod.js"), &cjs);
    assert_resolves(ctx, "./file-and-mod/",
                 Y("resolve/file-and-mod/main.js"), &cjs);
    assert_resolves(ctx, "./dir-js/",
                 Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "./mod-js-noext-rel/",
                 Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "./named-js.js/", Fail, &cjs);
    assert_resolves(ctx, "./named-js/", Fail, &cjs);
    assert_resolves(ctx, "./named-noext/", Fail, &cjs);

    let ctx = "resolve/subdir/hypothetical.js";
    assert_resolves(ctx, "./named-js", Fail, &cjs);

    assert_resolves(ctx, "../named-noext",
                  Y("resolve/named-noext"), &cjs);
    assert_resolves(ctx, "../named-js.js",
                  Y("resolve/named-js.js"), &cjs);
    assert_resolves(ctx, "../named-json.json",
                  Y("resolve/named-json.json"), &cjs);
    assert_resolves(ctx, "../named-mjs.mjs",
                  Y("resolve/named-mjs.mjs"), &esm);
    assert_resolves(ctx, "../named-jsz.jsz",
                  Y("resolve/named-jsz.jsz"), &cjs);

    assert_resolves(ctx, "../named-js",
                  Y("resolve/named-js.js"), &cjs);
    assert_resolves(ctx, "../named-json",
                  Y("resolve/named-json.json"), &cjs);
    assert_resolves(ctx, "../named-mjs",
                  Y("resolve/named-mjs.mjs"), &esm);

    assert_resolves(ctx, "../dir-js",
                  Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "../dir-js/index",
                  Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "../dir-json",
                  Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves(ctx, "../dir-json/index",
                  Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves(ctx, "../dir-mjs",
                  Y("resolve/dir-mjs/index.mjs"), &esm);
    assert_resolves(ctx, "../dir-mjs/index",
                  Y("resolve/dir-mjs/index.mjs"), &esm);

    assert_resolves(ctx, "../mod-noext-bare",
                  Y("resolve/mod-noext-bare/main-noext"), &cjs);
    assert_resolves(ctx, "../mod-noext-rel",
                  Y("resolve/mod-noext-rel/main-noext"), &cjs);

    assert_resolves(ctx, "../mod-main-nesting-bare",
                  Y("resolve/mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves(ctx, "../mod-main-nesting-bare/subdir",
                  Y("resolve/mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves(ctx, "../mod-main-nesting-rel",
                  Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves(ctx, "../mod-main-nesting-rel/subdir",
                  Y("resolve/mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves(ctx, "../mod-js-ext-bare",
                  Y("resolve/mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves(ctx, "../mod-js-ext-rel",
                  Y("resolve/mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "../mod-js-noext-bare",
                  Y("resolve/mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves(ctx, "../mod-js-noext-rel",
                  Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "../mod-js-dir-bare",
                  Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves(ctx, "../mod-js-dir-rel",
                  Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves(ctx, "../mod-json-ext-bare",
                  Y("resolve/mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves(ctx, "../mod-json-ext-rel",
                  Y("resolve/mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves(ctx, "../mod-json-noext-bare",
                  Y("resolve/mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves(ctx, "../mod-json-noext-rel",
                  Y("resolve/mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves(ctx, "../mod-json-dir-bare",
                  Y("resolve/mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves(ctx, "../mod-json-dir-rel",
                  Y("resolve/mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves(ctx, "../mod-mjs-ext-bare",
                  Y("resolve/mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "../mod-mjs-ext-rel",
                  Y("resolve/mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "../mod-mjs-noext-bare",
                  Y("resolve/mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "../mod-mjs-noext-rel",
                  Y("resolve/mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx, "../mod-mjs-dir-bare",
                  Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves(ctx, "../mod-mjs-dir-rel",
                  Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves(ctx, "../mod-js-slash-bare",
                  Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves(ctx, "../mod-js-slash-rel",
                  Y("resolve/mod-js-slash-rel/main.js"), &cjs);

    assert_resolves(ctx, "../named-jsz", Fail, &cjs);

    assert_resolves(ctx, "../file-and-dir",
                  Y("resolve/file-and-dir.js"), &cjs);
    assert_resolves(ctx, "../file-and-dir/",
                  Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves(ctx, "../file-and-mod",
                  Y("resolve/file-and-mod.js"), &cjs);
    assert_resolves(ctx, "../file-and-mod/",
                  Y("resolve/file-and-mod/main.js"), &cjs);
    assert_resolves(ctx, "../dir-js/",
                  Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves(ctx, "../mod-js-noext-rel/",
                  Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx, "../named-js.js/", Fail, &cjs);
    assert_resolves(ctx, "../named-js/", Fail, &cjs);
    assert_resolves(ctx, "../named-noext/", Fail, &cjs);

    assert_resolves(ctx, "../mod-self-slash",
                  Y("resolve/mod-self-slash/index.js"), &esm);
    assert_resolves(ctx, "../mod-self-slash/",
                  Y("resolve/mod-self-slash/index.js"), &esm);
    assert_resolves(ctx, "../mod-self-noslash",
                  Y("resolve/mod-self-noslash/index.js"), &esm);
    assert_resolves(ctx, "../mod-self-noslash/",
                  Y("resolve/mod-self-noslash/index.js"), &esm);
    assert_resolves(ctx, "../mod-outer/mod-parent-slash",
                  Y("resolve/mod-outer/index.js"), &esm);
    assert_resolves(ctx, "../mod-outer/mod-parent-slash/",
                  Y("resolve/mod-outer/index.js"), &esm);
    assert_resolves(ctx, "../mod-outer/mod-parent-noslash",
                  Y("resolve/mod-outer/index.js"), &esm);
    assert_resolves(ctx, "../mod-outer/mod-parent-noslash/",
                  Y("resolve/mod-outer/index.js"), &esm);
    assert_resolves("resolve/mod-outer/mod-parent-slash/hypothetical.js", "..",
                  Y("resolve/mod-outer/main.js"), &esm);
    assert_resolves("resolve/mod-outer/mod-parent-slash/hypothetical.js", "../",
                  Y("resolve/mod-outer/main.js"), &esm);
    assert_resolves("resolve/mod-outer/mod-parent-noslash/hypothetical.js", "..",
                  Y("resolve/mod-outer/main.js"), &esm);
    assert_resolves("resolve/mod-outer/mod-parent-noslash/hypothetical.js", "../",
                  Y("resolve/mod-outer/main.js"), &esm);

    assert_resolves("resolve/dir-js/hypothetical.js", ".",
                  Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves("resolve/dir-js/hypothetical.js", "./",
                  Y("resolve/dir-js/index.js"), &cjs);
    assert_resolves("resolve/dir-json/hypothetical.js", ".",
                  Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves("resolve/dir-json/hypothetical.js", "./",
                  Y("resolve/dir-json/index.json"), &cjs);
    assert_resolves("resolve/dir-mjs/hypothetical.js", ".",
                  Y("resolve/dir-mjs/index.mjs"), &esm);
    assert_resolves("resolve/dir-mjs/hypothetical.js", "./",
                  Y("resolve/dir-mjs/index.mjs"), &esm);

    assert_resolves("resolve/mod-noext-bare/hypothetical.js", ".",
                  Y("resolve/mod-noext-bare/main-noext"), &cjs);
    assert_resolves("resolve/mod-noext-bare/hypothetical.js", "./",
                  Y("resolve/mod-noext-bare/main-noext"), &cjs);
    assert_resolves("resolve/mod-noext-rel/hypothetical.js", ".",
                  Y("resolve/mod-noext-rel/main-noext"), &cjs);
    assert_resolves("resolve/mod-noext-rel/hypothetical.js", "./",
                  Y("resolve/mod-noext-rel/main-noext"), &cjs);

    assert_resolves("resolve/mod-main-nesting-bare/hypothetical.js", ".",
                  Y("resolve/mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-bare/hypothetical.js", "./",
                  Y("resolve/mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-bare/subdir/hypothetical.js", ".",
                  Y("resolve/mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-bare/subdir/hypothetical.js", "./",
                  Y("resolve/mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/hypothetical.js", ".",
                  Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/hypothetical.js", "./",
                  Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/subdir/hypothetical.js", "..",
                  Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/subdir/hypothetical.js", "../",
                  Y("resolve/mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/subdir/hypothetical.js", ".",
                  Y("resolve/mod-main-nesting-rel/subdir/inner-main.js"), &cjs);
    assert_resolves("resolve/mod-main-nesting-rel/subdir/hypothetical.js", "./",
                  Y("resolve/mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves("resolve/mod-js-ext-bare/hypothetical.js", ".",
                  Y("resolve/mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-ext-bare/hypothetical.js", "./",
                  Y("resolve/mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-ext-rel/hypothetical.js", ".",
                  Y("resolve/mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-ext-rel/hypothetical.js", "./",
                  Y("resolve/mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-noext-bare/hypothetical.js", ".",
                  Y("resolve/mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-noext-bare/hypothetical.js", "./",
                  Y("resolve/mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-noext-rel/hypothetical.js", ".",
                  Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-noext-rel/hypothetical.js", "./",
                  Y("resolve/mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-bare/hypothetical.js", ".",
                  Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-bare/hypothetical.js", "./",
                  Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-bare/main-js/hypothetical.js", "..",
                  Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-bare/main-js/hypothetical.js", "../",
                  Y("resolve/mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-rel/hypothetical.js", ".",
                  Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-rel/hypothetical.js", "./",
                  Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-rel/main-js/hypothetical.js", "..",
                  Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);
    assert_resolves("resolve/mod-js-dir-rel/main-js/hypothetical.js", "../",
                  Y("resolve/mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves("resolve/mod-json-ext-bare/hypothetical.js", ".",
                  Y("resolve/mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-ext-bare/hypothetical.js", "./",
                  Y("resolve/mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-ext-rel/hypothetical.js", ".",
                  Y("resolve/mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-ext-rel/hypothetical.js", "./",
                  Y("resolve/mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-noext-bare/hypothetical.js", ".",
                  Y("resolve/mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-noext-bare/hypothetical.js", "./",
                  Y("resolve/mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-noext-rel/hypothetical.js", ".",
                  Y("resolve/mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-noext-rel/hypothetical.js", "./",
                  Y("resolve/mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves("resolve/mod-json-dir-bare/hypothetical.js", ".",
                  Y("resolve/mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves("resolve/mod-json-dir-bare/hypothetical.js", "./",
                  Y("resolve/mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves("resolve/mod-json-dir-rel/hypothetical.js", ".",
                  Y("resolve/mod-json-dir-rel/main-json/index.json"), &cjs);
    assert_resolves("resolve/mod-json-dir-rel/hypothetical.js", "./",
                  Y("resolve/mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves("resolve/mod-mjs-ext-bare/hypothetical.js", ".",
                  Y("resolve/mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-ext-bare/hypothetical.js", "./",
                  Y("resolve/mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-ext-rel/hypothetical.js", ".",
                  Y("resolve/mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-ext-rel/hypothetical.js", "./",
                  Y("resolve/mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-noext-bare/hypothetical.js", ".",
                  Y("resolve/mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-noext-bare/hypothetical.js", "./",
                  Y("resolve/mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-noext-rel/hypothetical.js", ".",
                  Y("resolve/mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-noext-rel/hypothetical.js", "./",
                  Y("resolve/mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-bare/hypothetical.js", ".",
                  Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-bare/hypothetical.js", "./",
                  Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-bare/main-mjs/hypothetical.js", "..",
                  Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-bare/main-mjs/hypothetical.js", "../",
                  Y("resolve/mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-rel/hypothetical.js", ".",
                  Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-rel/hypothetical.js", "./",
                  Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-rel/main-mjs/hypothetical.js", "..",
                  Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);
    assert_resolves("resolve/mod-mjs-dir-rel/main-mjs/hypothetical.js", "../",
                  Y("resolve/mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves("resolve/mod-js-slash-bare/hypothetical.js", ".",
                  Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-bare/hypothetical.js", "./",
                  Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-bare/main/hypothetical.js", "..",
                  Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-bare/main/hypothetical.js", "../",
                  Y("resolve/mod-js-slash-bare/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-rel/hypothetical.js", ".",
                  Y("resolve/mod-js-slash-rel/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-rel/hypothetical.js", "./",
                  Y("resolve/mod-js-slash-rel/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-rel/main/hypothetical.js", "..",
                  Y("resolve/mod-js-slash-rel/main.js"), &cjs);
    assert_resolves("resolve/mod-js-slash-rel/main/hypothetical.js", "../",
                  Y("resolve/mod-js-slash-rel/main.js"), &cjs);

    assert_resolves("resolve/file-and-dir/hypothetical.js", ".",
                  Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves("resolve/file-and-dir/hypothetical.js", "./",
                  Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves("resolve/file-and-dir/subdir/hypothetical.js", "..",
                  Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves("resolve/file-and-dir/subdir/hypothetical.js", "../",
                  Y("resolve/file-and-dir/index.js"), &cjs);
    assert_resolves("resolve/file-and-mod/hypothetical.js", ".",
                  Y("resolve/file-and-mod/main.js"), &cjs);
    assert_resolves("resolve/file-and-mod/hypothetical.js", "./",
                  Y("resolve/file-and-mod/main.js"), &cjs);
    assert_resolves("resolve/file-and-mod/subdir/hypothetical.js", "..",
                  Y("resolve/file-and-mod/main.js"), &cjs);
    assert_resolves("resolve/file-and-mod/subdir/hypothetical.js", "../",
                  Y("resolve/file-and-mod/main.js"), &cjs);

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx, "./file-and-dir/submod",
                 Y("resolve/file-and-dir.js"), &cjs);
    assert_resolves(ctx, "./file-and-dir/submod/",
                 Y("resolve/file-and-dir.js"), &cjs);
    assert_resolves(ctx, "./file-and-mod/submod",
                 Y("resolve/file-and-mod.js"), &cjs);
    assert_resolves(ctx, "./file-and-mod/submod/",
                 Y("resolve/file-and-mod.js"), &cjs);

    // absolute paths

    let ctx = "resolve/subdir/hypothetical.js";
    let mut path = fixture_path();
    path.push("resolve/named-js");
    assert_resolves(ctx, path.to_str().unwrap(),
                  Y("resolve/named-js.js"), &cjs);

    // modules

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx,          "n-named-noext",
           Y("resolve/node_modules/n-named-noext"), &cjs);
    assert_resolves(ctx,          "n-named-js.js",
           Y("resolve/node_modules/n-named-js.js"), &cjs);
    assert_resolves(ctx,          "n-named-json.json",
           Y("resolve/node_modules/n-named-json.json"), &cjs);
    assert_resolves(ctx,          "n-named-mjs.mjs",
           Y("resolve/node_modules/n-named-mjs.mjs"), &esm);
    assert_resolves(ctx,          "n-named-jsz.jsz",
           Y("resolve/node_modules/n-named-jsz.jsz"), &cjs);

    assert_resolves(ctx,          "n-named-js",
           Y("resolve/node_modules/n-named-js.js"), &cjs);
    assert_resolves(ctx,          "n-named-json",
           Y("resolve/node_modules/n-named-json.json"), &cjs);
    assert_resolves(ctx,          "n-named-mjs",
           Y("resolve/node_modules/n-named-mjs.mjs"), &esm);

    assert_resolves(ctx,          "n-dir-js",
           Y("resolve/node_modules/n-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "n-dir-js/index",
           Y("resolve/node_modules/n-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "n-dir-json",
           Y("resolve/node_modules/n-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "n-dir-json/index",
           Y("resolve/node_modules/n-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "n-dir-mjs",
           Y("resolve/node_modules/n-dir-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "n-dir-mjs/index",
           Y("resolve/node_modules/n-dir-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "n-mod-noext-bare",
           Y("resolve/node_modules/n-mod-noext-bare/main-noext"), &cjs);
    assert_resolves(ctx,          "n-mod-noext-rel",
           Y("resolve/node_modules/n-mod-noext-rel/main-noext"), &cjs);

    assert_resolves(ctx,          "n-mod-main-nesting-bare",
           Y("resolve/node_modules/n-mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "n-mod-main-nesting-bare/subdir",
           Y("resolve/node_modules/n-mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves(ctx,          "n-mod-main-nesting-rel",
           Y("resolve/node_modules/n-mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "n-mod-main-nesting-rel/subdir",
           Y("resolve/node_modules/n-mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves(ctx,          "n-mod-js-ext-bare",
           Y("resolve/node_modules/n-mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-ext-rel",
           Y("resolve/node_modules/n-mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-noext-bare",
           Y("resolve/node_modules/n-mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-noext-rel",
           Y("resolve/node_modules/n-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-dir-bare",
           Y("resolve/node_modules/n-mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-dir-rel",
           Y("resolve/node_modules/n-mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves(ctx,          "n-mod-json-ext-bare",
           Y("resolve/node_modules/n-mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "n-mod-json-ext-rel",
           Y("resolve/node_modules/n-mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "n-mod-json-noext-bare",
           Y("resolve/node_modules/n-mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "n-mod-json-noext-rel",
           Y("resolve/node_modules/n-mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "n-mod-json-dir-bare",
           Y("resolve/node_modules/n-mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves(ctx,          "n-mod-json-dir-rel",
           Y("resolve/node_modules/n-mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves(ctx,          "n-mod-mjs-ext-bare",
           Y("resolve/node_modules/n-mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "n-mod-mjs-ext-rel",
           Y("resolve/node_modules/n-mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "n-mod-mjs-noext-bare",
           Y("resolve/node_modules/n-mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "n-mod-mjs-noext-rel",
           Y("resolve/node_modules/n-mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "n-mod-mjs-dir-bare",
           Y("resolve/node_modules/n-mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "n-mod-mjs-dir-rel",
           Y("resolve/node_modules/n-mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "n-mod-js-slash-bare",
           Y("resolve/node_modules/n-mod-js-slash-bare/main.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-slash-rel",
           Y("resolve/node_modules/n-mod-js-slash-rel/main.js"), &cjs);

    assert_resolves(ctx,          "n-named-jsz", Fail, &cjs);

    assert_resolves(ctx,          "n-file-and-dir",
           Y("resolve/node_modules/n-file-and-dir.js"), &cjs);
    assert_resolves(ctx,          "n-file-and-dir/",
           Y("resolve/node_modules/n-file-and-dir/index.js"), &cjs);
    assert_resolves(ctx,          "n-file-and-mod",
           Y("resolve/node_modules/n-file-and-mod.js"), &cjs);
    assert_resolves(ctx,          "n-file-and-mod/",
           Y("resolve/node_modules/n-file-and-mod/main.js"), &cjs);
    assert_resolves(ctx,          "n-dir-js/",
           Y("resolve/node_modules/n-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "n-mod-js-noext-rel/",
           Y("resolve/node_modules/n-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "n-named-js.js/", Fail, &cjs);
    assert_resolves(ctx,          "n-named-js/", Fail, &cjs);
    assert_resolves(ctx,          "n-named-noext/", Fail, &cjs);

    assert_resolves(ctx,          "./n-named-noext", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-js.js", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-json.json", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-mjs.mjs", Fail, &esm);
    assert_resolves(ctx,          "./n-named-jsz.jsz", Fail, &cjs);

    assert_resolves(ctx,          "./n-named-js", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-json", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-mjs", Fail, &esm);

    assert_resolves(ctx,          "./n-dir-js", Fail, &cjs);
    assert_resolves(ctx,          "./n-dir-js/index", Fail, &cjs);
    assert_resolves(ctx,          "./n-dir-json", Fail, &cjs);
    assert_resolves(ctx,          "./n-dir-json/index", Fail, &cjs);
    assert_resolves(ctx,          "./n-dir-mjs", Fail, &esm);
    assert_resolves(ctx,          "./n-dir-mjs/index", Fail, &esm);

    assert_resolves(ctx,          "./n-mod-noext-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-noext-rel", Fail, &cjs);

    assert_resolves(ctx,          "./n-mod-main-nesting-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-main-nesting-bare/subdir", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-main-nesting-rel", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-main-nesting-rel/subdir", Fail, &cjs);

    assert_resolves(ctx,          "./n-mod-js-ext-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-ext-rel", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-noext-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-noext-rel", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-dir-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-dir-rel", Fail, &cjs);

    assert_resolves(ctx,          "./n-mod-json-ext-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-json-ext-rel", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-json-noext-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-json-noext-rel", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-json-dir-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-json-dir-rel", Fail, &cjs);

    assert_resolves(ctx,          "./n-mod-mjs-ext-bare", Fail, &esm);
    assert_resolves(ctx,          "./n-mod-mjs-ext-rel", Fail, &esm);
    assert_resolves(ctx,          "./n-mod-mjs-noext-bare", Fail, &esm);
    assert_resolves(ctx,          "./n-mod-mjs-noext-rel", Fail, &esm);
    assert_resolves(ctx,          "./n-mod-mjs-dir-bare", Fail, &esm);
    assert_resolves(ctx,          "./n-mod-mjs-dir-rel", Fail, &esm);

    assert_resolves(ctx,          "./n-mod-js-slash-bare", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-slash-rel", Fail, &cjs);

    assert_resolves(ctx,          "./n-named-jsz", Fail, &cjs);

    assert_resolves(ctx,          "./n-file-and-dir", Fail, &cjs);
    assert_resolves(ctx,          "./n-file-and-dir/", Fail, &cjs);
    assert_resolves(ctx,          "./n-file-and-mod", Fail, &cjs);
    assert_resolves(ctx,          "./n-file-and-mod/", Fail, &cjs);
    assert_resolves(ctx,          "./n-dir-js/", Fail, &cjs);
    assert_resolves(ctx,          "./n-mod-js-noext-rel/", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-js.js/", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-js/", Fail, &cjs);
    assert_resolves(ctx,          "./n-named-noext/", Fail, &cjs);

    assert_resolves(ctx,          "shadowed",
           Y("resolve/node_modules/shadowed/index.js"), &cjs);

    assert_resolves(ctx,          "@user/scoped",
           Y("resolve/node_modules/@user/scoped/index.js"), &cjs);
    assert_resolves(ctx,          "@user/scoped/index",
           Y("resolve/node_modules/@user/scoped/index.js"), &cjs);
    assert_resolves(ctx,          "@user/scoped/index.js",
           Y("resolve/node_modules/@user/scoped/index.js"), &cjs);

    assert_resolves(ctx,          "shallow/s-named-noext",
           Y("resolve/node_modules/shallow/s-named-noext"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-js.js",
           Y("resolve/node_modules/shallow/s-named-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-json.json",
           Y("resolve/node_modules/shallow/s-named-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-mjs.mjs",
           Y("resolve/node_modules/shallow/s-named-mjs.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-named-jsz.jsz",
           Y("resolve/node_modules/shallow/s-named-jsz.jsz"), &cjs);

    assert_resolves(ctx,          "shallow/s-named-js",
           Y("resolve/node_modules/shallow/s-named-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-json",
           Y("resolve/node_modules/shallow/s-named-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-mjs",
           Y("resolve/node_modules/shallow/s-named-mjs.mjs"), &esm);

    assert_resolves(ctx,          "shallow/s-dir-js",
           Y("resolve/node_modules/shallow/s-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-dir-js/index",
           Y("resolve/node_modules/shallow/s-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-dir-json",
           Y("resolve/node_modules/shallow/s-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-dir-json/index",
           Y("resolve/node_modules/shallow/s-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-dir-mjs",
           Y("resolve/node_modules/shallow/s-dir-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-dir-mjs/index",
           Y("resolve/node_modules/shallow/s-dir-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "shallow/s-mod-noext-bare",
           Y("resolve/node_modules/shallow/s-mod-noext-bare/main-noext"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-noext-rel",
           Y("resolve/node_modules/shallow/s-mod-noext-rel/main-noext"), &cjs);

    assert_resolves(ctx,          "shallow/s-mod-main-nesting-bare",
           Y("resolve/node_modules/shallow/s-mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-main-nesting-bare/subdir",
           Y("resolve/node_modules/shallow/s-mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-main-nesting-rel",
           Y("resolve/node_modules/shallow/s-mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-main-nesting-rel/subdir",
           Y("resolve/node_modules/shallow/s-mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves(ctx,          "shallow/s-mod-js-ext-bare",
           Y("resolve/node_modules/shallow/s-mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-ext-rel",
           Y("resolve/node_modules/shallow/s-mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-noext-bare",
           Y("resolve/node_modules/shallow/s-mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-noext-rel",
           Y("resolve/node_modules/shallow/s-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-dir-bare",
           Y("resolve/node_modules/shallow/s-mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-dir-rel",
           Y("resolve/node_modules/shallow/s-mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves(ctx,          "shallow/s-mod-json-ext-bare",
           Y("resolve/node_modules/shallow/s-mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-json-ext-rel",
           Y("resolve/node_modules/shallow/s-mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-json-noext-bare",
           Y("resolve/node_modules/shallow/s-mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-json-noext-rel",
           Y("resolve/node_modules/shallow/s-mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-json-dir-bare",
           Y("resolve/node_modules/shallow/s-mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-json-dir-rel",
           Y("resolve/node_modules/shallow/s-mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves(ctx,          "shallow/s-mod-mjs-ext-bare",
           Y("resolve/node_modules/shallow/s-mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-mod-mjs-ext-rel",
           Y("resolve/node_modules/shallow/s-mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-mod-mjs-noext-bare",
           Y("resolve/node_modules/shallow/s-mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-mod-mjs-noext-rel",
           Y("resolve/node_modules/shallow/s-mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-mod-mjs-dir-bare",
           Y("resolve/node_modules/shallow/s-mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "shallow/s-mod-mjs-dir-rel",
           Y("resolve/node_modules/shallow/s-mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "shallow/s-mod-js-slash-bare",
           Y("resolve/node_modules/shallow/s-mod-js-slash-bare/main.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-slash-rel",
           Y("resolve/node_modules/shallow/s-mod-js-slash-rel/main.js"), &cjs);

    assert_resolves(ctx,          "shallow/s-named-jsz", Fail, &cjs);

    assert_resolves(ctx,          "shallow/s-file-and-dir",
           Y("resolve/node_modules/shallow/s-file-and-dir.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-file-and-dir/",
           Y("resolve/node_modules/shallow/s-file-and-dir/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-file-and-mod",
           Y("resolve/node_modules/shallow/s-file-and-mod.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-file-and-mod/",
           Y("resolve/node_modules/shallow/s-file-and-mod/main.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-dir-js/",
           Y("resolve/node_modules/shallow/s-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-mod-js-noext-rel/",
           Y("resolve/node_modules/shallow/s-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "shallow/s-named-js.js/", Fail, &cjs);
    assert_resolves(ctx,          "shallow/s-named-js/", Fail, &cjs);
    assert_resolves(ctx,          "shallow/s-named-noext/", Fail, &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-noext",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-noext"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-js.js",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-json.json",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-mjs.mjs",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-mjs.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-jsz.jsz",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-jsz.jsz"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-js",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-json",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-mjs",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-named-mjs.mjs"), &esm);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-js",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-js/index",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-json",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-json/index",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-json/index.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-mjs",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-mjs/index",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-noext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-noext-bare/main-noext"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-noext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-noext-rel/main-noext"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-main-nesting-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-main-nesting-bare/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-main-nesting-bare/subdir",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-main-nesting-bare/subdir/inner-main.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-main-nesting-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-main-nesting-rel/subdir/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-main-nesting-rel/subdir",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-main-nesting-rel/subdir/inner-main.js"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-ext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-ext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-ext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-ext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-noext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-noext-bare/main-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-noext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-dir-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-dir-bare/main-js/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-dir-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-dir-rel/main-js/index.js"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-ext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-ext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-ext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-ext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-noext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-noext-bare/main-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-noext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-noext-rel/main-json.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-dir-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-dir-bare/main-json/index.json"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-json-dir-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-json-dir-rel/main-json/index.json"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-ext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-ext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-ext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-ext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-noext-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-noext-bare/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-noext-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-noext-rel/main-mjs.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-dir-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-dir-bare/main-mjs/index.mjs"), &esm);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-mjs-dir-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-mjs-dir-rel/main-mjs/index.mjs"), &esm);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-slash-bare",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-slash-bare/main.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-slash-rel",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-slash-rel/main.js"), &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-jsz", Fail, &cjs);

    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-file-and-dir",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-file-and-dir.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-file-and-dir/",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-file-and-dir/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-file-and-mod",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-file-and-mod.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-file-and-mod/",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-file-and-mod/main.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-dir-js/",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-dir-js/index.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-mod-js-noext-rel/",
           Y("resolve/node_modules/deep/dir1/dir2/dir3/d-mod-js-noext-rel/main-js.js"), &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-js.js/", Fail, &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-js/", Fail, &cjs);
    assert_resolves(ctx,          "deep/dir1/dir2/dir3/d-named-noext/", Fail, &cjs);

    let ctx = "resolve/subdir/hypothetical.js";
    assert_resolves(ctx,                 "shadowed",
            Y("resolve/subdir/node_modules/shadowed/index.js"), &cjs);

    let ctx = "resolve/subdir/subdir2/hypothetical.js";
    assert_resolves(ctx,                          "shadowed",
            Y("resolve/subdir/subdir2/node_modules/shadowed/index.js"), &cjs);

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx,  "./dotfiles", Fail, &cjs);
    assert_resolves(ctx,  "./dotfiles/", Fail, &esm);

    assert_resolves(ctx,  "./dotfiles/.thing",
                  Y("resolve/dotfiles/.thing"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.thing-js",
                  Y("resolve/dotfiles/.thing-js.js"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.thing-js.js",
                  Y("resolve/dotfiles/.thing-js.js"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.thing-json",
                  Y("resolve/dotfiles/.thing-json.json"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.thing-json.json",
                  Y("resolve/dotfiles/.thing-json.json"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.thing-mjs",
                  Y("resolve/dotfiles/.thing-mjs.mjs"), &esm);
    assert_resolves(ctx,  "./dotfiles/.thing-mjs.mjs",
                  Y("resolve/dotfiles/.thing-mjs.mjs"), &esm);

    assert_resolves(ctx,  "./dotfiles/.js",
                  Y("resolve/dotfiles/.js"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.json",
                  Y("resolve/dotfiles/.json"), &cjs);
    assert_resolves(ctx,  "./dotfiles/.mjs",
                  Y("resolve/dotfiles/.mjs"), &esm);

    assert_resolves(ctx,  "./dotfiles/mod-noext",
                  Y("resolve/dotfiles/mod-noext/.thing"), &cjs);
    assert_resolves(ctx,  "./dotfiles/mod-js",
                  Y("resolve/dotfiles/mod-js/.thing-js.js"), &cjs);
    assert_resolves(ctx,  "./dotfiles/mod-json",
                  Y("resolve/dotfiles/mod-json/.thing-json.json"), &cjs);
    assert_resolves(ctx,  "./dotfiles/mod-mjs",
                  Y("resolve/dotfiles/mod-mjs/.thing-mjs.mjs"), &esm);

    let ctx = "resolve-order/hypothetical.js";
    assert_resolves(ctx,  "./1-file",
            Y("resolve-order/1-file"), &cjs);
    assert_resolves(ctx,  "./2-file",
            Y("resolve-order/2-file.js"), &cjs);
    assert_resolves(ctx,  "./3-file",
            Y("resolve-order/3-file.json"), &cjs);
    assert_resolves(ctx,  "./1-dir",
            Y("resolve-order/1-dir.js"), &cjs);
    assert_resolves(ctx,  "./2-dir",
            Y("resolve-order/2-dir.json"), &cjs);
    assert_resolves(ctx,  "./3-dir",
            Y("resolve-order/3-dir/index.js"), &cjs);
    assert_resolves(ctx,  "./4-dir",
            Y("resolve-order/4-dir/index.json"), &cjs);
    assert_resolves(ctx,  "./1-dir/",
            Y("resolve-order/1-dir/index.js"), &cjs);
    assert_resolves(ctx,  "./2-dir/",
            Y("resolve-order/2-dir/index.js"), &cjs);
    assert_resolves(ctx,  "./3-dir/",
            Y("resolve-order/3-dir/index.js"), &cjs);
    assert_resolves(ctx,  "./4-dir/",
            Y("resolve-order/4-dir/index.json"), &cjs);
}

fn test_resolve_unicode_with<F>(mut assert_resolves: F)
where F: FnMut(&str, &str, Resolution<'static>, &InputOptions) {
    let cjs = InputOptions {
        for_browser: false,
        es6_syntax: false,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx,  "./unicode/𝌆",
                  Y("resolve/unicode/𝌆.js"), &cjs);
    assert_resolves(ctx,  "./unicode/𝌆.js",
                  Y("resolve/unicode/𝌆.js"), &cjs);
}

#[test]
fn test_resolve_consistency() {
    // meta-test: ensure test_resolve matches node behavior

    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    enum Target {
        Browserify,
        Webpack,
        Node,
    }
    impl Display for Target {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(match *self {
                Target::Browserify => "browserify",
                Target::Webpack => "webpack",
                Target::Node => "node",
            })
        }
    }

    type Cases = Vec<(String, Resolution<'static>)>;
    type CaseMap = FnvHashMap<String, Cases>;

    let mut cjs = FnvHashMap::default();
    let mut browser = FnvHashMap::default();
    let mut esm = FnvHashMap::default();

    {
        let mut append = |ctx: &str, from: &str, to: Resolution<'static>, input_options: &InputOptions| {
            let assertions = if input_options.for_browser {
                &mut browser
            } else if input_options.es6_syntax {
                &mut esm
            } else {
                &mut cjs
            };
            assertions.entry(ctx.to_owned())
                .or_insert_with(Vec::default)
                .push((from.to_owned(), to));
        };

        // test_resolve_with(&mut append);
        // test_resolve_unicode_with(&mut append);
        test_browser_with(&mut append);
    }

    fn make_source(base: &Path, target: Target, cases: &Cases) -> Vec<u8> {
        // browser tests use module.exports = __filename
        let mut b = if target == Target::Node { indoc!(br#"
            'use strict'
            const assert = require('assert').strict
            let success = true
            function n(from) {
                let fail = false, to
                try {to = require.resolve(from), fail = true} catch(_) {}
                if (fail) {
                    console.error(`failed:\n  '${from}'\ndoes not fail to resolve; it resolved to:\n  '${to}'\n`)
                    success = false
                }
            }
            function y(from, to) {
                let realTo
                try {
                    assert.equal(realTo = require.resolve(from), to)
                } catch (e) {
                    console.error(`failed:\n  '${from}'\ndoes not resolve to:\n  '${to}'\nit resolved to:\n  '${realTo}'\n`)
                    console.error(e.stack)
                    success = false
                }
            }
        "#).to_vec() } else { indoc!(br#"
            'use strict'
            let success = true
            function y(real, from, to) {
                if (real && real.startsWith('.')) real = real.slice(1)
                if (real && !real.startsWith('/')) real = '/' + real
                if (real !== to) {
                    console.error(`failed:\n  '${from}'\ndoes not resolve to:\n  '${to}'\nit resolved to:\n  '${real}'\n`)
                    success = false
                }
            }
            function i(real, realTo, from) {
                if (Object.prototype.toString.call(real) !== '[object Object]' || Object.keys(real).length !== 0) {
                    console.error(`failed:\n  '${from}'\nis not ignored; it resolved to:\n  '${realTo}'\n`)
                    success = false
                }
            }
            function n(real, from) {
                if (real && real.startsWith('.')) real = real.slice(1)
                if (real && !real.startsWith('/')) real = '/' + real
                if (real) {
                    console.error(`failed:\n  '${from}'\ndoes not fail to resolve; it resolved to:\n  '${real}'\n`)
                    success = false
                }
            }
        "#).to_vec() };
        for (from, to) in cases {
            let from_path = Path::new(from);
            let from = if from_path.is_absolute() {
                let suffix = from_path.strip_prefix(fixture_path()).expect("absolute path outside of fixtures");
                let mut from = base.join(suffix.parent().unwrap()).canonicalize().unwrap();
                from.push(suffix.file_name().unwrap());
                serde_json::to_string(&from)
            } else {
                serde_json::to_string(&from)
            }.unwrap();
            match to {
                Y(to) => {
                    let mut to_path = base.to_owned();
                    to_path.append_resolving(to);
                    let to = serde_json::to_string(to_path.canonicalize().unwrap().to_str().unwrap()).unwrap();
                    match target {
                        Target::Browserify => {
                            writeln!(b, "y(require({from}), {from}, {to})", from=from, to=to).unwrap();
                        }
                        Target::Webpack => {
                            writeln!(b, "y(require.resolve({from}), {from}, {to})", from=from, to=to).unwrap();
                        }
                        Target::Node => {
                            writeln!(b, "y({from}, {to})", from=from, to=to).unwrap();
                        }
                    }
                }
                Ignore => match target {
                    Target::Browserify => {
                        writeln!(b, "i(require({from}), require({from}), {from})", from=from).unwrap();
                    }
                    Target::Webpack => {
                        writeln!(b, "i(require({from}), require.resolve({from}), {from})", from=from).unwrap();
                    }
                    Target::Node => {
                        panic!("ignore tests are invalid for node target");
                    }
                },
                External => unimplemented!(),
                Fail => match target {
                    Target::Browserify => {
                        writeln!(b, "n(require({from}), {from})", from=from).unwrap();
                    }
                    Target::Webpack => {
                        writeln!(b, "n(require.resolve({from}), {from})", from=from).unwrap();
                    }
                    Target::Node => {
                        writeln!(b, "n({from})", from=from).unwrap();
                    }
                },
            }
        }
        writeln!(b, r#"process.nextTick(() => {{if (!success) throw new Error('failed: {} consistency')}})"#, target).unwrap();
        // io::stdout().write_all(&b).unwrap();
        b
    }
    fn test_file(base: &Path, esm: bool, target: Target, ctx: &str, cases: &Cases) {
        let mut ctx_dir = base.to_owned();
        ctx_dir.append_resolving(ctx);
        ctx_dir.pop();
        // let ext = if esm { ".mjs" } else { ".js" };
        let ext = ".js";

        let mut file = tempfile::Builder::new()
            .suffix(ext)
            .tempfile_in(&ctx_dir)
            .unwrap();
        file.as_file_mut()
            .write_all(&make_source(base, target, cases))
            .unwrap();

        let path = file.path().canonicalize().unwrap();
        let path = path.to_str().unwrap();
        let output;
        if target != Target::Node {
            let mut to_file = tempfile::Builder::new()
                .suffix(ext)
                .tempfile_in(&ctx_dir)
                .unwrap();
            let to_path = to_file.path().canonicalize().unwrap();
            let to_path = to_path.to_str().unwrap();
            let to_write = to_file.as_file_mut();

            let mut tools_path = fixture_path();
            tools_path.append_resolving("tools");
            // let tools_path = base.join("tools");

            match target {
                Target::Webpack => {
                    let webpack_path = tools_path.join("node_modules/.bin/webpack");
                    let config_path = tools_path.join("webpack.config.js");
                    let ok = process::Command::new(webpack_path)
                        .current_dir("/")
                        .args(&["--config", &config_path.to_str().unwrap(), &path, "-o", &to_path])
                        .status()
                        .expect("failed to run webpack")
                        .success();
                    if !ok {
                        panic!("webpack failed");
                    }
                },
                Target::Browserify => {
                    to_write
                        .write_all(indoc!(br#"
                            require = function() {};
                        "#))
                        .unwrap();

                    let browserify_path = tools_path.join("node_modules/.bin/browserify");
                    let mut browserify = process::Command::new(browserify_path)
                        .current_dir("/")
                        .stderr(process::Stdio::inherit())
                        .stdout(process::Stdio::piped())
                        .args(&["--ignore-missing", "--no-commondir", &path]) // , "-o", &to_path
                        .spawn()
                        .expect("failed to start browserify");
                    io::copy(browserify.stdout.as_mut().unwrap(), to_write).unwrap();
                    let ok = browserify
                        .wait()
                        .expect("failed to run browserify")
                        .success();
                    if !ok {
                        panic!("browserify failed");
                    }
                }
                Target::Node => unreachable!(),
            }

            fs::copy(&to_path, "/tmp/test.js").unwrap();
            output = process::Command::new("node")
                .args(&[&to_path])
                .output()
                .expect("failed to run node");
        } else {
            let mut args = Vec::new();
            if esm {
                args.push("--experimental-modules");
            }
            args.push(path);
            output = process::Command::new("node")
                .args(&args)
                .output()
                .expect("failed to run node");
        }

        if !output.status.success() {
            io::stderr().write(&output.stderr).unwrap();
            panic!("tests are inconsistent with node/browserify");
        }
    }
    fn test_file_map(base: &Path, esm: bool, target: Target, map: &CaseMap) {
        for (ctx, cases) in map.into_iter() {
            test_file(base, esm, target, ctx, cases)
        }
    }

    let base_dir = tempfile::tempdir().unwrap();
    let fixture_dir = fixture_path();
    for entry in WalkDir::new(&fixture_dir)
        .into_iter()
        .filter_map(Result::ok) {
        let local_path = entry.path().strip_prefix(&fixture_dir).unwrap();
        if local_path.components().next().is_none() { continue }

        let new_path = base_dir.path().join(local_path);
        // println!("{} {}", entry.path().display(), new_path.display());
        if !local_path.starts_with("tools/node_modules") {
            if entry.file_type().is_dir() {
                fs::create_dir(new_path).unwrap();
            } else {
                fs::copy(entry.path(), new_path).unwrap();
            }
        }
    }
    // npm_install(&base_dir.path().join("tools"));
    npm_install(&fixture_dir.join("tools"));
    test_file_map(base_dir.path(), false, Target::Node, &cjs);
    test_file_map(base_dir.path(), false, Target::Webpack, &browser);
    if false {
        test_file_map(base_dir.path(), false, Target::Browserify, &browser);
    }
    test_file_map(base_dir.path(), true, Target::Node, &esm);
}

#[test]
fn test_browser() {
    test_browser_with(assert_resolves);
}
fn test_browser_with<F>(mut assert_resolves: F)
where F: FnMut(&str, &str, Resolution<'static>, &InputOptions) {
    let no = InputOptions {
        for_browser: false,
        es6_syntax: true,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };
    let br = InputOptions {
        for_browser: true,
        es6_syntax: true,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };

    let ctx = "browser/hypothetical.js";
    assert_resolves(ctx,  "./alternate-main-rel",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-rel/main-default",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-rel/main-default.js",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-bare",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-bare/main-default",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-bare/main-default.js",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-rel",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-rel/index",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-rel/index.js",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-bare",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-bare/index",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-index-bare/index.js",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-rel",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-rel/default/main",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-rel/default/main.js",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-bare",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-bare/default/main",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-subdir-bare/default/main.js",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,  "./alternate-main-rel",
                  Y("browser/alternate-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-main-rel/main-default",
                  Y("browser/alternate-main-rel/main-default.js"), &br);
    assert_resolves(ctx,  "./alternate-main-rel/main-default.js",
                  Y("browser/alternate-main-rel/main-default.js"), &br);
    assert_resolves(ctx,  "./alternate-main-bare",
                  Y("browser/alternate-main-bare/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-main-bare/main-default",
                  Y("browser/alternate-main-bare/main-default.js"), &br);
    assert_resolves(ctx,  "./alternate-main-bare/main-default.js",
                  Y("browser/alternate-main-bare/main-default.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-rel",
                  Y("browser/alternate-main-index-rel/index-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-rel/index",
                  Y("browser/alternate-main-index-rel/index.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-rel/index.js",
                  Y("browser/alternate-main-index-rel/index.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-bare",
                  Y("browser/alternate-main-index-bare/index-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-bare/index",
                  Y("browser/alternate-main-index-bare/index.js"), &br);
    assert_resolves(ctx,  "./alternate-main-index-bare/index.js",
                  Y("browser/alternate-main-index-bare/index.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-rel",
                  Y("browser/alternate-main-subdir-rel/browser/main.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-rel/default/main",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-rel/default/main.js",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-bare",
                  Y("browser/alternate-main-subdir-bare/browser/main.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-bare/default/main",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &br);
    assert_resolves(ctx,  "./alternate-main-subdir-bare/default/main.js",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &br);
    let ctx = "browser/alternate-main-rel/hypothetical.js";
    assert_resolves(ctx,                     ".",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                     "./main-default",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                     "./main-default.js",
                  Y("browser/alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                     ".",
                  Y("browser/alternate-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,                     "./main-default.js",
                  Y("browser/alternate-main-rel/main-default.js"), &br);
    let ctx = "browser/alternate-main-bare/hypothetical.js";
    assert_resolves(ctx,                      ".",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                      "./main-default.js",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                      "./main-default",
                  Y("browser/alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                      ".",
                  Y("browser/alternate-main-bare/main-browser.js"), &br);
    assert_resolves(ctx,                      "./main-default",
                  Y("browser/alternate-main-bare/main-default.js"), &br);
    assert_resolves(ctx,                      "./main-default.js",
                  Y("browser/alternate-main-bare/main-default.js"), &br);
    let ctx = "browser/alternate-main-index-rel/hypothetical.js";
    assert_resolves(ctx,                           ".",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,                           "./index",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,                           "./index.js",
                  Y("browser/alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,                           ".",
                  Y("browser/alternate-main-index-rel/index-browser.js"), &br);
    assert_resolves(ctx,                           "./index.js",
                  Y("browser/alternate-main-index-rel/index.js"), &br);
    let ctx = "browser/alternate-main-index-bare/hypothetical.js";
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,                            "./index.js",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,                            "./index",
                  Y("browser/alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-main-index-bare/index-browser.js"), &br);
    assert_resolves(ctx,                            "./index",
                  Y("browser/alternate-main-index-bare/index.js"), &br);
    assert_resolves(ctx,                            "./index.js",
                  Y("browser/alternate-main-index-bare/index.js"), &br);
    let ctx = "browser/alternate-main-subdir-rel/hypothetical.js";
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,                            "./default/main",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,                            "./default/main.js",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-main-subdir-rel/browser/main.js"), &br);
    assert_resolves(ctx,                            "./default/main.js",
                  Y("browser/alternate-main-subdir-rel/default/main.js"), &br);
    let ctx = "browser/alternate-main-subdir-bare/hypothetical.js";
    assert_resolves(ctx,                             ".",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,                             "./default/main.js",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,                             "./default/main",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,                             ".",
                  Y("browser/alternate-main-subdir-bare/browser/main.js"), &br);
    assert_resolves(ctx,                             "./default/main",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &br);
    assert_resolves(ctx,                             "./default/main.js",
                  Y("browser/alternate-main-subdir-bare/default/main.js"), &br);

    let ctx = "browser/hypothetical.js";
    assert_resolves(ctx,  "./alternate-files-main-rel",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-rel/main-default",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-rel/main-default.js",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-bare",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-bare/main-default",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-bare/main-default.js",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files-main-rel",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files-main-rel/main-default",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files-main-rel/main-default.js",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files-main-bare",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files-main-bare/main-default",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files-main-bare/main-default.js",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);
    let ctx = "browser/alternate-files-main-rel/hypothetical.js";
    assert_resolves(ctx,                           ".",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                           "./main-default",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                           "./main-default.js",
                  Y("browser/alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,                           ".",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,                           "./main-default",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,                           "./main-default.js",
                  Y("browser/alternate-files-main-rel/main-browser.js"), &br);
    let ctx = "browser/alternate-files-main-bare/hypothetical.js";
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                            "./main-default",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                            "./main-default.js",
                  Y("browser/alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,                            ".",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,                            "./main-default",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,                            "./main-default.js",
                  Y("browser/alternate-files-main-bare/node_modules/main-browser.js"), &br);

    let ctx = "browser/hypothetical.js";
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);

    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,  "./alternate-files/file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);

    let ctx = "browser/alternate-files/hypothetical.js";
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);

    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/node_modules/file-from-rel-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/alternate-files/node_modules/file-from-bare-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                  "./file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);

    let ctx = "browser/hypothetical.js";
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &no);

    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-ext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-browser/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-rel-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-rel-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-rel-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-noext-to-bare-ext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-ext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-ext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-ext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-noext-to-bare-noext-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-noext-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-ext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-ext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/node_modules/file-from-bare-noext-to-bare-slash-browser.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default.json",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default.json"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-noext-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/index",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);
    assert_resolves(ctx,                 "n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js",
                  Y("browser/node_modules/n-alternate-files/file-from-bare-slash-to-bare-slash-default/index.js"), &br);

    assert_resolves(ctx,       "n-alternate-main-rel",
        Y("browser/node_modules/n-alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-rel/main-default",
        Y("browser/node_modules/n-alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-rel/main-default.js",
        Y("browser/node_modules/n-alternate-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-bare",
        Y("browser/node_modules/n-alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-bare/main-default",
        Y("browser/node_modules/n-alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-bare/main-default.js",
        Y("browser/node_modules/n-alternate-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-rel",
        Y("browser/node_modules/n-alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-rel/index",
        Y("browser/node_modules/n-alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-rel/index.js",
        Y("browser/node_modules/n-alternate-main-index-rel/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-bare",
        Y("browser/node_modules/n-alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-bare/index",
        Y("browser/node_modules/n-alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-index-bare/index.js",
        Y("browser/node_modules/n-alternate-main-index-bare/index.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel",
        Y("browser/node_modules/n-alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel/default/main",
        Y("browser/node_modules/n-alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel/default/main.js",
        Y("browser/node_modules/n-alternate-main-subdir-rel/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare",
        Y("browser/node_modules/n-alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare/default/main",
        Y("browser/node_modules/n-alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare/default/main.js",
        Y("browser/node_modules/n-alternate-main-subdir-bare/default/main.js"), &no);
    assert_resolves(ctx,       "n-alternate-main-rel",
        Y("browser/node_modules/n-alternate-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-rel/main-default",
        Y("browser/node_modules/n-alternate-main-rel/main-default.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-rel/main-default.js",
        Y("browser/node_modules/n-alternate-main-rel/main-default.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-bare",
        Y("browser/node_modules/n-alternate-main-bare/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-bare/main-default",
        Y("browser/node_modules/n-alternate-main-bare/main-default.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-bare/main-default.js",
        Y("browser/node_modules/n-alternate-main-bare/main-default.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-rel",
        Y("browser/node_modules/n-alternate-main-index-rel/index-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-rel/index",
        Y("browser/node_modules/n-alternate-main-index-rel/index.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-rel/index.js",
        Y("browser/node_modules/n-alternate-main-index-rel/index.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-bare",
        Y("browser/node_modules/n-alternate-main-index-bare/index-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-bare/index",
        Y("browser/node_modules/n-alternate-main-index-bare/index.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-index-bare/index.js",
        Y("browser/node_modules/n-alternate-main-index-bare/index.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel",
        Y("browser/node_modules/n-alternate-main-subdir-rel/browser/main.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel/default/main",
        Y("browser/node_modules/n-alternate-main-subdir-rel/default/main.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-rel/default/main.js",
        Y("browser/node_modules/n-alternate-main-subdir-rel/default/main.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare",
        Y("browser/node_modules/n-alternate-main-subdir-bare/browser/main.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare/default/main",
        Y("browser/node_modules/n-alternate-main-subdir-bare/default/main.js"), &br);
    assert_resolves(ctx,       "n-alternate-main-subdir-bare/default/main.js",
        Y("browser/node_modules/n-alternate-main-subdir-bare/default/main.js"), &br);

    assert_resolves(ctx,       "n-alternate-files-main-rel",
        Y("browser/node_modules/n-alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-rel/main-default",
        Y("browser/node_modules/n-alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-rel/main-default.js",
        Y("browser/node_modules/n-alternate-files-main-rel/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-bare",
        Y("browser/node_modules/n-alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-bare/main-default",
        Y("browser/node_modules/n-alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-bare/main-default.js",
        Y("browser/node_modules/n-alternate-files-main-bare/main-default.js"), &no);
    assert_resolves(ctx,       "n-alternate-files-main-rel",
        Y("browser/node_modules/n-alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-files-main-rel/main-default",
        Y("browser/node_modules/n-alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-files-main-rel/main-default.js",
        Y("browser/node_modules/n-alternate-files-main-rel/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-files-main-bare",
        Y("browser/node_modules/n-alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-files-main-bare/main-default",
        Y("browser/node_modules/n-alternate-files-main-bare/node_modules/main-browser.js"), &br);
    assert_resolves(ctx,       "n-alternate-files-main-bare/main-default.js",
        Y("browser/node_modules/n-alternate-files-main-bare/node_modules/main-browser.js"), &br);

    let ctx = "browser/hypothetical.js";
    assert_resolves(ctx,  "./ignore-files/file-bare-noext",
                  Y("browser/ignore-files/file-bare-noext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-bare-noext.js",
                  Y("browser/ignore-files/file-bare-noext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-bare-ext",
                  Y("browser/ignore-files/file-bare-ext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-bare-ext.js",
                  Y("browser/ignore-files/file-bare-ext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-rel-noext",
                  Y("browser/ignore-files/file-rel-noext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-rel-noext.js",
                  Y("browser/ignore-files/file-rel-noext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-rel-ext",
                  Y("browser/ignore-files/file-rel-ext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-rel-ext.js",
                  Y("browser/ignore-files/file-rel-ext.js"), &no);
    assert_resolves(ctx,  "./ignore-files/file-bare-noext", Ignore, &br);
    assert_resolves(ctx,  "./ignore-files/file-bare-noext.js",
                  Y("browser/ignore-files/file-bare-noext.js"), &br);
    assert_resolves(ctx,  "./ignore-files/file-bare-ext", Ignore, &br);
    assert_resolves(ctx,  "./ignore-files/file-bare-ext.js", Ignore, &br);
    assert_resolves(ctx,  "./ignore-files/file-rel-noext", Ignore, &br);
    assert_resolves(ctx,  "./ignore-files/file-rel-noext.js",
                  Y("browser/ignore-files/file-rel-noext.js"), &br);
    assert_resolves(ctx,  "./ignore-files/file-rel-ext", Ignore, &br);
    assert_resolves(ctx,  "./ignore-files/file-rel-ext.js", Ignore, &br);
}

#[test]
fn test_external() {
    let ext = InputOptions {
        for_browser: false,
        es6_syntax: false,
        es6_syntax_everywhere: false,
        external: vec![
            "external".to_owned(),
            "external-only-module".to_owned(),
        ].into_iter().collect(),
    };
    let non = InputOptions {
        for_browser: false,
        es6_syntax: false,
        es6_syntax_everywhere: false,
        external: Default::default(),
    };

    let ctx = "resolve/hypothetical.js";
    assert_resolves(ctx, "external", External, &ext);
    assert_resolves(ctx, "external/", External, &ext);
    assert_resolves(ctx, "external/file.js", External, &ext);
    assert_resolves(ctx, "external/file", External, &ext);
    assert_resolves(ctx, "external/subdir", External, &ext);
    assert_resolves(ctx, "external/subdir/", External, &ext);
    assert_resolves(ctx, "external/subdir/index.js", External, &ext);
    assert_resolves(ctx, "./external",
                 Y("resolve/external.js"), &ext);

    assert_resolves(ctx, "./external",
                 Y("resolve/external.js"), &non);
    assert_resolves(ctx,       "external",
        Y("resolve/node_modules/external/index.js"), &non);
    assert_resolves(ctx,       "external/",
        Y("resolve/node_modules/external/index.js"), &non);
    assert_resolves(ctx,       "external/file.js",
        Y("resolve/node_modules/external/file.js"), &non);
    assert_resolves(ctx,       "external/file",
        Y("resolve/node_modules/external/file.js"), &non);
    assert_resolves(ctx,       "external/subdir",
        Y("resolve/node_modules/external/subdir/index.js"), &non);
    assert_resolves(ctx,       "external/subdir/index",
        Y("resolve/node_modules/external/subdir/index.js"), &non);
    assert_resolves(ctx,       "external/subdir/index.js",
        Y("resolve/node_modules/external/subdir/index.js"), &non);
}

fn npm_install(dir: &Path) {
    let node_modules = dir.join("node_modules");
    if node_modules.is_dir() { return }

    let ok = process::Command::new("npm")
        .arg("install")
        .arg("--silent")
        // .arg("--verbose")
        .current_dir(dir)
        .status()
        .expect("failed to run `npm install`")
        .success();
    if !ok {
        panic!("`npm install` did not exit successfully");
    }
}

cfg_if! {
    if #[cfg(feature = "bench")] {
        #[bench]
        fn bench_vlq(b: &mut test::Bencher) {
            let mut vlq = Vlq::new();
            b.iter(|| {
                test::black_box(vlq.enc(-1001));
            });
        }

        #[bench]
        fn bench_cjs_simple(b: &mut test::Bencher) {
            let entry_point = Path::new("examples/simple/index.js");
            npm_install(entry_point.parent().unwrap());
            let input_options = InputOptions::default();
            let output = "/dev/null";
            let map_output = SourceMapOutput::Inline;

            b.iter(|| {
                let _ = bundle(&entry_point, input_options, &output, &map_output).unwrap();
            });
        }

        #[bench]
        fn bench_es6_simple(b: &mut test::Bencher) {
            let entry_point = Path::new("examples/es6-simple/index.mjs");
            npm_install(entry_point.parent().unwrap());
            let input_options = InputOptions {
                es6_syntax: true,
                ..InputOptions::default()
            };
            let output = "/dev/null";
            let map_output = SourceMapOutput::Inline;

            b.iter(|| {
                let _ = bundle(&entry_point, input_options, &output, &map_output).unwrap();
            });
        }

        #[bench]
        fn bench_es6_everywhere_simple(b: &mut test::Bencher) {
            let entry_point = Path::new("examples/es6-everywhere-simple/index.js");
            npm_install(entry_point.parent().unwrap());
            let input_options = InputOptions {
                es6_syntax: true,
                es6_syntax_everywhere: true,
                ..InputOptions::default()
            };
            let output = "/dev/null";
            let map_output = SourceMapOutput::Inline;

            b.iter(|| {
                let _ = bundle(&entry_point, input_options, &output, &map_output).unwrap();
            });
        }

        #[bench]
        fn bench_write_map_to(b: &mut test::Bencher) {
            let writer = Writer {
                modules: {
                    let mut modules = FnvHashMap::default();
                    for i in 0..1000 {
                        let mut path = PathBuf::new();
                        path.push(i.to_string());
                        path.push("examples/es6-everywhere-simple/node_modules/itt/index.js");
                        modules.insert(
                            path,
                            Module {
                                source: Source {
                                    prefix: "~function() {".to_owned(),
                                    body: include_str!("itt.js").to_owned(),
                                    suffix: "}()".to_owned(),
                                    original: None,
                                },
                                deps: {
                                    let mut deps = FnvHashMap::new();
                                    deps.insert("./math".to_owned(), Resolved::Normal(
                                        Path::new("examples/es6-everywhere-simple/math.js").to_owned(),
                                    ));
                                    deps.insert("itt".to_owned(), Resolved::Normal(
                                        Path::new("examples/es6-everywhere-simple/node_modules/itt/index.js").to_owned(),
                                    ));
                                    deps
                                },
                            },
                        );
                    }
                    modules
                },
                entry_point: Path::new("examples/es6-everywhere-simple/index.js"),
                map_output: &SourceMapOutput::Inline,
            };

            let mut out = Vec::new();
            b.iter(|| {
                out.clear();
                writer.write_map_to(&mut out).unwrap();
            });
            b.bytes = out.len() as u64;
        }
    }
}
