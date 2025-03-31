plugins {
    kotlin("jvm") version "2.1.10"
}

group = "com.tunafysh"
version = "0.5"

repositories {
    mavenCentral()
}

dependencies {
    testImplementation(kotlin("test"))
    implementation("com.formdev:flatlaf:3.5.4")
}

tasks.test {
    useJUnitPlatform()
}
kotlin {
    jvmToolchain(21)
}