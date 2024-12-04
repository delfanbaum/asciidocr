# asciidocr

A(n in-progress, but more or less functional) Rust tool/library for processing
Asciidoc files.

## Installation

Right now the crate provides an `asciidocr` executable with a default HTML
build as well as a `json` backend that can be used to check against
the still-in-progress asciidoc official spec/technology compatibility toolkit.
An experimental `docx` build is provided behind a feature flag.

To install from `crates.io`:

```console
$ cargo install asciidocr
```

To install from source, clone and `cd` into the repo and run:

```console
$ cargo install --path .
```

To include the experimental docx backend, enable the `docx` feature:

```console
$ cargo install --features "docx" asciidocr
```

```console
$ cargo install --path . --features "docx"
```

## Usage (Command-Line)

Here's the usage with the `docx` feature enabled:

```console
$ asciidocr --help
A CLI and library for processing and converting asciidoc files

Usage: asciidocr [OPTIONS] <FILE>

Arguments:
  <FILE>
          Asciidoc file for processing. To read from standard input (stdin), use "-"

Options:
  -o, --out-file <OUTPUT>
          Optionally provide a filename for the output. To send to standard out (stdout), use "-"

  -b, --backend <BACKEND>
          Optionally select a backend for conversion
          
          [default: htmlbook]

          Possible values:
          - htmlbook: Produces "Htmlbook-like" HTML documents
          - docx:     !Experimental! Produces a "manuscript-styled" DOCX document
          - json:     Produces the Abstract Syntax Tree generated by the parser as json. When STDIN ("-") is provided as FILE, this backend serves as an Asciidoc TCK adapter

  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version

```

## Limitations

`asciidocr` currently produces "Htmlbook"-style HTML for all handled
blocks/inlines and has _limited, experimental_ support for producing `docx` files,
namely prose files without tables, lists, etc., etc. *Fair warning*: content may
be dropped while creating `docx` files until that feature stabilizes a little
more. It should, however, be good to go for your next great short story.

`asciidocr` (more or less) parses the vast majority of "common" asciidoc markup
features. Some things it does _not_ do (yet):

* "Literal" blocks (`...`) and inlines (`+` delimited text)
* Checklists
* Offsets
* Tagged regions/tagged includes
* Conditionals
* Complex table markup
* Complex nested lists

For a more complete list of the current limitations and caveats, see
`LIMITATIONS.adoc`.

## Project Goals 

A non-exhaustive list:

* Coverage of the majority, if not all, asciidoc language features
* Passes the [language compatibility toolkit](https://gitlab.eclipse.org/eclipse/asciidoc-lang/asciidoc-tck) tests
* Clean, simple HTML output as a default
* Native "document" (`docx` or `odt`, probably `docx`) output in a "manuscript"
  template, like what you might send to a publisher or literary journal
* PyO3 hooks/project for use inside Python contexts (will be behind a feature
  flag)

## Similar Projects

Great minds think alike, and as such, a few other people are working on asciidoc
tools in Rust now, too. Here are at least a few I know about:

* [`asciidork`](https://github.com/jaredh159/asciidork)
* [`acdc`](https://github.com/nlopes/acdc/tree/main/acdc-parser)

## Known Bugs

Things that _should_ work, but are currently acting up:

* Definition list term ordering under certain circumstances.

If you discover other bugs, please open an
[issue](https://github.com/delfanbaum/asciidocr/issues).
