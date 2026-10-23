// Run by `make sdk:verify:jvm` after it publishes the jar: the gate copies
// this build out of the checkout and runs it against the published jar
// (build.gradle.kts).
rootProject.name = "check-installed-jar"
