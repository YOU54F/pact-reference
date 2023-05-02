#!/bin/bash -eu
set -eu

# Tested at https://www.shellcheck.net/
# linux/386
# linux/amd64
# linux/arm/v6
# linux/arm/v7
# linux/arm64/v8
# linux/ppc64le
# linux/s390x
GREEN='\033[0;32m'
NC='\033[0m' # No Color
tool=${tool:-"pact_verifier_cli"}
tool_version=${tool_version:-''}
if [[ $tool == 'pact_verifier_cli' && $tool_version == '' ]]; then
    tool_version="0.10.6"
elif [[ $tool == 'pact_mock_server_cli' && $tool_version == '' ]]; then
    tool_version="1.0.1"
fi
docker_owner="you54f"
versions=(3.17 3.16 3.15 3.14)
LOAD=${LOAD:-''}
BUILD_MULTI=${BUILD_MULTI:-''}
PUSH_MULTI=${PUSH_MULTI:-''}
for version in "${versions[@]}"; do
    sed <Dockerfile.test 's/ENTRYPOINT \[ \"pact_verifier_cli\" \]/ENTRYPOINT \[ '\"${tool}\"' \]/g' >Dockerfile.build
    image=${docker_owner}/${tool}:alpine-${version}
    if [[ $LOAD == 'true' ]]; then
        echo -e "${GREEN}LOAD set - building target platform image ${image} and loading to local docker registry${NC}"
        docker buildx build . -f Dockerfile.build \
            --build-arg=ALPINE_VERSION="${version}" \
            --build-arg=PACT_TOOL="${tool}" \
            --build-arg=PACT_TOOL_VERSION=${tool_version} \
            -t "${image}" \
            --load \
            --platform=linux/arm64
    elif [[ $BUILD_MULTI == 'true' ]]; then
        echo -e "${GREEN} BUILD_MULTI set - building multi-arch image ${image} locally${NC}"
        docker buildx build . -f Dockerfile.build \
            --build-arg=ALPINE_VERSION="${version}" \
            --build-arg=PACT_TOOL="${tool}" \
            --build-arg=PACT_TOOL_VERSION=${tool_version} \
            -t "${image}" \
            --platform=linux/arm64,linux/amd64,linux/arm/v6,linux/arm/v7
    elif [[ $PUSH_MULTI == 'true' ]]; then
        echo -e "${GREEN} PUSH_MULTI set - building and pushing ${image} to Dockerhub${NC}"
        docker buildx build . -f Dockerfile.build \
            --build-arg=ALPINE_VERSION="${version}" \
            --build-arg=PACT_TOOL="${tool}" \
            --build-arg=PACT_TOOL_VERSION=${tool_version} \
            -t "${image}" \
            --push \
            --platform=linux/arm64,linux/amd64,linux/arm/v6,linux/arm/v7
    else
        echo -e "${GREEN} no local build command, pulling ${image} from Dockerhub${NC}"
    fi
    docker run --rm -it --init "${image}" --version
    docker run --rm -it --init "${image}" --help
    if [[ $tool == 'pact_mock_server_cli' ]]; then
        docker run --rm --init "${image}" start &
        PID=$!
        sleep 1
        kill $PID
    fi
    rm Dockerfile.build
    echo -e "${GREEN} All done building ${image}${NC}"
done
for version in "${versions[@]}"; do
    echo -e "${GREEN} success for ${image}${NC}"
done
