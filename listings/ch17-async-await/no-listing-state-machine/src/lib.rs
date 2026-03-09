// required for mdbook test
extern crate trpl;

// ANCHOR: enum
enum PageTitleFuture<'a> {
    Initial { url: &'a str },
    GetAwaitPoint { url: &'a str },
    TextAwaitPoint { response: trpl::Response },
}
// ANCHOR_END: enum
