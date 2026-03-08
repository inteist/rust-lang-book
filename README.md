# The Rust Programming Language

![Build Status](https://github.com/inteist/rust-lang-book/workflows/CI/badge.svg)

This repository contains the source of "The Rust Programming Language" book.

[The book is available in dead-tree form from No Starch Press][nostarch].

[nostarch]: https://nostarch.com/rust-programming-language-2nd-edition

You can also read the book for free online. Please see the book as shipped with
the latest [stable], [beta], or [nightly] Rust releases. Be aware that issues
in those versions may have been fixed in this repository already, as those
releases are updated less frequently.

[stable]: https://doc.rust-lang.org/stable/book/
[beta]: https://doc.rust-lang.org/beta/book/
[nightly]: https://doc.rust-lang.org/nightly/book/

See the [releases] to download the generated epub

[releases]: https://github.com/inteist/rust-lang-book/releases

## Requirements

Building the book requires [mdBook], ideally the same version that
rust-lang/rust uses in [this file][rust-mdbook]. To get it:

[mdBook]: https://github.com/rust-lang/mdBook
[rust-mdbook]: https://github.com/rust-lang/rust/blob/HEAD/src/tools/rustbook/Cargo.toml

```zsh
$ cargo install mdbook --locked --version <version_num>
```

## Building

To build the book, type:

```zsh
$ mdbook build
```

The output will be in the `book` subdirectory. To check it out, open it in
your web browser.

_Firefox:_

```zsh
$ firefox book/index.html                       # Linux
$ open -a "Firefox" book/index.html             # OS X
$ Start-Process "firefox.exe" .\book\index.html # Windows (PowerShell)
$ start firefox.exe .\book\index.html           # Windows (Cmd)
```

_Chrome:_

```zsh
$ google-chrome book/index.html                 # Linux
$ open -a "Google Chrome" book/index.html       # OS X
$ Start-Process "chrome.exe" .\book\index.html  # Windows (PowerShell)
$ start chrome.exe .\book\index.html            # Windows (Cmd)
```

To run the tests:

```zsh
$ cd packages/trpl
$ mdbook test --library-path packages/trpl/target/debug/deps
```

## EPUB generation

To build an EPUB from the current book sources, it is best to use an **all in one** shell script run:

```zsh
$ ./tools/generate-epub.sh
```


This script will:

1. Build mdBook HTML output into `tmp/epub-html`
2. Build the `mdbook_epub` generator
3. Generate `dist/the-rust-programming-language.epub`
4. Validate ZIP integrity of the generated EPUB


**Note** this for adds optional `--codeblock-font-size` parameter to the generator, which can be used to adjust the font size of code blocks in the generated EPUB. For example, to set the code block font size to 0.9 times the normal text size, you can run:

```zsh
$ ./tools/generate-epub.sh --codeblock-font-size 0.8
```

This is useful for improving readability on smaller screens, such as e-readers, where the default font size may be too large for code blocks. Adjust the value as needed to find the optimal font size for your device. This is **the beauty of open source **- you can customize things to match exactly what you need!



If you want to run the generator directly (without the wrapper script), use:

```zsh
# Generator sample command
$ cargo run -p rust-book-tools --bin mdbook_epub -- \
	--book-dir tmp/epub-html \
	--summary src/SUMMARY.md \
	--book-toml book.toml \
	--output dist/the-rust-programming-language.epub \
	--validate
    --codeblock-font-size 0.9
```


### TODOs
- [ ] Make sure the name and metadata have correct information 
- [ ] Add generated EPUB to releases (dependent on the above)


## Contributing

We'd love your help! Please see [CONTRIBUTING.md][contrib] to learn about the
kinds of contributions we're looking for.

[contrib]: https://github.com/rust-lang/book/blob/main/CONTRIBUTING.md

Because the book is [printed][nostarch], and because we want
to keep the online version of the book close to the print version when
possible, it may take longer than you're used to for us to address your issue
or pull request.

So far, we've been doing a larger revision to coincide with [Rust Editions](https://doc.rust-lang.org/edition-guide/). Between those larger
revisions, we will only be correcting errors. If your issue or pull request
isn't strictly fixing an error, it might sit until the next time that we're
working on a large revision: expect on the order of months or years. Thank you
for your patience!

### Translations

We'd love help translating the book! See the [Translations] label to join in
efforts that are currently in progress. Open a new issue to start working on
a new language! We're waiting on [mdbook support] for multiple languages
before we merge any in, but feel free to start!

[Translations]: https://github.com/rust-lang/book/issues?q=is%3Aopen+is%3Aissue+label%3ATranslations
[mdbook support]: https://github.com/rust-lang/mdBook/issues/5

## Spellchecking

To scan source files for spelling errors, you can use the `spellcheck.sh`
script available in the `ci` directory. It needs a dictionary of valid words,
which is provided in `ci/dictionary.txt`. If the script produces a false
positive (say, you used the word `BTreeMap` which the script considers invalid),
you need to add this word to `ci/dictionary.txt` (keep the sorted order for
consistency).
