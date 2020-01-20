#!/bin/bash

package_executable() {
    local temp_dir=$(mktemp -d)
    local name="$PROJECT_NAME-$TRAVIS_TAG-$TARGET"

    # Setting up the directory structure for packaging the program. 
    local staging="$temp_dir/$name"
    local executable="target/$TARGET/release/$PROJECT_NAME"
    mkdir -p "$staging"
    mkdir -p "$staging/docs"
    mkdir -p "$staging/tests"
    cp {README.adoc,LICENSE} "$staging"
    cp {docs/manual.adoc,CHANGELOG.adoc,"$PROJECT_NAME.1"} "$staging/docs"
    cp -r {tests} "$staging/tests"

    # Stripping the size of the binary file
    upx --best "$executable"
    cp "target/$TARGET/release/$PROJECT_NAME" "$staging"

    # This directory is where the binaries will be stored.
    local out_dir="$(pwd)/deployment"
    mkdir -p "$out_dir"

    # Creating the archive from the staging area.
    if [ "$TRAVIS_OS_NAME" = "windows" ]; then 
        local out_file="$name.7z"
        7z a -t7z "$out_dir/$out_file" "$staging"
    else
        local out_file="$name.tar.gz"
        tar czf "$out_dir/$out_file" --directory="$staging" .
    fi
}

main() {
    rustup component add rustfmt
    cargo fmt --all -- --check && cargo test --all && cargo build --target "$TARGET" --verbose --release && package_executable
}

main