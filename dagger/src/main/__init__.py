DEVELOP_VERSION = "0.1.0-develop"

import asyncio

import dagger
from dagger import dag, function, object_type
from jinja2 import Template

from .rust import Rust

PLATFORM_TARGET = {
    dagger.Platform("linux/amd64"): "x86_64-unknown-linux-musl",  # a.k.a. x86_64
    dagger.Platform("linux/arm64"): "aarch64-unknown-linux-musl",  # a.k.a. aarch64
}


@object_type
class Yakup:
    @function
    async def prepare(self, source: dagger.Directory) -> dagger.File:
        """Plans the provided source directory"""
        base = await Rust().rust()
        return (
            base
            .with_workdir("/src")
            .with_directory("/src", source)
            .with_exec(["cargo", "chef", "prepare"])
            .file("recipe.json")
        )

    @function
    async def cook(self, source: dagger.Directory, target: str | None = None) -> dagger.Container:
        """Cooks the provided source directory"""
        base = await Rust().rust()
        recipe = await self.prepare(source)
        target_args = ["--target", target] if target else []
        return (
            base
            .with_workdir("/src")
            .with_directory("/src", source)
            .with_file("/src/recipe.json", recipe)
            .with_exec(["cargo", "chef", "cook", "--recipe-path", "/src/recipe.json", "--release", "--tests"] + target_args)
            .with_exec(["cargo", "chef", "cook", "--recipe-path", "/src/recipe.json", "--release", "--clippy"] + target_args)
            .with_exec(["cargo", "chef", "cook", "--recipe-path", "/src/recipe.json", "--release"] + target_args)
        )

    async def test(self, source: dagger.Directory, target: str | None = None) -> dagger.Container:
        """Tests the provided source directory"""
        cooked = await self.cook(source, target)
        target_args = ["--target", target] if target else []
        return (
            cooked
            .with_exec(["cargo", "nextest", "run", "--profile", "ci", "--release"] + target_args)
            .with_exec(["cargo", "clippy", "--no-deps", "--release"] + target_args + ["--", "--deny", "warnings"])
        )

    @function
    async def build(self, source: dagger.Directory, target: str | None = None) -> dagger.File:
        """Builds the provided source directory"""
        cooked = await self.cook(source, target)
        target_args = ["--target", target] if target else []
        return (
            cooked
            .with_exec(["cargo", "build", "--release", "--bin", "controller"] + target_args)
            .file(f"target/{target}/release/controller")
        )

    @function
    async def docker(self, source: dagger.Directory, platform: dagger.Platform | None = None,
                     version: str = DEVELOP_VERSION) -> dagger.Container:
        """Builds a Docker image for the provided source directory"""
        target = PLATFORM_TARGET.get(platform)
        yakup = await self.build(source, target)
        return (
            dag.container(platform=platform)
            .from_("cgr.dev/chainguard/static:latest")
            .with_workdir("/bin")
            .with_file("/bin/yakup", yakup)
            .with_entrypoint(["/bin/yakup"])
        )

    @function
    async def assemble_manifests(
            self, source: dagger.Directory, image: str = "ttl.sh/mortenlj-yakup", version: str = DEVELOP_VERSION
    ) -> dagger.File:
        """Assemble manifests"""
        template_dir = source.directory("deploy")
        documents = []
        for filepath in await template_dir.entries():
            src = await template_dir.file(filepath).contents()
            if not filepath.endswith(".j2"):
                contents = src
            else:
                template = Template(src, enable_async=True)
                contents = await template.render_async(image=image, version=version)
            if contents.startswith("---"):
                documents.append(contents)
            else:
                documents.append("---\n" + contents)
        return await source.with_new_file("deploy.yaml", "\n".join(documents)).file("deploy.yaml")

    @function
    async def publish(
            self, source: dagger.Directory, image: str = "ttl.sh/mortenlj-yakup", version: str = DEVELOP_VERSION
    ) -> list[str]:
        """Publish the application container after building and testing it on-the-fly"""
        platforms = [
            dagger.Platform("linux/amd64"),  # a.k.a. x86_64
            dagger.Platform("linux/arm64"),  # a.k.a. aarch64
        ]
        cos = []
        manifest = dag.container()
        for v in ["latest", version]:
            variants = []
            for platform in platforms:
                variants.append(self.docker(source, platform, version))
            cos.append(manifest.publish(f"{image}:{v}", platform_variants=await asyncio.gather(*variants)))

        return await asyncio.gather(*cos)
