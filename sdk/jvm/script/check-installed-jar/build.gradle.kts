// Resolves io.github.kengos:xlsx-graft from the Maven repository the gate
// published it to (/tmp/repo) and its JNA from the image's Gradle cache
// (--offline), then runs CheckInstalledJar on the line's JVM. No lockfile: the
// jar is the one just published, and the check reads the SDK's lockfile
// itself.

plugins {
    application
}

repositories {
    maven { url = uri("/tmp/repo") }
    mavenCentral()
}

dependencies {
    implementation("io.github.kengos:xlsx-graft:0.0.0")
}

application {
    mainClass = "CheckInstalledJar"
}

tasks.withType<JavaCompile>().configureEach {
    options.release = 8
    options.encoding = "UTF-8"
    options.compilerArgs.addAll(listOf("-Xlint:all", "-Xlint:-options", "-Werror"))
}

tasks.named<JavaExec>("run") {
    System.getenv("XLSX_GRAFT_JVM_HOME")?.let { setExecutable("$it/bin/java") }
}
