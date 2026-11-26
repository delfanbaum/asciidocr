#!/bin/sh
# Generates the expected HTML for Asciidoctor-style fixtures from their .adoc files, with
# Asciidoctor itself (the oracle; the fixtures were made with 2.0.17), so that nobody writes them
# by hand. Fixtures under `embedded/` are `asciidoctor -s` output; the others are full documents
# without the stylesheet (`-a stylesheet!`), with the generator <meta> line removed (we don't
# write one) and the "Last updated" time replaced by NOW (the test helper does the same).
#
# usage: scripts/asciidoctor-fixtures.sh tests/data/asciidoctor-html/[embedded/]name.adoc ...
set -eu
command -v asciidoctor >/dev/null || { echo "asciidoctor is not installed" >&2; exit 1; }
for adoc in "$@"; do
    html="${adoc%.adoc}.html"
    case "$adoc" in
    */embedded/*) asciidoctor -s -o - "$adoc" >"$html" ;;
    *)
        asciidoctor -a 'stylesheet!' -o - "$adoc" |
            sed -e '/<meta name="generator"/d' -e 's/Last updated .*/Last updated NOW/' >"$html"
        ;;
    esac
    echo "$html"
done
