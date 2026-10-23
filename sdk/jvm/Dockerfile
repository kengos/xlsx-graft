# The Java SDK's gate environment: `make sdk:*:jvm` builds and runs this
# image, so nobody needs a JDK or Gradle installed. The build context is
# sdk/jvm.
#
# The image holds two JDKs, Gradle and the locked dependencies in Gradle's
# cache, and nothing else. JDK 25 runs Gradle and the tools (Gradle 9 needs 17
# or newer to run, the formatter 21); the line's JDK runs the tests and loads
# the published jar, so every line is checked on its own JVM. The engine
# library is NOT copied in: the gate mounts the repository at /repo and points
# XLSX_GRAFT_LIBRARY at the library `make engine:capi-lib` built there, so
# changing the engine never rebuilds this image. That library is built on
# bookworm (glibc 2.36); Temurin's images are Ubuntu noble (glibc 2.39), and
# glibc loads a library built against the same or an older glibc. Temurin's
# alpine (musl) would load nothing.
#
# JVM_PATCH and JVM_LINE have no default on purpose. mk/sdk/jvm.mk maps
# JVM_VER (8, 11, …) to the Temurin release, and it is the one place that
# number is written. The tools' JDK and Gradle are pinned here and nowhere
# else; the file's hash is in the image's tag, so bumping them builds a new
# one. Gradle's own image is not the base: its tag moves to a new JDK when it
# is rebuilt.
ARG JVM_PATCH

FROM gradle:9.8.2-jdk25-noble AS gradle
FROM eclipse-temurin:${JVM_PATCH}-jdk-noble AS line

# `sdk:lock:jvm` runs this stage alone: it has to write the lockfiles, so it
# cannot resolve from them.
FROM eclipse-temurin:25.0.4.1_1-jdk-noble AS tools
ARG JVM_LINE
COPY --from=gradle /opt/gradle /opt/gradle
COPY --from=line /opt/java/openjdk /opt/jdk-line

# Nothing a gate runs writes into the mounted checkout, which it would leave
# owned by root: the build directory goes to /tmp (build.gradle.kts), and
# Gradle's cache lives in the image, outside /repo (the mount would hide it).
# A named UTF-8 locale, so neither the JVM nor javadoc falls back to an empty
# one.
ENV PATH=/opt/gradle/bin:${PATH} \
    GRADLE_USER_HOME=/opt/gradle-home \
    XLSX_GRAFT_BUILD_DIR=/tmp/gradle-build \
    XLSX_GRAFT_JVM_HOME=/opt/jdk-line \
    XLSX_GRAFT_JVM_LINE=${JVM_LINE} \
    LANG=C.UTF-8 \
    LC_ALL=C.UTF-8

FROM tools

# Only the files that decide the dependencies. Editing a .java file then never
# re-runs this. STRICT locking (build.gradle.kts): a build file edit the lock
# no longer satisfies (a new dependency, a version above the locked one)
# without `make sdk:lock:jvm` fails here rather than gating on the stale lock.
# A version lowered below the locked one still resolves to the locked one,
# which is what runs.
#
# Resolving the configurations is not enough: the formatter Spotless fetches
# is no configuration, and the test task picks up its launcher only when it
# runs. So every task a gate runs is run once over a throwaway class and its
# test, after which the gate's --offline builds fetch nothing.
WORKDIR /warm
COPY settings.gradle.kts build.gradle.kts gradle.properties gradle.lockfile buildscript-gradle.lockfile ./
RUN <<'WARM'
set -eu
mkdir -p src/main/java/warm src/test/java/warm
cat > src/main/java/warm/Warm.java <<'JAVA'
package warm;

/** Exists only so this image build runs every task a gate runs. */
public final class Warm {
  /** A warm class. */
  public Warm() {}

  /**
   * One.
   *
   * @return one
   */
  public int one() {
    return 1;
  }
}
JAVA
cat > src/test/java/warm/WarmTest.java <<'JAVA'
package warm;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class WarmTest {
  @Test
  void runs() {
    assertEquals(1, new Warm().one());
  }
}
JAVA
gradle spotlessCheck compileJava compileTestJava javadoc test jacocoTestCoverageVerification
gradle publishToMavenLocal -Dmaven.repo.local=/tmp/warm-repo
cd /
rm -rf /warm /tmp/warm-repo /tmp/gradle-build
WARM

WORKDIR /repo/sdk/jvm
