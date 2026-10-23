// The Java SDK: the library, its gate tasks and its publication.
//
// Gradle, javac, javadoc and the formatter run on the image's JDK 25 and
// compile Java 8 bytecode (--release 8, the floor in docs/sdk.md). The tests
// run on the JVM of the line being gated (XLSX_GRAFT_JVM_HOME, set by the
// image), so each line's gate checks the bytecode on that line's JVM.
//
// Every configuration is locked (gradle.lockfile, and the plugins in
// buildscript-gradle.lockfile; `make sdk:lock:jvm` writes both). The
// formatter Spotless fetches is not a configuration Gradle can lock, so its
// version is written here and the gate runs --offline against what the
// image's build resolved.

import org.gradle.api.tasks.testing.logging.TestExceptionFormat

buildscript {
    dependencyLocking {
        lockAllConfigurations()
        lockMode = LockMode.STRICT
    }
}

plugins {
    java
    jacoco
    `maven-publish`
    id("com.diffplug.spotless") version "8.10.4"
}

group = "io.github.kengos"

repositories {
    mavenCentral()
}

// STRICT: a configuration without a lock state fails rather than resolving
// freely, as a dependency missing from the lock does.
dependencyLocking {
    lockAllConfigurations()
    lockMode = LockMode.STRICT
}

dependencies {
    // The one dependency, and a check in script/check-installed-jar keeps it
    // the only one in the published POM.
    implementation("net.java.dev.jna:jna:5.19.1")

    // JUnit 5, not 6: JUnit 6 needs Java 17 to run, and the tests run on the
    // floor's Java 8.
    testImplementation(platform("org.junit:junit-bom:5.14.4"))
    testImplementation("org.junit.jupiter:junit-jupiter")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// Nothing a gate runs writes into the mounted checkout, which it would leave
// owned by root: the image points the build directory at /tmp (the gate also
// moves .gradle/ there with --project-cache-dir).
System.getenv("XLSX_GRAFT_BUILD_DIR")?.let { layout.buildDirectory = file(it) }

java {
    withSourcesJar()
    withJavadocJar()
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 8
    options.encoding = "UTF-8"
    // -options: JDK 25's javac warns that --release 8 is obsolete, and
    // supporting Java 8 is the point.
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Xlint:-options", "-Werror"))
}

tasks.javadoc {
    (options as StandardJavadocDocletOptions).apply {
        addStringOption("-release", "8")
        encoding = "UTF-8"
        // Named, so the javadoc jar does not vary with the builder's locale.
        locale = "en_US"
        addBooleanOption("Xdoclint:all", true)
        addBooleanOption("Werror", true)
        addBooleanOption("quiet", true)
    }
}

tasks.jar {
    manifest {
        // Java 8 has no module-info; this names the module on Java 9 and later.
        attributes(
            "Automatic-Module-Name" to "io.github.kengos.xlsxgraft",
            "Implementation-Version" to project.version,
        )
    }
}

tasks.test {
    useJUnitPlatform()
    System.getenv("XLSX_GRAFT_JVM_HOME")?.let { executable = "$it/bin/java" }
    testLogging {
        events("passed", "skipped", "failed")
        exceptionFormat = TestExceptionFormat.FULL
        showStandardStreams = true
    }
}

jacoco {
    toolVersion = "0.8.15"
}

tasks.jacocoTestCoverageVerification {
    dependsOn(tasks.test)
    violationRules {
        // Per class, so a failure names the class that falls short.
        rule {
            element = "CLASS"
            limit {
                counter = "LINE"
                minimum = "1.0".toBigDecimal()
            }
        }
    }
}

spotless {
    java {
        target("src/**/*.java", "script/check-installed-jar/src/**/*.java")
        // Not 1.37.0, the latest: Spotless 8.10.4 fails on every file with
        // it (an InvocationTargetException out of the formatter step), while
        // google-java-format 1.37.0 run on its own formats them.
        googleJavaFormat("1.36.1")
    }
}

// No Gradle module metadata (.module) beside the POM: a Gradle build would
// resolve the jar's dependencies from it rather than from the POM, and the
// POM is the one that script/check-installed-jar checks.
tasks.withType<GenerateModuleMetadata>().configureEach {
    enabled = false
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])
            pom {
                name = "xlsx-graft"
                description = "Read, verify, create and edit xlsx workbooks"
                url = "https://github.com/kengos/xlsx-graft"
                licenses {
                    // The workspace's license, as an SPDX expression.
                    license { name = "MIT OR Apache-2.0 OR BSD-3-Clause" }
                }
            }
        }
    }
}
