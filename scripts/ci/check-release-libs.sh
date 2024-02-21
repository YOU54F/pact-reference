#!/bin/bash -eu

# Usage: ./check-release-libs.sh [OPTIONS]
# 
# This script checks the release assets of a GitHub repository.
# 
# Options:
#   -r, --repo <REPO>        The GitHub repository to check (default: pact-foundation/pact-js-core)
#   -t, --tag <TAG>          The release tag to check (default: TAG)
#   -l, --list-assets        List the remote release assets
#   -f, --fetch-assets       Fetch the remote release assets (will clean local assets)
#   -c, --clean-assets       Clean the local release assets
#
# Example:
#   ./check-release-libs.sh -r myorg/myrepo -t v1.0.0 -l
#
# This will list the remote release assets of the myorg/myrepo repository for the v1.0.0 tag.

# Parse command line arguments
while [[ $# -gt 0 ]]
do
key="$1"

case $key in
    -r|--repo)
    REPO="$2"
    shift # past argument
    shift # past value
    ;;
    -t|--tag)
    TAG="$2"
    shift # past argument
    shift # past value
    ;;
    -l|--list-assets)
    LIST_ASSETS=true
    shift # past argument
    ;;
    -f|--fetch-assets)
    FETCH_ASSETS=true
    shift # past argument
    ;;
    -c|--clean-assets)
    CLEAN_ASSETS=true
    shift # past argument
    ;;
    *)    # unknown option
    echo "Unknown option: $1"
    exit 1
    ;;
esac
done

# Set default values for REPO and TAG if not provided
REPO=${REPO:-you54f/pact-reference}
TAG=${NEXT_TAG:-${TAG:-latest}}

echo "Checking release assets"

if [[ "${CLEAN_ASSETS:-}" = true || "${FETCH_ASSETS:-}" = true ]]; then
    echo "Cleaning local release assets"
    rm -rf *.tar.gz
    rm -rf prebuilds
fi

if [[ "$TAG" == "" ]]; then
    echo "Please provide a release TAG to check"
    exit 1
else
    GH_TAG_OPTION="$TAG"
    if [[ "$TAG" == "latest" ]]; then
        GH_TAG_OPTION=$(gh release list -R "${REPO}" | grep libpact_ffi | sort -k1 | head -n1 | awk '{print $1}')
        # GH_TAG_OPTION=$(gh release list -R "${REPO}" | grep libpact_ffi | sort -k1 | head -n1 |  cut -d'-' -f2- | awk '{print $1}')
        echo "latest tag is ${GH_TAG_OPTION}"
    fi

    if [[ "${LIST_ASSETS:-}" = true || "${FETCH_ASSETS:-}" = true ]]; then
        echo "Listing remote release assets for ${REPO} ${GH_TAG_OPTION}"
        ASSETS=$(gh release view --repo "${REPO}" $GH_TAG_OPTION --json assets | jq '.assets[].name')
    fi

    if [ "${FETCH_ASSETS:-}" = true ]; then
        echo "Fetching release assets"
        gh release download --repo "${REPO}" $GH_TAG_OPTION
    fi

fi

ERRORS=()
# ls *.gz
# ls *.gz | xargs -n1 tar -xzf
# rm *.tar.gz
# ls -1 prebuilds/**

echo $ASSETS | grep -e 'libpact_ffi-linux-aarch64-musl.a.gz' > /dev/null || ERRORS='libpact_ffi-linux-aarch64-musl.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-aarch64-musl.so.gz' > /dev/null || ERRORS='libpact_ffi-linux-aarch64-musl.so.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-x86_64-musl.a.gz' > /dev/null || ERRORS='libpact_ffi-linux-x86_64-musl.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-x86_64-musl.so.gz' > /dev/null || ERRORS='libpact_ffi-linux-x86_64-musl.so.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-aarch64.a.gz' > /dev/null || ERRORS='libpact_ffi-linux-aarch64.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-aarch64.so.gz' > /dev/null || ERRORS='libpact_ffi-linux-aarch64.so.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-x86_64.a.gz' > /dev/null || ERRORS='libpact_ffi-linux-x86_64.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-linux-x86_64.so.gz' > /dev/null || ERRORS='libpact_ffi-linux-x86_64.so.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-x86_64.dll.lib.gz' > /dev/null || ERRORS='pact_ffi-windows-x86_64.dll.lib.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-x86_64.dll.gz' > /dev/null || ERRORS='pact_ffi-windows-x86_64.dll.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-x86_64.lib.gz' > /dev/null || ERRORS='pact_ffi-windows-x86_64.lib.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-aarch64.dll.lib.gz' > /dev/null || ERRORS='pact_ffi-windows-aarch64.dll.lib.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-aarch64.dll.gz' > /dev/null || ERRORS='pact_ffi-windows-aarch64.dll.gz'
echo $ASSETS | grep -e 'pact_ffi-windows-aarch64.lib.gz' > /dev/null || ERRORS='pact_ffi-windows-aarch64.lib.gz'
echo $ASSETS | grep -e 'libpact_ffi-osx-aarch64-apple-darwin.a.gz' > /dev/null || ERRORS='libpact_ffi-osx-aarch64-apple-darwin.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-osx-aarch64-apple-darwin.dylib.gz' > /dev/null || ERRORS='libpact_ffi-osx-aarch64-apple-darwin.dylib.gz'
echo $ASSETS | grep -e 'libpact_ffi-osx-x86_64.a.gz' > /dev/null || ERRORS='libpact_ffi-osx-x86_64.a.gz'
echo $ASSETS | grep -e 'libpact_ffi-osx-x86_64.dylib.gz' > /dev/null || ERRORS='libpact_ffi-osx-x86_64.dylib.gz'

if [ ! -z "${ERRORS:-}" ]; then
    echo "The following files are missing from the release:"
    echo $ERRORS
    exit 1
else
    echo "All release files are present"
    echo "PACT_FFI_REL_TAG=${GH_TAG_OPTION}" >> $GITHUB_ENV 
    echo "PACT_FFI_VERSION=$(echo $GH_TAG_OPTION |  cut -d'-' -f2-)" >> $GITHUB_ENV 
fi