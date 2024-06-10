VERSION 0.8

IMPORT github.com/mortenlj/earthly-lib/kubernetes/commands AS lib-k8s-commands

FROM busybox

prepare:
    FROM rust:1
    WORKDIR /code
    RUN cargo install cargo-chef
    RUN apt-get --yes update && apt-get --yes install cmake musl-tools gcc-aarch64-linux-gnu
    RUN rustup target add x86_64-unknown-linux-musl
    RUN rustup target add aarch64-unknown-linux-musl

    ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=/usr/bin/aarch64-linux-gnu-gcc
    ENV CC_aarch64_unknown_linux_musl=/usr/bin/aarch64-linux-gnu-gcc

    SAVE IMAGE --push ghcr.io/mortenlj/indeed/cache:prepare

chef-planner:
    FROM +prepare
    COPY --dir api controller Cargo.lock Cargo.toml .
    RUN cargo chef prepare --recipe-path recipe.json
    SAVE ARTIFACT recipe.json

chef-cook:
    FROM +prepare
    COPY +chef-planner/recipe.json recipe.json
    ARG target
    RUN cargo chef cook --recipe-path recipe.json --release --target ${target}
    SAVE IMAGE --push ghcr.io/mortenlj/indeed/cache:chef-cook-${target}

build:
    FROM +chef-cook

    COPY --dir api controller Cargo.lock Cargo.toml .
    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG target
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    RUN cargo build --bin controller --release --target ${target}

    SAVE ARTIFACT target/${target}/release/controller indeed
    SAVE IMAGE --push ghcr.io/mortenlj/indeed/cache:build-${target}

crd:
    FROM +chef-cook --target=x86_64-unknown-linux-musl

    COPY --dir api controller Cargo.lock Cargo.toml .
    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    RUN cargo run --bin crd

    SAVE ARTIFACT target/crd/application.yaml application.yaml

docker:
    FROM cgr.dev/chainguard/static:latest

    WORKDIR /bin
    ARG target=x86_64-unknown-linux-musl
    COPY --platform=linux/amd64 (+build/indeed --target=$target) indeed

    CMD ["/bin/suffiks-ingress"]

    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH

    ARG REGISTRY=ghcr.io/mortenlj/indeed
    ARG image=${REGISTRY}/indeed
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    SAVE IMAGE --push ${image}:${VERSION} ${image}:latest

manifests:
    FROM dinutac/jinja2docker:latest
    WORKDIR /manifests

    COPY deploy/* /templates
    COPY --platform=linux/amd64 +crd/application.yaml /templates

    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG REGISTRY=mortenlj/indeed
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    ARG image=${REGISTRY}/indeed

    FOR template IN $(ls /templates/*.yaml)
        RUN cat ${template} >> ./deploy.yaml
    END

    FOR template IN $(ls /templates/*.j2)
        RUN jinja2 ${template} >> ./deploy.yaml
    END

    SAVE ARTIFACT ./deploy.yaml AS LOCAL deploy.yaml

deploy:
    BUILD --platform=linux/amd64 +prepare
    BUILD --platform=linux/arm64 +docker --target=aarch64-unknown-linux-musl
    BUILD --platform=linux/amd64 +docker --target=x86_64-unknown-linux-musl
    BUILD +manifests
