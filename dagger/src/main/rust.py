import dagger
from dagger import dag, object_type, function

RUST_REPO = "rust-lang/rust"


@object_type
class Rust:
    @function
    async def rust(self) -> dagger.Container:
        tag = await dag.github().get_latest_release(RUST_REPO).tag()
        return (
            dag.container()
            .from_(f"rust:{tag}")
            .with_exec(["apt-get", "--yes", "update"])
            .with_exec(
                ["apt-get", "--yes", "install", "cmake", "musl-tools", "gcc-aarch64-linux-gnu", "gcc-x86-64-linux-gnu"])
            .with_exec(["rustup", "target", "add", "x86_64-unknown-linux-musl"])
            .with_exec(["rustup", "target", "add", "aarch64-unknown-linux-musl"])
            .with_exec(["rustup", "component", "add", "clippy"])
            .with_env_variable("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER", "/usr/bin/aarch64-linux-gnu-gcc")
            .with_env_variable("CC_aarch64_unknown_linux_musl", "/usr/bin/aarch64-linux-gnu-gcc")
            .with_env_variable("CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER", "/usr/bin/x86_64-linux-gnu-gcc")
            .with_env_variable("CC_x86_64_unknown_linux_musl", "/usr/bin/x86_64-linux-gnu-gcc")
            .with_exec(["cargo", "install", "cargo-binstall"])
            .with_exec(["cargo", "binstall", "--no-confirm", "cargo-chef"])
            .with_exec(["cargo", "binstall", "--no-confirm", "cargo-nextest"])
        )
