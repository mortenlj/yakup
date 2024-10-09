import asyncio
from datetime import datetime
from typing import Annotated

from jinja2 import Template

import dagger
from dagger import dag, function, object_type, DefaultPath

PROD_IMAGE = "cgr.dev/chainguard/static:latest"
DEVELOP_IMAGE = "ttl.sh/mortenlj-yakup"

DEVELOP_VERSION = f"{datetime.now().strftime("%Y%m%d%H%M%S")}-develop"

PLATFORM_TARGET = {
    dagger.Platform("linux/amd64"): "x86_64-unknown-linux-musl",  # a.k.a. x86_64
    dagger.Platform("linux/arm64"): "aarch64-unknown-linux-musl",  # a.k.a. aarch64
}


@object_type
class Yakup:
    @function
    async def rust(self) -> dagger.Container:
        tools = (
            dag.container()
            .from_(f"rust:1")
            .with_exec(["apt-get", "--yes", "update"])
            .with_exec(
                ["apt-get", "--yes", "install", "cmake", "musl-tools", "gcc-aarch64-linux-gnu", "gcc-x86-64-linux-gnu"])
        )
        for target in PLATFORM_TARGET.values():
            tools = tools.with_exec(["rustup", "target", "add", target])
        return (
            tools
            .with_exec(["rustup", "component", "add", "clippy"])
            .with_env_variable("CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER", "/usr/bin/aarch64-linux-gnu-gcc")
            .with_env_variable("CC_aarch64_unknown_linux_musl", "/usr/bin/aarch64-linux-gnu-gcc")
            .with_env_variable("CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER", "/usr/bin/x86_64-linux-gnu-gcc")
            .with_env_variable("CC_x86_64_unknown_linux_musl", "/usr/bin/x86_64-linux-gnu-gcc")
            .with_exec(["cargo", "install", "cargo-binstall"])
            .with_exec(["cargo", "binstall", "--no-confirm", "cargo-chef"])
            .with_exec(["cargo", "binstall", "--no-confirm", "cargo-nextest"])
        )

    @function
    async def prepare(self, source: Annotated[dagger.Directory, DefaultPath("/")]) -> dagger.File:
        """Plans the provided source directory"""
        base = await self.rust()
        return (
            base
            .with_workdir("/src")
            .with_directory("/src/api", source.directory("api"))
            .with_directory("/src/controller", source.directory("controller"))
            .with_directory("/src/.config", source.directory(".config"))
            .with_file("/src/Cargo.toml", source.file("Cargo.toml"))
            .with_file("/src/Cargo.lock", source.file("Cargo.lock"))
            .with_exec(["cargo", "chef", "prepare"])
            .file("recipe.json")
        )

    @function
    async def cook(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            target: str | None = None
    ) -> dagger.Container:
        """Cooks the provided source directory"""
        base = await self.rust()
        recipe = await self.prepare(source)
        if target is None:
            target = PLATFORM_TARGET.get(await dag.default_platform())

        pot = base.with_workdir("/src").with_file("/src/recipe.json", recipe)
        for variant in (["--tests"], ["--clippy"], []):
            pot = pot.with_exec(["cargo", "chef", "cook",
                                 "--recipe-path", "/src/recipe.json", "--release",
                                 "--target", target] +
                                variant)
        return pot

    @function
    async def project(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            target: str | None = None
    ) -> dagger.Container:
        """Prepares the provided source directory"""
        cooked = await self.cook(source, target)
        return (
            cooked
            .with_directory("/src/api", source.directory("api"))
            .with_directory("/src/controller", source.directory("controller"))
            .with_directory("/src/.config", source.directory(".config"))
            .with_file("/src/Cargo.toml", source.file("Cargo.toml"))
            .with_file("/src/Cargo.lock", source.file("Cargo.lock"))
        )

    @function
    async def test(self, source: Annotated[dagger.Directory, DefaultPath("/")]) -> dagger.File:
        """Tests the provided source directory"""
        platform = await dag.default_platform()
        target = PLATFORM_TARGET.get(platform)
        proj = await self.project(source, target)
        return (
            proj
            .with_exec(
                ["cargo", "clippy", "--no-deps", "--release", "--target", target, "--", "--deny", "warnings"])
            .with_exec(["cargo", "nextest", "run", "--profile", "ci", "--release", "--target", target])
            .file("target/nextest/ci/junit.xml")
        )

    @function
    async def build(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            target: str | None = None
    ) -> dagger.File:
        """Builds the provided source directory"""
        proj = await self.project(source, target)
        if target is None:
            target = PLATFORM_TARGET.get(await dag.default_platform())
        return (
            proj
            .with_exec(["cargo", "build", "--release", "--bin", "controller", "--target", target])
            .file(f"target/{target}/release/controller")
        )

    @function
    async def docker(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            platform: dagger.Platform | None = None
    ) -> dagger.Container:
        """Builds a Docker image for the provided source directory"""
        if platform is None:
            platform = await dag.default_platform()
        target = PLATFORM_TARGET.get(platform)
        yakup = await self.build(source, target)
        return (
            dag.container(platform=platform)
            .from_(PROD_IMAGE)
            .with_workdir("/bin")
            .with_file("/bin/yakup", yakup)
            .with_entrypoint(["/bin/yakup"])
        )

    @function
    async def crd(self, source: Annotated[dagger.Directory, DefaultPath("/")]) -> dagger.File:
        """Generate CRD"""
        target = PLATFORM_TARGET.get(await dag.default_platform())
        proj = await self.project(source, target)
        return (
            proj
            .with_exec(["cargo", "run", "--bin", "crd", "--release", "--target", target])
            .file("target/crd/application.yaml")
        )

    @function
    async def assemble_manifests(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            image: str = DEVELOP_IMAGE,
            version: str = DEVELOP_VERSION
    ) -> dagger.File:
        """Assemble manifests"""
        template_dir = source.directory("deploy")
        documents = []
        for filepath in await template_dir.entries():
            src = await template_dir.file(filepath).contents()
            if filepath.endswith(".yaml"):
                contents = src
            elif filepath.endswith(".j2"):
                template = Template(src, enable_async=True)
                contents = await template.render_async(image=image, version=version)
            else:
                continue
            if contents.startswith("---"):
                documents.append(contents)
            else:
                documents.append("---\n" + contents)
        documents.append("")
        return await source.with_new_file("deploy.yaml", "\n".join(documents)).file("deploy.yaml")

    @function
    async def publish(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            image: str = DEVELOP_IMAGE,
            version: str = DEVELOP_VERSION
    ) -> list[str]:
        """Publish the application container after building and testing it on-the-fly"""
        platforms = {
            await dag.default_platform(),
            dagger.Platform("linux/amd64"),  # a.k.a. x86_64
            dagger.Platform("linux/arm64"),  # a.k.a. aarch64
        }
        cos = []
        manifest = dag.container()
        for v in ["latest", version]:
            variants = []
            for platform in platforms:
                variants.append(self.docker(source, platform))
            cos.append(manifest.publish(f"{image}:{v}", platform_variants=await asyncio.gather(*variants)))

        return await asyncio.gather(*cos)

    @function
    async def assemble(
            self,
            source: Annotated[dagger.Directory, DefaultPath("/")],
            image: str = DEVELOP_IMAGE,
            version: str = DEVELOP_VERSION
    ) -> dagger.Directory:
        """Collect all deployment artifacts (container, crd and manifests)"""
        outputs = dag.directory()
        files = await asyncio.gather(
            self.publish_to_file(self.publish(source, image, version)),
            self.crd(source),
            self.assemble_manifests(source, image, version),
        )
        for f in files:
            filename = await f.name()
            outputs = outputs.with_file(filename, f)
        return outputs

    @staticmethod
    async def publish_to_file(publish_task) -> dagger.File:
        image_tags = await publish_task
        return (
            dag.directory()
            .with_new_file("image_tags.txt", "\n".join(image_tags))
            .file("image_tags.txt")
        )
