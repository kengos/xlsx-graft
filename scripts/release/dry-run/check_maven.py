"""Checks a Maven Central bundle the way the Central Portal does.

    python3 -I scripts/release/dry-run/check_maven.py <bundle.zip> <classifier>...

(scripts/release/maven-bundle.sh makes the bundle.) The bundle holds the jar,
its sources and javadoc jars, a jar for each classifier named, and the POM,
each with `.asc`, `.md5` and `.sha1` files, the checksums being those of the
file; the version is not a SNAPSHOT; and the POM carries what Central requires:
name, description, url, a license with a name and a url, a developer, and the
source control's url and connection. Every failure is printed, then the exit
status is 1.
"""

from __future__ import annotations

import hashlib
import re
import sys
import zipfile
from xml.etree import ElementTree

PREFIX = "io/github/kengos/xlsx-graft/"
POM_FIELDS = ("name", "description", "url")


def child(element: ElementTree.Element | None, tag: str) -> ElementTree.Element | None:
    if element is None:
        return None
    for each in element:
        if each.tag.rsplit("}", 1)[-1] == tag:
            return each
    return None


def children(element: ElementTree.Element | None, parent: str, tag: str) -> list[ElementTree.Element]:
    """The <tag> children of the <parent> child of `element`."""
    container = child(element, parent)
    if container is None:
        return []
    return [each for each in container if each.tag.rsplit("}", 1)[-1] == tag]


def value(element: ElementTree.Element | None, tag: str) -> str:
    found = child(element, tag)
    return (found.text or "").strip() if found is not None else ""


def check_pom(pom: bytes) -> list[str]:
    project = ElementTree.fromstring(pom)
    problems = [f"the POM has no <{field}>" for field in POM_FIELDS if not value(project, field)]
    for field in ("groupId", "artifactId", "version"):
        if not value(project, field):
            problems.append(f"the POM has no <{field}>")
    if value(project, "version").endswith("-SNAPSHOT"):
        problems.append("the version is a SNAPSHOT")
    licenses = children(project, "licenses", "license")
    if not licenses:
        problems.append("the POM has no <license>")
    for each in licenses:
        if not value(each, "name") or not value(each, "url"):
            problems.append("a <license> of the POM has no name or no url")
    developers = children(project, "developers", "developer")
    if not developers or not all(value(d, "name") or value(d, "id") for d in developers):
        problems.append("the POM has no <developer> with a name or an id")
    scm = child(project, "scm")
    if not value(scm, "url") or not value(scm, "connection"):
        problems.append("the POM's <scm> has no url or no connection")
    return problems


def main(bundle: str, *classifiers: str) -> int:
    problems: list[str] = []
    with zipfile.ZipFile(bundle) as archive:
        names = set(archive.namelist())
        files = sorted(n for n in names if n.startswith(PREFIX) and not n.endswith("/") and not re.search(r"\.(asc|md5|sha1)$", n))
        poms = [n for n in files if n.endswith(".pom")]
        jars = [n for n in files if n.endswith(".jar")]
        versions = {n[len(PREFIX) :].split("/")[0] for n in files}
        if len(versions) != 1:
            problems.append(f"the bundle holds the versions {sorted(versions)}, not one")
        version = next(iter(versions), "")
        stem = f"{PREFIX}{version}/xlsx-graft-{version}"
        wanted = [f"{stem}.jar", f"{stem}-sources.jar", f"{stem}-javadoc.jar", f"{stem}.pom"]
        wanted += [f"{stem}-{classifier}.jar" for classifier in classifiers]
        for name in wanted:
            if name not in files:
                problems.append(f"no {name}")
        for name in files:
            if name not in wanted:
                problems.append(f"{name} is in the bundle and nobody asked for it")
            if f"{name}.asc" not in names:
                problems.append(f"no {name}.asc")
            data = archive.read(name)
            for suffix, algorithm in ((".md5", "md5"), (".sha1", "sha1")):
                if name + suffix not in names:
                    problems.append(f"no {name}{suffix}")
                elif archive.read(name + suffix).decode().strip() != hashlib.new(algorithm, data).hexdigest():
                    problems.append(f"{name}{suffix} is not the {algorithm} of the file")
        if len(poms) == 1:
            problems += check_pom(archive.read(poms[0]))
        if not jars:
            problems.append("no jar")
    for problem in problems:
        print(f"check_maven: {problem}", file=sys.stderr)
    print(f"check_maven: {len(names)} entries, {len(problems)} problems")
    return 1 if problems else 0


if __name__ == "__main__":
    if len(sys.argv) < 2:
        sys.exit(f"usage: {sys.argv[0]} <bundle.zip> <classifier>...")
    sys.exit(main(*sys.argv[1:]))
