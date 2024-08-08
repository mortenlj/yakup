VERSION 0.8

FROM busybox

ARG NATIVEPLATFORM
IF [[ "${NATIVEPLATFORM}" == "linux/arm64" ]]
    RUN echo "Running on arm64"
    ARG --global NATIVETARGET=aarch64-unknown-linux-musl
ELSE
    RUN echo "Running on x86_64 we assume"
    ARG --global NATIVETARGET=x86_64-unknown-linux-musl
END
RUN echo "Set NATIVETARGET to ${NATIVETARGET}"

prepare:
    FROM rust:1
    WORKDIR /code
    RUN apt-get --yes update && apt-get --yes install cmake musl-tools gcc-aarch64-linux-gnu gcc-x86-64-linux-gnu
    RUN rustup target add x86_64-unknown-linux-musl
    RUN rustup target add aarch64-unknown-linux-musl
    RUN rustup component add clippy

    ENV CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=/usr/bin/aarch64-linux-gnu-gcc
    ENV CC_aarch64_unknown_linux_musl=/usr/bin/aarch64-linux-gnu-gcc

    ENV CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=/usr/bin/x86_64-linux-gnu-gcc
    ENV CC_x86_64_unknown_linux_musl=/usr/bin/x86_64-linux-gnu-gcc

    RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
    RUN cargo binstall --no-confirm --no-cleanup cargo-chef cargo-nextest

    SAVE IMAGE --push ghcr.io/mortenlj/yakup/cache:prepare

chef-planner:
    FROM +prepare --target=${NATIVETARGET}
    COPY --dir api controller .config Cargo.lock Cargo.toml .
    RUN cargo chef prepare --recipe-path recipe.json
    SAVE ARTIFACT recipe.json

chef-cook:
    ARG --required target
    FROM +prepare --target=${target}
    COPY +chef-planner/recipe.json recipe.json
    RUN cargo chef cook --recipe-path recipe.json --release --target ${target} --tests
    RUN cargo chef cook --recipe-path recipe.json --release --target ${target} --clippy
    RUN cargo chef cook --recipe-path recipe.json --release --target ${target}
    SAVE IMAGE --push ghcr.io/mortenlj/yakup/cache:chef-cook-${target}

test:
    FROM +chef-cook --target=${NATIVETARGET}

    COPY --dir api controller .config Cargo.lock Cargo.toml .
    RUN cargo nextest run --profile ci --release --target ${NATIVETARGET}
    RUN cargo clippy --no-deps --release --target ${NATIVETARGET} -- --deny warnings

    SAVE IMAGE --push ghcr.io/mortenlj/yakup/cache:test

build:
    ARG --required target
    FROM +chef-cook --target=${target}

    COPY --dir api controller .config Cargo.lock Cargo.toml .
    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    RUN cargo build --bin controller --release --target ${target}

    SAVE ARTIFACT target/${target}/release/controller yakup
    SAVE IMAGE --push ghcr.io/mortenlj/yakup/cache:build-${target}

crd:
    FROM +chef-cook --target=${NATIVETARGET}

    COPY --dir api controller .config Cargo.lock Cargo.toml .
    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    RUN cargo run --bin crd --release --target ${NATIVETARGET}

    SAVE ARTIFACT target/crd/application.yaml AS LOCAL target/yaml/application.yaml

docker:
    FROM cgr.dev/chainguard/static:latest

    WORKDIR /bin
    ARG target=${NATIVETARGET}
    COPY --platform=linux/amd64 (+build/yakup --target=${target}) yakup

    CMD ["/bin/yakup"]

    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH

    ARG REGISTRY=ghcr.io/mortenlj/yakup
    ARG image=${REGISTRY}/yakup
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    SAVE IMAGE --push ${image}:${VERSION} ${image}:latest

manifests:
    FROM dinutac/jinja2docker:latest
    WORKDIR /manifests

    COPY deploy/* /templates

    # builtins must be declared
    ARG EARTHLY_GIT_SHORT_HASH
    ARG REGISTRY=mortenlj/yakup
    ARG VERSION=$EARTHLY_GIT_SHORT_HASH
    ARG image=${REGISTRY}/yakup

    FOR template IN $(ls /templates/*.yaml)
        RUN cat ${template} >> ./deploy.yaml
    END

    FOR template IN $(ls /templates/*.j2)
        RUN jinja2 ${template} >> ./deploy.yaml
    END

    SAVE ARTIFACT ./deploy.yaml AS LOCAL target/yaml/deploy.yaml

deploy:
    BUILD --platform=linux/amd64 +prepare --target=${NATIVETARGET}
    BUILD +test
    BUILD --platform=linux/arm64 +docker --target=aarch64-unknown-linux-musl
    BUILD --platform=linux/amd64 +docker --target=x86_64-unknown-linux-musl
    BUILD +manifests
    BUILD +crd
